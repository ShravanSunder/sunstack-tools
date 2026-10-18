#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

#[derive(Clone, Copy)]
enum CarrierScenario {
    DifferentInputStaged,
    SameInputSpeculative,
    SeparateRouter,
}

fn write_mixed_rules(root: &std::path::Path, scenario: CarrierScenario) {
    let mut local = fixture::rule("local");
    local["evidence"][0]["select"] = json!({"kind":"frontmatter_field","field":"description"});
    local["verdict"] = json!({"kind":"mechanical","explanation":"Require a description","check":{"kind":"expect","expect":{"present":true}}});
    let mut expanded = fixture::rule("expanded");
    expanded["evidence"][0]["select"] =
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    let mut gate = fixture::decision(
        if matches!(scenario, CarrierScenario::SameInputSpeculative) {
            "subject"
        } else {
            "other"
        },
    );
    // The same-input gate reads the whole original; legacy U001 IDs are absent.
    gate["evidence"][0]["select"] = json!({"kind":"whole_document"});
    expanded["applicability"] = json!([gate]);
    fixture::write_set(root, &[local, expanded], vec![]);
    std::fs::write(
        root.join("source.md"),
        "---\ndescription: Ready\n---\n\nFirst target.\n\nSecond target.\n",
    )
    .unwrap();
    if matches!(scenario, CarrierScenario::SeparateRouter) {
        std::fs::write(root.join("router.json"), serde_json::to_vec(&json!({
            "schema_version":1,"name":"routing","description":"separate routing settlement",
            "inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},
            "evidence":[fixture::declaration("other","router_context")],
            "question":{"type":"choice","instructions":"select route","criteria":{"alpha":"run checks","none":"unrouted"}},
            "routes":{"alpha":[{"rule_set":"set.json","inputs":{"subject":"subject","other":"other"}}]},"confidence_floor":0.6
        })).unwrap()).unwrap();
    }
}

fn answer_pass(request: controlled::PendingHttpRequest) {
    let answers = request.body["questions"].as_object().unwrap().iter().map(|(name, question)| {
        let value = if question["type"] == "choice" {
            json!({"type":"choice","choice":"alpha","confidence":0.95,"probabilities":{"alpha":0.95,"none":0.05}})
        } else { json!({"type":"noul","noul":0.95}) };
        (name.clone(), value)
    }).collect::<serde_json::Map<_,_>>();
    request.respond(200, json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5}}));
}

async fn observe_mixed_carrier(scenario: CarrierScenario) {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    write_mixed_rules(root.path(), scenario);
    let mut wire = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    wire["execution"]["max_concurrent_requests"] = json!(1);
    wire["execution"]["speculation"] = json!(if matches!(
        scenario,
        CarrierScenario::SameInputSpeculative
    ) {
        "on"
    } else {
        "off"
    });
    wire["attempts"]["retry_count"] = json!(0);
    if matches!(scenario, CarrierScenario::SeparateRouter) {
        let group = wire["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        wire["definitions"] =
            json!({"kind":"router","path":"router.json","binding_groups":[group]});
    }
    let mut execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(wire).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    let mut bodies: Vec<Value> = Vec::new();
    let outcome = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            tokio::select! {
                result = &mut execution => break result.unwrap(),
                request = server.next() => {
                    bodies.push(request.body.clone());
                    answer_pass(request);
                }
            }
        }
    })
    .await
    .unwrap();
    let sent = server.finish().await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let expected_counts = match scenario {
        CarrierScenario::DifferentInputStaged => vec![1, 2],
        CarrierScenario::SameInputSpeculative => vec![3],
        CarrierScenario::SeparateRouter => vec![1, 1, 2],
    };
    let question_counts = bodies
        .iter()
        .map(|body| body["questions"].as_object().unwrap().len())
        .collect::<Vec<_>>();
    assert_eq!(question_counts, expected_counts);
    assert_eq!(sent, expected_counts.len());
    assert_eq!(report.measurements.model_requests, sent);
    let owners = bodies
        .iter()
        .flat_map(|body| body["questions"].as_object().unwrap().values())
        .map(|question| question["instructions"]["owner"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        owners
            .iter()
            .filter(|owner| owner.starts_with("rule:expanded:decision:"))
            .count(),
        1
    );
    assert_eq!(
        owners
            .iter()
            .filter(|owner| owner.starts_with("rule:expanded:") && !owner.contains(":decision:"))
            .count(),
        2
    );
    assert!(owners.iter().all(|owner| !owner.starts_with("rule:local")));
    if matches!(scenario, CarrierScenario::SeparateRouter) {
        assert_eq!(owners[0], "router");
        assert_eq!(
            serde_json::to_value(&report.routing).unwrap()["groups"][0]["outcome"]["kind"],
            "routed"
        );
    }
    assert!(report.is_complete());
    let local = report
        .occurrences
        .iter()
        .find(|result| result.key().rule_id.as_str() == "local")
        .unwrap();
    assert!(matches!(
        local,
        RuleOccurrenceResult::Checked {
            outcome: LocalSettlement::Completed {
                result: MechanicalResult::Pass,
                ..
            },
            ..
        }
    ));
    let children = report
        .occurrences
        .iter()
        .filter(|result| result.key().rule_id.as_str() == "expanded")
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 2);
    assert!(
        children
            .iter()
            .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
    );
    assert!(report.requests.iter().all(|request| !matches!(
        request.status,
        RequestStatus::NotSent {
            reason: NotSentReason::EvidenceRefused { .. }
        }
    )));
}

#[tokio::test]
async fn a_local_result_keeps_its_carrier_for_a_different_input_gated_unit_expansion() {
    observe_mixed_carrier(CarrierScenario::DifferentInputStaged).await;
}

#[tokio::test]
async fn a_local_result_keeps_same_input_speculative_gate_and_unit_children() {
    observe_mixed_carrier(CarrierScenario::SameInputSpeculative).await;
}

#[tokio::test]
async fn a_local_result_keeps_unit_children_after_separate_router_settlement() {
    observe_mixed_carrier(CarrierScenario::SeparateRouter).await;
}
