#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn rules_split_greedily_at_rule_boundaries_and_reassess_real_bodies() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[
            fixture::rule("one"),
            fixture::rule("two"),
            fixture::rule("three"),
        ],
        vec![fixture::decision("subject")],
    );
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    input.request_budget.max_questions = 3;
    let backend = transport::backend(&server.endpoint);
    let execution = evaluate_rules(
        input,
        RuleExecutionContext {
            backend: Arc::new(backend),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    );
    tokio::pin!(execution);
    let mut bodies = Vec::new();
    let outcome=tokio::time::timeout(Duration::from_secs(5),async {loop {tokio::select!{outcome=&mut execution=>break outcome, request=server.next()=>{bodies.push(request.body.clone());request.noul(0.9);}}}}).await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.requests.len(), 2);
    assert_eq!(report.measurements.model_requests, 2);
    assert_eq!(report.occurrences.len(), 3);
    assert!(
        report
            .occurrences
            .iter()
            .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
    );
    assert!(matches!(
        report.requests[0].mode,
        RequestMode::Split { part: 0, of: 2 }
    ));
    assert!(matches!(
        report.requests[1].mode,
        RequestMode::Split { part: 1, of: 2 }
    ));
    let first_body = bodies
        .iter()
        .find(|body| body["questions"].as_object().unwrap().len() == 3)
        .unwrap();
    let second_body = bodies
        .iter()
        .find(|body| body["questions"].as_object().unwrap().len() == 1)
        .unwrap();
    assert_eq!(first_body["questions"].as_object().unwrap().len(), 3);
    assert_eq!(second_body["questions"].as_object().unwrap().len(), 1);
    assert_eq!(
        first_body["questions"]["q000001"]["instructions"]["owner"],
        "rule_set_decision:0"
    );
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn carried_gate_that_cannot_share_split_zero_is_sent_concurrently_once() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[fixture::rule("one")],
        vec![fixture::decision("subject")],
    );
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    input.request_budget.max_questions = 1;
    let execution = tokio::spawn(evaluate_rules(
        input,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    // Neither response is released until both real HTTP bodies arrive: staging either request deadlocks this bound.
    let first = server.next().await;
    let second = server.next().await;
    let owners = [
        first.body["questions"]["q000001"]["instructions"]["owner"]
            .as_str()
            .unwrap()
            .to_owned(),
        second.body["questions"]["q000001"]["instructions"]["owner"]
            .as_str()
            .unwrap()
            .to_owned(),
    ];
    assert_eq!(
        owners
            .iter()
            .filter(|owner| owner.as_str() == "rule_set_decision:0")
            .count(),
        1
    );
    second.noul(0.9);
    first.noul(0.9);
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.requests.len(), 2);
    assert_eq!(report.measurements.model_requests, 2);
    assert!(matches!(
        report.occurrences[0],
        RuleOccurrenceResult::Judged { .. }
    ));
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn oversized_rule_is_refused_without_preventing_a_fitting_peer_and_request_limit_counts_splits()
 {
    for limited in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut first = fixture::rule("one");
        if !limited {
            first["questions"] =
                serde_json::json!({"extra":{"type":"noul","instructions":"extra"}});
        }
        fixture::write_set(root.path(), &[first, fixture::rule("two")], vec![]);
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.request_budget.max_questions = 1;
        if limited {
            input.request_budget.max_requests = 1;
        }
        let execution = evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>request.noul(0.9)}}}).await.unwrap();
        let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.model_requests, 1);
        if limited {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            ));
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::NotStarted {
                    reason: NotStartedReason::RequestLimit,
                    ..
                }
            ));
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Budget { .. },
                    ..
                }
            ));
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::Judged { .. }
            ));
            assert!(
                serde_json::to_string(&report.diagnostics)
                    .unwrap()
                    .contains("rule_exceeds_budget")
            );
        }
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn optional_content_is_removed_once_before_splitting_but_required_content_is_never_trimmed() {
    for optional in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let mut rule = fixture::rule("one");
        let mut extra = fixture::declaration("other", "extra");
        extra["requirement"] = serde_json::json!(if optional { "optional" } else { "required" });
        rule["evidence"].as_array_mut().unwrap().push(extra);
        fixture::write_set(root.path(), &[rule], vec![]);
        std::fs::write(
            root.path().join("other.md"),
            format!("U001: {}\n", "OPTIONAL_OVERSIZE_\"\\\n☃".repeat(400)),
        )
        .unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.request_budget.max_request_bytes = 3000;
        let execution = evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let mut bodies = Vec::new();
        let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>{bodies.push(request.body.clone());request.noul(0.9);}}}}).await.unwrap();
        if optional {
            let RuleInvocationOutcome::CompleteReport { report } = outcome else {
                panic!("{outcome:?}")
            };
            assert_eq!(report.measurements.model_requests, 1);
            assert_eq!(bodies.len(), 1);
            let serialized = serde_json::to_vec(&bodies[0]).unwrap();
            assert!(serialized.len() <= 3000);
            assert!(!String::from_utf8_lossy(&serialized).contains("OPTIONAL_OVERSIZE"));
            let extra = &bodies[0]["state"]["evidence"]["rule:one/extra"];
            assert!(serde_json::to_string(extra).unwrap().contains("limit"));
        } else {
            let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
                panic!("{outcome:?}")
            };
            assert_eq!(report.measurements.model_requests, 0);
            assert!(bodies.is_empty());
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Budget { .. },
                    ..
                }
            ));
        }
        assert_eq!(server.finish().await, usize::from(optional));
    }
}
