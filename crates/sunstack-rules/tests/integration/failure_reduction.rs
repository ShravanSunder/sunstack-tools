#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/observed_backend.rs"]
mod observed;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::{CancellationToken, DecisionRunOutcome};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn settled_gate_controls_failed_speculative_rule_regardless_of_real_adapter_completion_order()
{
    for gate_probability in [0.1, 0.9] {
        for failure_first in [false, true] {
            let root = tempfile::tempdir().unwrap();
            fixture::write_set(
                root.path(),
                &[fixture::rule("one")],
                vec![fixture::decision("other")],
            );
            let mut server = controlled::ControlledServer::start().await;
            let mut input = fixture::input(root.path(), &server.endpoint);
            input.execution.speculation = Speculation::On;
            input.attempts.retry_count = 0;
            let (send, mut events) = tokio::sync::mpsc::unbounded_channel();
            let backend = observed::ObservedBackend {
                inner: transport::backend(&server.endpoint),
                events: send,
            };
            let execution = tokio::spawn(evaluate_rules(
                input,
                RuleExecutionContext {
                    backend: Arc::new(backend),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: CancellationToken::new(),
                },
            ));
            let first = server.next().await;
            let second = server.next().await;
            let (gate, rule) = if first.body["questions"]["q000001"]["instructions"]["owner"]
                == "rule_set_decision:0"
            {
                (first, second)
            } else {
                (second, first)
            };
            if failure_first {
                rule.respond(503, json!({"error":"do not copy provider detail"}));
                assert!(matches!(
                    observed::next_event(&mut events).await,
                    observed::AdapterEvent::AttemptFailed
                ));
                gate.noul(gate_probability);
            } else {
                gate.noul(gate_probability);
                assert!(matches!(
                    observed::next_event(&mut events).await,
                    observed::AdapterEvent::ResponseAdmitted
                ));
                rule.respond(503, json!({"error":"do not copy provider detail"}));
            }
            let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
                .await
                .unwrap()
                .unwrap();
            let report = match outcome {
                RuleInvocationOutcome::CompleteReport { report } if gate_probability < 0.6 => {
                    report
                }
                RuleInvocationOutcome::IncompleteReport { report } if gate_probability >= 0.6 => {
                    report
                }
                _ => panic!("{outcome:?}"),
            };
            assert_eq!(report.measurements.model_requests, 2);
            assert_eq!(report.measurements.inference_attempts, 2);
            if gate_probability < 0.6 {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::NotApplicable { .. }
                ));
            } else {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::EvaluationFailed {
                        cause: Cause::Request { .. },
                        ..
                    }
                ));
            }
            for request in &report.requests {
                if let RequestStatus::Sent {
                    outcome: DecisionRunOutcome::ProviderFailure { .. },
                } = &request.status
                {
                    assert!(
                        request.discarded.is_empty(),
                        "a failed request returned no admitted answers to discard"
                    );
                }
                if let RequestStatus::Sent {
                    outcome: DecisionRunOutcome::Completed { receipt, .. },
                } = &request.status
                {
                    assert_eq!(receipt.usage.cost_usd, Some(0.001));
                }
            }
            let failures = report
                .requests
                .iter()
                .filter_map(|request| match &request.status {
                    RequestStatus::Sent {
                        outcome: DecisionRunOutcome::ProviderFailure { attempts, .. },
                    } => Some(attempts),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].len(), 1);
            assert_eq!(
                report
                    .requests
                    .iter()
                    .filter(|request| matches!(
                        request.status,
                        RequestStatus::Sent {
                            outcome: DecisionRunOutcome::Completed { .. }
                        }
                    ))
                    .count(),
                1
            );
            assert_eq!(server.finish().await, 2);
        }
    }
}
#[tokio::test]
async fn malformed_informational_answer_fails_the_combined_request_without_salvaging_rule_verdicts()
{
    for malformed in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut first = fixture::rule("one");
        first["questions"] = json!({"detail":{"type":"noul","instructions":"informational"}});
        fixture::write_set(root.path(), &[first, fixture::rule("two")], vec![]);
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
        let request = server.next().await;
        assert_eq!(request.body["questions"].as_object().unwrap().len(), 3);
        request.respond(200,json!({"model":"typesafe/test","answers":{"q000001":{"type":"noul","noul":0.9},"q000002":{"type":"noul","noul":if malformed{json!("bad")}else{json!(0.9)}},"q000003":{"type":"noul","noul":0.9}},"usage":{"input_tokens":20,"output_tokens":4}}));
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::IncompleteReport { report } if malformed => report,
            RuleInvocationOutcome::CompleteReport { report } if !malformed => report,
            _ => panic!("{outcome:?}"),
        };
        assert_eq!(report.requests.len(), 1);
        assert_eq!(report.occurrences.len(), 2);
        if malformed {
            assert!(report.occurrences.iter().all(|result| matches!(
                result,
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Request { request_index: 0 },
                    ..
                }
            )));
            assert!(matches!(
                report.requests[0].status,
                RequestStatus::Sent {
                    outcome: DecisionRunOutcome::InvalidResponse { .. }
                }
            ));
            assert!(
                !serde_json::to_string(&report.occurrences)
                    .unwrap()
                    .contains("verdict")
            );
        } else {
            assert!(
                report
                    .occurrences
                    .iter()
                    .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
            );
        }
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn controlled_server_counts_received_bodies_even_when_a_response_is_held_at_shutdown() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
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
    let held = server.next().await;
    let count = server.finish().await;
    drop(held);
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        outcome,
        RuleInvocationOutcome::IncompleteReport { .. }
    ));
    assert_eq!(
        count, 1,
        "the observer counts received bodies, not completed replies"
    );
}

#[tokio::test]
async fn caller_cancellation_retains_a_completed_peer_and_never_dispatches_the_queued_group() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    for group in 0..3 {
        std::fs::write(
            root.path().join(format!("group{group}.md")),
            format!("U001: GROUP_{group}\n"),
        )
        .unwrap();
    }
    let mut server = controlled::ControlledServer::start().await;
    let mut input = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    let base = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
    input["definitions"]["rule_sets"][0]["binding_groups"] = json!(
        (0..3)
            .map(|group| {
                let mut binding = base.clone();
                binding["bindings"]["subject"]["path"] = json!(format!("group{group}.md"));
                binding
            })
            .collect::<Vec<_>>()
    );
    input["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"*.md"}]);
    input["execution"]["max_concurrent_requests"] = json!(1);
    input["attempts"]["retry_count"] = json!(0);
    let cancellation = CancellationToken::new();
    let execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(input).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: cancellation.clone(),
        },
    ));
    let first = server.next().await;
    assert!(
        serde_json::to_string(&first.body["state"])
            .unwrap()
            .contains("GROUP_0")
    );
    first.noul(0.9);
    // A single permit makes receipt of this body a causal barrier: the first task
    // settled before the next group acquired evidence and dispatched.
    let held = server.next().await;
    assert!(
        serde_json::to_string(&held.body["state"])
            .unwrap()
            .contains("GROUP_1")
    );
    cancellation.cancel();
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::Cancelled {
        progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
        ..
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    let report = *partial_report;
    assert_eq!(report.measurements.files_read, 2);
    assert_eq!(report.measurements.model_requests, 2);
    let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[0] else {
        panic!("{report:?}")
    };
    let VerdictBasis::Noul { answer } = &judgment.verdict.basis else {
        panic!("{judgment:?}")
    };
    let RequestStatus::Sent {
        outcome:
            DecisionRunOutcome::Completed {
                answers,
                receipt,
                attempts,
            },
    } = &report.requests[answer.request_index].status
    else {
        panic!("{report:?}")
    };
    assert!(answers.contains_key(&answer.question_name));
    assert_eq!(receipt.usage.cost_usd, Some(0.001));
    assert_eq!(attempts.len(), 1);
    assert!(matches!(
        report.occurrences[1],
        RuleOccurrenceResult::Cancelled { .. }
    ));
    assert!(matches!(
        report.occurrences[2],
        RuleOccurrenceResult::NotStarted {
            reason: NotStartedReason::CallerCancelled,
            ..
        }
    ));
    assert!(matches!(
        report.requests[1].status,
        RequestStatus::Sent {
            outcome: DecisionRunOutcome::Cancelled { .. }
        }
    ));
    assert_eq!(server.finish().await, 2);
    drop(held);
}
