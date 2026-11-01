#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/observed_backend.rs"]
mod observed;
#[path = "../support/decision_http.rs"]
mod transport;
use proptest::prelude::*;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[derive(Clone, Copy, Debug)]
enum ExecutionLayout {
    Staged,
    Speculative,
    Split,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum SemanticOccurrence {
    NotApplicable,
    Pass,
    FailWarn,
}
fn semantic_results(report: &RuleInvocationReport) -> Vec<SemanticOccurrence> {
    report
        .occurrences
        .iter()
        .map(|occurrence| match occurrence {
            RuleOccurrenceResult::NotApplicable { .. } => SemanticOccurrence::NotApplicable,
            RuleOccurrenceResult::Judged { judgment, .. } => match judgment.verdict.result {
                VerdictResult::Pass => SemanticOccurrence::Pass,
                VerdictResult::Fail {
                    activation: Activation::Warn,
                } => SemanticOccurrence::FailWarn,
                _ => panic!("unexpected severity"),
            },
            _ => panic!("completed-answer fixture produced {occurrence:?}"),
        })
        .collect()
}
fn respond(request: controlled::PendingHttpRequest, probabilities: [f64; 3]) {
    let answers = request.body["questions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, question)| {
            let owner = question["instructions"]["owner"].as_str().unwrap();
            let probability = match owner {
                "rule_set_decision:0" => probabilities[0],
                "rule:one" => probabilities[1],
                "rule:two" => probabilities[2],
                _ => panic!("unexpected owner"),
            };
            (name.clone(), json!({"type":"noul","noul":probability}))
        })
        .collect::<serde_json::Map<_, _>>();
    request.respond(200,json!({"model":"typesafe/canned","answers":answers,"usage":{"input_tokens":20,"output_tokens":3,"cost":0.001}}));
}
async fn run_layout(
    layout: ExecutionLayout,
    probabilities: [f64; 3],
    priority: [u8; 3],
) -> Vec<SemanticOccurrence> {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[fixture::rule("one"), fixture::rule("two")],
        vec![fixture::decision("other")],
    );
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    if !matches!(layout, ExecutionLayout::Staged) {
        input.execution.speculation = Speculation::On;
    }
    if matches!(layout, ExecutionLayout::Split) {
        input.request_budget.max_questions = 1;
    }
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
    let expected = match layout {
        ExecutionLayout::Staged => 1,
        ExecutionLayout::Speculative => 2,
        ExecutionLayout::Split => 3,
    };
    let mut requests = Vec::new();
    for priority in priority.into_iter().take(expected) {
        requests.push((priority, server.next().await));
    }
    requests.sort_by_key(|(priority, _)| *priority);
    for (_, request) in requests {
        respond(request, probabilities);
        assert!(matches!(
            observed::next_event(&mut events).await,
            observed::AdapterEvent::ResponseAdmitted
        ));
    }
    if matches!(layout, ExecutionLayout::Staged) && probabilities[0] >= 0.6 {
        respond(server.next().await, probabilities);
        assert!(matches!(
            observed::next_event(&mut events).await,
            observed::AdapterEvent::ResponseAdmitted
        ));
    }
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{layout:?}: {outcome:?}")
    };
    for request in &report.requests {
        if let RequestStatus::Sent {
            outcome:
                sunstack_decisions::DecisionRunOutcome::Completed {
                    answers, receipt, ..
                },
        } = &request.status
        {
            assert_eq!(answers.len(), request.questions.len());
            assert_eq!(receipt.usage.cost_usd, Some(0.001));
        }
    }
    for group in &report.groups {
        for entry in &group.entries {
            if let EntryResult::Decision {
                outcome:
                    DecisionEntryOutcome::Passed { answer }
                    | DecisionEntryOutcome::DidNotPass { answer },
                ..
            } = &entry.result
            {
                assert!(report.answer(answer).is_some());
            }
        }
    }
    if probabilities[0] < 0.6 && !matches!(layout, ExecutionLayout::Staged) {
        assert_eq!(
            report
                .requests
                .iter()
                .flat_map(|request| &request.discarded)
                .count(),
            2
        );
    }
    assert_eq!(server.finish().await, report.measurements.model_requests);
    semantic_results(&report)
}
#[tokio::test]
async fn handwritten_fixed_completed_answer_oracles_hold_for_staged_speculative_and_split_plans() {
    let cases = [
        (
            [0.1, 0.9, 0.9],
            vec![
                SemanticOccurrence::NotApplicable,
                SemanticOccurrence::NotApplicable,
            ],
        ),
        (
            [0.6, 0.79, 0.8],
            vec![SemanticOccurrence::FailWarn, SemanticOccurrence::Pass],
        ),
        (
            [0.9, 0.0, 1.0],
            vec![SemanticOccurrence::FailWarn, SemanticOccurrence::Pass],
        ),
    ];
    for (probabilities, expected) in cases {
        for layout in [
            ExecutionLayout::Staged,
            ExecutionLayout::Speculative,
            ExecutionLayout::Split,
        ] {
            assert_eq!(
                run_layout(layout, probabilities, [2, 1, 0]).await,
                expected,
                "{layout:?}"
            );
        }
    }
}
#[test]
fn random_completed_answers_and_actual_admission_orders_match_an_independent_staged_oracle() {
    // One runtime owns the property harness; block_on is its synchronous test entrypoint.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 32,
        source_file: Some(file!()),
        ..Default::default()
    });
    runner
        .run(
            &(0u16..=1000, 0u16..=1000, 0u16..=1000, any::<[u8; 3]>()),
            |(gate, one, two, priority)| {
                let probabilities = [
                    f64::from(gate) / 1000.0,
                    f64::from(one) / 1000.0,
                    f64::from(two) / 1000.0,
                ];
                // This reference uses only the reviewed0.6 gate/0.8 verdict thresholds; it never calls
                // planning, entry reduction, verdict evaluation or the production result projector.
                let expected = if gate < 600 {
                    vec![SemanticOccurrence::NotApplicable; 2]
                } else {
                    [one, two]
                        .into_iter()
                        .map(|probability| {
                            if probability >= 800 {
                                SemanticOccurrence::Pass
                            } else {
                                SemanticOccurrence::FailWarn
                            }
                        })
                        .collect()
                };
                for layout in [
                    ExecutionLayout::Staged,
                    ExecutionLayout::Speculative,
                    ExecutionLayout::Split,
                ] {
                    let actual = runtime.block_on(run_layout(layout, probabilities, priority));
                    prop_assert_eq!(&actual, &expected);
                }
                Ok(())
            },
        )
        .unwrap();
}
