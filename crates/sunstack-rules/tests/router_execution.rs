#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
fn router(root: &std::path::Path) {
    std::fs::write(root.join("router.json"),serde_json::to_vec(&json!({"schema_version":1,"name":"router","description":"group routing","inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},"evidence":[fixture::declaration("subject","router_context")],"question":{"type":"choice","instructions":"route","criteria":{"alpha":"a","beta":"b","none":"unrouted"}},"routes":{"alpha":[{"rule_set":"set.json","inputs":{"subject":"subject","other":"other"}}],"beta":[{"rule_set":"set.json","inputs":{"subject":"subject","other":"other"}}]},"confidence_floor":0.6})).unwrap()).unwrap();
}
fn answer(request: controlled::PendingHttpRequest, label: &str, confidence: f64) {
    let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)|{let answer=if question["type"]=="choice"{json!({"type":"choice","choice":label,"confidence":confidence,"probabilities":{"alpha":0.6,"beta":0.3,"none":0.1}})}else{json!({"type":"noul","noul":0.9})};(name.clone(),answer)}).collect::<serde_json::Map<_,_>>();
    request.respond(200,json!({"model":"typesafe/router","answers":answers,"usage":{"input_tokens":30,"output_tokens":5,"cost":0.002}}));
}
#[tokio::test]
async fn router_groups_have_independent_controlling_answers_and_authored_reports_under_reverse_completion()
 {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    router(root.path());
    std::fs::write(root.path().join("second.md"), "U001: second group\n").unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut input = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    let first = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
    let mut second = first.clone();
    second["bindings"]["subject"]["path"] = json!("second.md");
    input["definitions"] =
        json!({"kind":"router","path":"router.json","binding_groups":[first,second]});
    input["repositories"][0]["input_paths"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"exact","path":"second.md"}));
    let execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(input).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    let mut requests = Vec::new();
    for _ in 0..4 {
        requests.push(server.next().await);
    }
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.body["questions"]
                .as_object()
                .unwrap()
                .values()
                .any(|question| question["type"] == "choice"))
            .count(),
        2
    );
    for request in requests.into_iter().rev() {
        let group_two = serde_json::to_string(&request.body["state"])
            .unwrap()
            .contains("second group");
        answer(request, if group_two { "beta" } else { "alpha" }, 0.6);
    }
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let RoutingReport::Router { groups } = &report.routing else {
        panic!("{report:?}")
    };
    assert_eq!(groups.len(), 2);
    for (position, label) in ["alpha", "beta"].into_iter().enumerate() {
        assert_eq!(groups[position].position, position);
        let RouterOutcome::Routed {
            label: selected,
            answer,
            ..
        } = &groups[position].outcome
        else {
            panic!("{groups:?}")
        };
        assert_eq!(selected.as_str(), label);
        assert!(matches!(
            report.answer(answer),
            Some(sunstack_decisions::JudgmentAnswer::Choice { .. })
        ));
    }
    assert_eq!(report.occurrences.len(), 2);
    assert_eq!(report.occurrences[0].key().group, 0);
    assert_eq!(report.occurrences[1].key().group, 1);
    assert_eq!(report.measurements.model_requests, 4);
    assert_eq!(report.measurements.files_read, 2);
    assert_eq!(
        report
            .requests
            .iter()
            .flat_map(|request| &request.discarded)
            .count(),
        2
    );
    assert_eq!(server.finish().await, 4);
}
#[tokio::test]
async fn native_choice_confidence_controls_uncertain_routed_and_no_route_outcomes() {
    for (confidence, label, expected) in [
        (0.59, "alpha", "uncertain_intent"),
        (0.60, "alpha", "routed"),
        (0.4, "alpha", "uncertain_intent"),
        (0.9, "none", "no_route"),
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
        router(root.path());
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"] =
            json!({"kind":"router","path":"router.json","binding_groups":[group]});
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>answer(request,label,confidence)}}}).await.unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        let value = serde_json::to_value(&report.routing).unwrap();
        assert_eq!(value["groups"][0]["outcome"]["kind"], expected);
        assert_eq!(report.occurrences.len(), usize::from(expected == "routed"));
        if expected == "uncertain_intent" {
            assert_eq!(
                value["groups"][0]["outcome"]["confidence"],
                json!(confidence)
            );
            assert_eq!(
                value["groups"][0]["outcome"]["distribution"]["alpha"],
                json!(0.6)
            );
        }
        assert_eq!(server.finish().await, report.measurements.model_requests);
    }
}

#[tokio::test]
async fn same_ruleset_under_two_route_labels_keeps_opposite_wired_gates_separate() {
    for selected in ["alpha", "beta"] {
        let root = tempfile::tempdir().unwrap();
        let mut rule = fixture::rule("one");
        rule["evidence"]
            .as_array_mut()
            .unwrap()
            .push(fixture::declaration("other", "context"));
        fixture::write_set(root.path(), &[rule], vec![fixture::decision("subject")]);
        router(root.path());
        let path = root.path().join("router.json");
        let mut definition: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        definition["routes"]["beta"][0]["inputs"] = json!({"subject":"other","other":"subject"});
        std::fs::write(&path, serde_json::to_vec(&definition).unwrap()).unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"] =
            json!({"kind":"router","path":"router.json","binding_groups":[group]});
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let first = server.next().await;
        let second = server.next().await;
        let (alpha, beta) = if first.body["questions"]
            .as_object()
            .unwrap()
            .values()
            .any(|question| question["type"] == "choice")
        {
            (first, second)
        } else {
            (second, first)
        };
        beta.noul(0.9);
        let answers=alpha.body["questions"].as_object().unwrap().iter().map(|(name,question)|{let answer=if question["type"]=="choice"{json!({"type":"choice","choice":selected,"confidence":0.9,"probabilities":{"alpha":0.5,"beta":0.5,"none":0.0}})}else{json!({"type":"noul","noul":if question["instructions"]["owner"].as_str().unwrap().starts_with("rule_set_decision:"){0.1}else{0.9}})};(name.clone(),answer)}).collect::<serde_json::Map<_,_>>();
        alpha.respond(200,json!({"model":"typesafe/test","answers":answers,"usage":{"input_tokens":10,"output_tokens":3}}));
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.occurrences.len(), 1);
        let wire = serde_json::to_value(&report).unwrap();
        assert_eq!(wire["groups"][0]["binding"]["label"], selected);
        assert_eq!(
            wire["groups"][0]["entries"][0]["key"]["owner"]["route"],
            selected
        );
        let routes = wire["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|request| request["owner"]["route"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(routes, vec!["alpha", "beta"]);
        if selected == "alpha" {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable { .. }
            ));
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            ));
        }
        assert_eq!(server.finish().await, 2);
    }
}
