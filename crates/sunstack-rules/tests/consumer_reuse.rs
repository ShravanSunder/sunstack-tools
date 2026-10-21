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
fn favorable(report: &RuleInvocationReport) -> bool {
    !report.occurrences.is_empty() && report.occurrences.iter().all(|result| matches!(result, RuleOccurrenceResult::Judged { judgment, .. } if matches!(judgment.verdict.result, VerdictResult::Pass)))
}
#[tokio::test]
async fn lint_and_eval_consumers_share_definitions_but_keep_independent_expectations_and_failure_controls()
 {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("same")], vec![]);
    // These expected verdicts are supplied by the consumers; none is derived from
    // the engine's reducer. Completeness and a favorable verdict are distinct.
    let lint_cases = [
        ("at", true),
        ("below", false),
        ("provider", false),
        ("invalid", false),
        ("cancel", false),
        ("budget", false),
        ("selection", false),
    ];
    let eval_cases = [
        ("below", false),
        ("at", true),
        ("selection", false),
        ("budget", false),
        ("cancel", false),
        ("invalid", false),
        ("provider", false),
    ];
    for (driver, cases) in [("lint", lint_cases), ("eval", eval_cases)] {
        for (case, passes) in cases {
            let mut server = controlled::ControlledServer::start().await;
            std::fs::write(
                root.path().join("source.md"),
                format!("U001: {driver} source\n"),
            )
            .unwrap();
            let mut input = fixture::input(root.path(), &server.endpoint);
            input.attempts.retry_count = 0;
            if case == "budget" {
                input.request_budget.max_request_bytes = 1;
            }
            if case == "selection" {
                std::fs::write(root.path().join("source.md"), "no authored paragraph\n").unwrap();
            }
            let cancellation = CancellationToken::new();
            let execution = tokio::spawn(evaluate_rules(
                input,
                RuleExecutionContext {
                    backend: Arc::new(transport::backend(&server.endpoint)),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: cancellation.clone(),
                },
            ));
            let sends = !matches!(case, "budget" | "selection");
            if sends {
                let request = server.next().await;
                assert!(
                    serde_json::to_string(&request.body["state"])
                        .unwrap()
                        .contains(&format!("{driver} source"))
                );
                match case {
                    "at" => request.noul(0.8),
                    "below" => request.noul(0.7999),
                    "provider" => request.respond(503, json!({"error":"controlled"})),
                    "invalid" => request.respond(200, json!({"model":"typesafe/test","answers":{},"usage":{"input_tokens":1,"output_tokens":1}})),
                    "cancel" => { cancellation.cancel(); drop(request); },
                    _ => unreachable!(),
                }
            }
            let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
                .await
                .unwrap()
                .unwrap();
            let report = match outcome {
                RuleInvocationOutcome::CompleteReport { report }
                    if matches!(case, "at" | "below") =>
                {
                    report
                }
                RuleInvocationOutcome::IncompleteReport { report }
                    if matches!(case, "provider" | "invalid" | "budget" | "selection") =>
                {
                    report
                }
                RuleInvocationOutcome::Cancelled {
                    progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                    ..
                } if case == "cancel" => *partial_report,
                other => panic!("{driver}/{case}: {other:?}"),
            };
            assert_eq!(favorable(&report), passes, "{driver}/{case}");
            if matches!(case, "at" | "below") {
                let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[0] else {
                    panic!("{report:?}")
                };
                let VerdictBasis::Noul { answer } = &judgment.verdict.basis else {
                    panic!("{judgment:?}")
                };
                assert!(report.answer(answer).is_some());
                assert_eq!(judgment.declared_severity, Severity::Warn);
                assert!(
                    serde_json::to_string(&judgment.target)
                        .unwrap()
                        .contains("U001")
                );
            } else {
                assert!(
                    !report
                        .occurrences
                        .iter()
                        .any(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
                );
            }
            assert_eq!(server.finish().await, usize::from(sends));
        }
    }
}
