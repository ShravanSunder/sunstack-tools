#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
#[tokio::test]
async fn deferred_scan_performs_no_acquisition_after_nonpass_failure_or_cancellation_with_a_passing_control()
 {
    for outcome_kind in ["pass", "nonpass", "failure", "cancel", "unresolved"] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![
                fixture::decision("subject"),
                json!({"kind":"condition","test":{"kind":"pattern_present","input":"other","pattern":{"kind":"literal","text":"LATE_PASS"},"max_scan_bytes":if outcome_kind=="unresolved"{1}else{4096}}}),
            ],
        );
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.attempts.retry_count = 0;
        let cancellation = CancellationToken::new();
        let execution = tokio::spawn(evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: cancellation.clone(),
            },
        ));
        let first = server.next().await;
        assert_eq!(first.body["questions"].as_object().unwrap().len(), 2);
        assert!(
            !serde_json::to_string(&first.body["state"])
                .unwrap()
                .contains("gate content")
        );
        std::fs::write(root.path().join("other.md"), "U001: LATE_PASS\n").unwrap();
        match outcome_kind {
            "nonpass" => first.noul(0.1),
            "failure" => first.respond(503, json!({"error":"failure"})),
            "cancel" => {
                cancellation.cancel();
                drop(first);
            }
            _ => first.noul(0.9),
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } => *partial_report,
            _ => panic!("{outcome_kind}: {outcome:?}"),
        };
        assert_eq!(
            report.measurements.files_read,
            if matches!(outcome_kind, "pass" | "unresolved") {
                2
            } else {
                1
            }
        );
        assert_eq!(report.measurements.model_requests, 1);
        match outcome_kind {
            "pass" => assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            )),
            "nonpass" => assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable {
                    stopped_at: EntryKey { position: 0, .. },
                    ..
                }
            )),
            "unresolved" => assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Condition {
                        entry: EntryKey { position: 1, .. },
                        ..
                    },
                    ..
                }
            )),
            _ => assert!(!matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            )),
        }
        if matches!(outcome_kind, "nonpass" | "failure" | "cancel") {
            assert!(matches!(
                report.groups[0].entries[1].result,
                EntryResult::NotEvaluated { .. }
            ));
        }
        let _ = server.finish().await;
    }
}
#[tokio::test]
async fn a_same_input_then_condition_then_different_input_gate_preserves_the_lazy_frontier() {
    for condition in ["pass", "false", "unresolved"] {
        let root = tempfile::tempdir().unwrap();
        let literal = if condition == "false" {
            "not found"
        } else {
            "gate content"
        };
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![
                fixture::decision("subject"),
                json!({"kind":"condition","test":{"kind":"pattern_present","input":"other","pattern":{"kind":"literal","text":literal},"max_scan_bytes":if condition=="unresolved"{1}else{4096}}}),
                fixture::decision("later"),
            ],
        );
        std::fs::write(root.path().join("later.md"), "U001: C_ONLY\n").unwrap();
        let path = root.path().join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["inputs"]["later"] = json!({"kind":"document"});
        std::fs::write(path, set.to_string()).unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["later"] =
            json!({"kind":"document","repository":"repo","path":"later.md"});
        input["repositories"][0]["input_paths"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"exact","path":"later.md"}));
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let first = server.next().await;
        assert_eq!(first.body["questions"].as_object().unwrap().len(), 1);
        assert_eq!(
            first.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        first.noul(0.9);
        if condition == "pass" {
            let gate = server.next().await;
            assert_eq!(
                gate.body["questions"]["q000001"]["instructions"]["owner"],
                "rule_set_decision:2"
            );
            assert!(
                serde_json::to_string(&gate.body["state"])
                    .unwrap()
                    .contains("C_ONLY")
            );
            gate.noul(0.9);
            server.next().await.noul(0.9);
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            _ => panic!("{outcome:?}"),
        };
        assert_eq!(
            report.measurements.files_read,
            if condition == "pass" { 3 } else { 2 }
        );
        assert_eq!(
            report.measurements.model_requests,
            if condition == "pass" { 3 } else { 1 }
        );
        if condition != "pass" {
            assert!(matches!(
                report.groups[0].entries[2].result,
                EntryResult::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped
                }
            ));
            assert!(
                report
                    .requests
                    .iter()
                    .filter(|request| matches!(request.status, RequestStatus::Sent { .. }))
                    .all(|request| request.questions.iter().all(|question| !matches!(
                        question.owner,
                        QuestionOwner::RuleSetDecision {
                            entry: EntryKey { position: 2, .. }
                        }
                    )))
            );
        }
        assert_eq!(server.finish().await, report.measurements.model_requests);
    }
}
#[tokio::test]
async fn adjacent_different_input_gates_over_the_same_source_keep_separate_staged_selections() {
    for stop in ["nonpass", "failure", "pass"] {
        let root = tempfile::tempdir().unwrap();
        let first = fixture::decision("other");
        let mut second = fixture::decision("other");
        second["evidence"][0]["select"] = json!({"kind":"whole_document"});
        fixture::write_set(root.path(), &[fixture::rule("one")], vec![first, second]);
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.attempts.retry_count = 0;
        let execution = tokio::spawn(evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let gate = server.next().await;
        assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
        if stop == "failure" {
            gate.respond(503, json!({"error":"controlled failure"}));
        } else {
            gate.noul(if stop == "pass" { 0.9 } else { 0.1 });
        }
        if stop == "pass" {
            let gate = server.next().await;
            assert_eq!(
                gate.body["questions"]["q000001"]["instructions"]["owner"],
                "rule_set_decision:1"
            );
            gate.noul(0.9);
            server.next().await.noul(0.9);
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report } if stop != "failure" => report,
            RuleInvocationOutcome::IncompleteReport { report } if stop == "failure" => report,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            report.measurements.files_read,
            if stop == "pass" { 2 } else { 1 }
        );
        assert_eq!(
            report.measurements.files_parsed,
            if stop == "pass" { 3 } else { 1 }
        );
        assert_eq!(server.finish().await, if stop == "pass" { 3 } else { 1 });
    }
}

#[tokio::test]
async fn adjacent_different_input_gates_speculate_concurrently_with_one_control_each() {
    for passes in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut second = fixture::decision("other");
        second["evidence"][0]["select"] = json!({"kind":"whole_document"});
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![fixture::decision("other"), second],
        );
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.execution.speculation = Speculation::On;
        let execution = tokio::spawn(evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let mut held = Vec::new();
        for _ in 0..3 {
            held.push(server.next().await);
        }
        let owners = held
            .iter()
            .map(|request| {
                request.body["questions"]["q000001"]["instructions"]["owner"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            owners,
            std::collections::BTreeSet::from([
                "rule_set_decision:0".into(),
                "rule_set_decision:1".into(),
                "rule:one".into()
            ])
        );
        for request in held.into_iter().rev() {
            let first = request.body["questions"]["q000001"]["instructions"]["owner"]
                == "rule_set_decision:0";
            request.noul(if first && !passes { 0.1 } else { 0.9 });
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.files_read, 2);
        assert_eq!(report.measurements.model_requests, 3);
        if passes {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            ));
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable {
                    stopped_at: EntryKey { position: 0, .. },
                    ..
                }
            ));
            assert!(matches!(
                report.groups[0].entries[1].result,
                EntryResult::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped
                }
            ));
            assert_eq!(
                report
                    .requests
                    .iter()
                    .flat_map(|request| &request.discarded)
                    .count(),
                2
            );
        }
        assert_eq!(server.finish().await, 3);
    }
}
