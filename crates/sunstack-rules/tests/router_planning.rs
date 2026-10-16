#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
use sunstack_decisions::{CancellationToken, DecisionRunOutcome};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn routed_input(root: &Path, endpoint: &str) -> Value {
    let mut input = serde_json::to_value(fixture::input(root, endpoint)).unwrap();
    let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
    input["definitions"] = json!({"kind":"router","path":"router.json","binding_groups":[group]});
    input["attempts"]["retry_count"] = json!(0);
    input
}
fn write_router(root: &Path, different_source: bool) {
    let second_source = if different_source { "other" } else { "subject" };
    write_json(
        &root.join("router.json"),
        &json!({"schema_version":1,"name":"planning","description":"carrier selection","inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},"evidence":[fixture::declaration("subject","router_context")],"question":{"type":"choice","instructions":"select","criteria":{"alpha":"route","none":"unrouted"}},"routes":{"alpha":[{"rule_set":"set.json","inputs":{"subject":"subject","other":"other"}},{"rule_set":"second.json","inputs":{"subject":second_source,"other":"other"}}]},"confidence_floor":0.6}),
    );
}
fn answer(request: controlled::PendingHttpRequest) {
    let answers = request.body["questions"].as_object().unwrap().iter().map(|(name, question)| {
        let value = if question["type"] == "choice" {json!({"type":"choice","choice":"alpha","confidence":0.9,"probabilities":{"alpha":0.9,"none":0.1}})} else {json!({"type":"noul","noul":0.9})};
        (name.clone(), value)
    }).collect::<serde_json::Map<_,_>>();
    request.respond(200, json!({"model":"typesafe/planning","answers":answers,"usage":{"input_tokens":20,"output_tokens":4,"cost":0.001}}));
}
fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
#[tokio::test]
async fn leading_pruning_moves_router_control_to_the_next_carrier_or_a_router_only_request() {
    for all_pruned in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let stop = json!({"kind":"condition","test":{"kind":"origin_field_equals","field":"host","value":"different-host"}});
        fixture::write_set(root.path(), &[fixture::rule("one")], vec![stop.clone()]);
        let mut second: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
        if !all_pruned {
            second["applicability"] = json!([]);
        }
        write_json(&root.path().join("second.json"), &second);
        write_router(root.path(), false);
        let mut server = controlled::ControlledServer::start().await;
        let mut input = routed_input(root.path(), &server.endpoint);
        input["origin"] = json!({"kind":"hook","host":"actual-host","event":"proof"});
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            context(&server.endpoint),
        ));
        let request = server.next().await;
        assert_eq!(
            request.body["questions"].as_object().unwrap().len(),
            if all_pruned { 1 } else { 2 }
        );
        answer(request);
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.model_requests, 1);
        assert_eq!(report.occurrences.len(), 2);
        assert!(matches!(
            report.occurrences[0],
            RuleOccurrenceResult::NotApplicable { .. }
        ));
        assert_eq!(
            matches!(
                report.occurrences[1],
                RuleOccurrenceResult::NotApplicable { .. }
            ),
            all_pruned
        );
        if !all_pruned {
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::Judged { .. }
            ));
            let wire = serde_json::to_value(&report.requests[0].owner).unwrap();
            assert_eq!(
                wire["rule_set"],
                json!(root.path().join("second.json").canonicalize().unwrap())
            );
        }
        let RoutingReport::Router { groups } = &report.routing else {
            panic!("{report:?}")
        };
        let RouterOutcome::Routed { answer, .. } = &groups[0].outcome else {
            panic!("{groups:?}")
        };
        assert!(matches!(
            report.answer(answer),
            Some(sunstack_decisions::JudgmentAnswer::Choice { .. })
        ));
        assert_eq!(server.finish().await, 1);
    }
}
#[tokio::test]
async fn noncarrying_candidates_dispatch_before_routing_only_when_speculation_is_enabled() {
    for speculate in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
        std::fs::copy(
            root.path().join("set.json"),
            root.path().join("second.json"),
        )
        .unwrap();
        write_router(root.path(), true);
        let mut server = controlled::ControlledServer::start().await;
        let mut input = routed_input(root.path(), &server.endpoint);
        input["execution"]["speculation"] = json!(if speculate { "on" } else { "off" });
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            context(&server.endpoint),
        ));
        let first = server.next().await;
        if speculate {
            // Both real bodies arrive while the controlling choice is still held.
            let second = server.next().await;
            answer(second);
            answer(first);
        } else {
            answer(first);
            answer(server.next().await);
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.model_requests, 2);
        assert!(
            report
                .occurrences
                .iter()
                .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
        );
        let router_questions = report
            .requests
            .iter()
            .flat_map(|request| &request.questions)
            .filter(|binding| matches!(binding.owner, QuestionOwner::Router { .. }))
            .count();
        assert_eq!(router_questions, 1);
        assert_eq!(server.finish().await, 2);
    }
}
#[tokio::test]
async fn a_failed_router_is_incomplete_and_never_turns_candidate_answers_into_judgments() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    std::fs::copy(
        root.path().join("set.json"),
        root.path().join("second.json"),
    )
    .unwrap();
    write_router(root.path(), true);
    let mut server = controlled::ControlledServer::start().await;
    let input = routed_input(root.path(), &server.endpoint);
    let execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(input).unwrap(),
        context(&server.endpoint),
    ));
    server
        .next()
        .await
        .respond(503, json!({"error":"provider detail must stay private"}));
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let wire = serde_json::to_value(&report).unwrap();
    assert_eq!(
        wire["routing"]["groups"][0]["outcome"]["kind"],
        "router_failed"
    );
    assert!(
        report
            .occurrences
            .iter()
            .all(|result| matches!(result, RuleOccurrenceResult::EvaluationFailed { .. }))
    );
    assert!(matches!(
        report.requests[0].status,
        RequestStatus::Sent {
            outcome: DecisionRunOutcome::ProviderFailure { .. }
        }
    ));
    assert!(!wire.to_string().contains("provider detail"));
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn two_candidates_with_a_split_resolve_exactly_one_router_and_one_decision_per_group() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[fixture::rule("one"), fixture::rule("two")],
        vec![fixture::decision("subject")],
    );
    let mut second: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    second["rule_paths"] = json!(["second-rules.jsonl"]);
    write_json(&root.path().join("second.json"), &second);
    write_json(
        &root.path().join("second-rules.jsonl"),
        &fixture::rule("peer"),
    );
    write_router(root.path(), false);
    let mut server = controlled::ControlledServer::start().await;
    let mut input = routed_input(root.path(), &server.endpoint);
    input["request_budget"]["max_questions"] = json!(3);
    let execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(input).unwrap(),
        context(&server.endpoint),
    ));
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(server.next().await);
    }
    assert!(
        held.iter()
            .all(|request| request.body["questions"].as_object().unwrap().len() <= 3)
    );
    assert_eq!(
        held.iter()
            .flat_map(|request| request.body["questions"].as_object().unwrap().values())
            .filter(|question| question["type"] == "choice")
            .count(),
        1
    );
    assert_eq!(
        held.iter()
            .flat_map(|request| request.body["questions"].as_object().unwrap().values())
            .filter(|question| question["instructions"]["owner"] == "rule_set_decision:0")
            .count(),
        2
    );
    for request in held.into_iter().rev() {
        answer(request);
    }
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.occurrences.len(), 3);
    assert_eq!(report.measurements.model_requests, 3);
    assert_eq!(report.requests.len(), 3);
    let mut decision_refs = Vec::new();
    for group in &report.groups {
        let EntryResult::Decision {
            outcome: DecisionEntryOutcome::Passed { answer },
            ..
        } = &group.entries[0].result
        else {
            panic!("{group:?}")
        };
        assert!(matches!(
            report.answer(answer),
            Some(sunstack_decisions::JudgmentAnswer::Noul { .. })
        ));
        decision_refs.push(answer);
    }
    assert_eq!(decision_refs.len(), 2);
    assert_ne!(decision_refs[0], decision_refs[1]);
    for result in &report.occurrences {
        let RuleOccurrenceResult::Judged { judgment, .. } = result else {
            panic!("{result:?}")
        };
        let VerdictBasis::Noul { answer } = &judgment.verdict.basis else {
            panic!("{judgment:?}")
        };
        assert!(report.answer(answer).is_some());
    }
    let total_cost = report
        .requests
        .iter()
        .map(|request| match &request.status {
            RequestStatus::Sent {
                outcome:
                    DecisionRunOutcome::Completed {
                        receipt, attempts, ..
                    },
            } => {
                assert_eq!(attempts.len(), 1);
                receipt.usage.cost_usd.unwrap()
            }
            other => panic!("{other:?}"),
        })
        .sum::<f64>();
    assert!((total_cost - 0.003).abs() < 1e-12);
    assert_eq!(
        report
            .requests
            .iter()
            .flat_map(|request| &request.questions)
            .filter(|binding| matches!(binding.owner, QuestionOwner::Router { .. }))
            .count(),
        1
    );
    assert_eq!(server.finish().await, 3);
}

#[tokio::test]
async fn declared_two_ruleset_workload_measures_router_requests_against_per_ruleset_decisions() {
    for routed in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            if routed {
                vec![]
            } else {
                vec![fixture::decision("other")]
            },
        );
        std::fs::copy(
            root.path().join("set.json"),
            root.path().join("second.json"),
        )
        .unwrap();
        write_router(root.path(), false);
        let router_path = root.path().join("router.json");
        let mut router: Value =
            serde_json::from_slice(&std::fs::read(&router_path).unwrap()).unwrap();
        router["evidence"][0]["input"] = json!("other");
        write_json(&router_path, &router);
        let mut server = controlled::ControlledServer::start().await;
        let mut input = routed_input(root.path(), &server.endpoint);
        if !routed {
            let base = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
            let group = base["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
            input["definitions"] = json!({"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[group.clone()]},{"path":"second.json","binding_groups":[group]}]});
        }
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            context(&server.endpoint),
        );
        tokio::pin!(execution);
        let outcome = tokio::time::timeout(Duration::from_secs(3), async {loop {tokio::select! { result = &mut execution => break result, request = server.next() => answer(request)}}}).await.unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        let expected = if routed { 3 } else { 4 };
        assert_eq!(report.measurements.model_requests, expected);
        assert_eq!(report.measurements.files_read, 2);
        assert_eq!(report.occurrences.len(), 2);
        assert!(report.occurrences.iter().all(|result| matches!(result, RuleOccurrenceResult::Judged { judgment, .. } if matches!(judgment.verdict.result, VerdictResult::Pass))));
        assert_eq!(server.finish().await, expected);
    }
}
