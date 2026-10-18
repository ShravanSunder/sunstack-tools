#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/markdown_rule_fixture.rs"]
mod markdown_fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

fn mechanical_fixture(
    root: &std::path::Path,
    endpoint: &str,
    same_input: bool,
) -> RuleInvocationInput {
    let mut input = markdown_fixture::input(root, endpoint);
    let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
        panic!("expected rule sets")
    };
    // The local peer completes synchronously before the gate's HTTP can start.
    rule_sets.swap(0, 1);
    input.request_budget.max_questions = 1;
    input.request_budget.max_requests = 1;
    for name in ["gated", "peer"] {
        let path = root.join(name).join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if name == "peer" {
            rule["evidence"][0]["select"] =
                json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
        }
        let expect = if name == "peer" {
            json!({"starts_with":"U001:"})
        } else {
            json!({"matches":if same_input {"INITIAL"} else {"UPDATED"}})
        };
        rule["verdict"] = json!({"kind":"mechanical","explanation":"Keep the captured target","check":{"kind":"expect","expect":expect}});
        rule["severity"] = json!("block");
        std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    }
    if same_input {
        let path = root.join("gated/set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["applicability"][0]["evidence"][0] =
            json!({"id":"context","input":"subject","select":{"kind":"whole_document"}});
        std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
    }
    input
}

#[tokio::test]
async fn staged_and_carried_gates_govern_local_outcomes_and_keep_the_completed_peer_without_model_children()
 {
    for same_input in [false, true] {
        for stop in ["pass", "nonpass", "failure", "cancel"] {
            let root = tempfile::tempdir().unwrap();
            let mut server = controlled::ControlledServer::start().await;
            let input = mechanical_fixture(root.path(), &server.endpoint, same_input);
            let cancellation = CancellationToken::new();
            let execution = tokio::spawn(evaluate_rules(
                input,
                RuleExecutionContext {
                    backend: Arc::new(transport::backend(&server.endpoint)),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: cancellation.clone(),
                },
            ));
            let mut held = Some(server.next().await);
            let gate = held.as_ref().unwrap();
            assert_eq!(
                gate.body["questions"].as_object().unwrap().len(),
                1,
                "local children must not consume question budget"
            );
            assert!(
                gate.body["questions"]["q000001"]["instructions"]["owner"]
                    .as_str()
                    .unwrap()
                    .starts_with("rule_set_decision:")
            );
            std::fs::write(
                root.path().join("gated/source.md"),
                "- Use alpha; Use beta; Use gamma. UPDATED\n",
            )
            .unwrap();
            match stop {
                "pass" => held.take().unwrap().noul(0.95),
                "nonpass" => held.take().unwrap().noul(0.1),
                "failure" => held
                    .take()
                    .unwrap()
                    .respond(503, json!({"error":"controlled failure"})),
                "cancel" => cancellation.cancel(),
                _ => unreachable!(),
            }
            let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
                .await
                .unwrap()
                .unwrap();
            // Keep the socket held through cancellation settlement; EOF must not
            // race the cancellation outcome we are claiming to observe.
            drop(held);
            assert_eq!(server.finish().await, 1);
            assert_eq!(
                outcome.exit_code(),
                match stop {
                    "pass" | "nonpass" => 0,
                    "cancel" => 4,
                    _ => 3,
                }
            );
            let report = match outcome {
                RuleInvocationOutcome::CompleteReport { report }
                | RuleInvocationOutcome::IncompleteReport { report } => report,
                RuleInvocationOutcome::Cancelled {
                    progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                    ..
                } => *partial_report,
                other => panic!("{other:?}"),
            };
            assert_eq!(report.measurements.model_requests, 1);
            assert_eq!(
                report.measurements.files_read,
                if !same_input && stop == "pass" { 3 } else { 2 }
            );
            let peer = report
                .occurrences
                .iter()
                .find(|result| result.key().rule_id.as_str() == "peer")
                .unwrap();
            assert!(
                matches!(
                    peer,
                    RuleOccurrenceResult::Checked {
                        outcome: LocalSettlement::Completed {
                            result: MechanicalResult::Pass,
                            ..
                        },
                        ..
                    }
                ),
                "{peer:?}"
            );
            let children = report
                .occurrences
                .iter()
                .filter(|result| result.key().rule_id.as_str() == "many")
                .collect::<Vec<_>>();
            assert_eq!(
                children.len(),
                if same_input || stop == "pass" { 3 } else { 0 }
            );
            for child in children {
                assert!(
                    match stop {
                        "pass" => matches!(
                            child,
                            RuleOccurrenceResult::Checked {
                                outcome: LocalSettlement::Completed {
                                    result: MechanicalResult::Pass,
                                    ..
                                },
                                ..
                            }
                        ),
                        "nonpass" => matches!(child, RuleOccurrenceResult::NotApplicable { .. }),
                        "failure" => matches!(child, RuleOccurrenceResult::EvaluationFailed { .. }),
                        "cancel" => matches!(child, RuleOccurrenceResult::Cancelled { .. }),
                        _ => false,
                    },
                    "same_input={same_input}, {stop}: {child:?}"
                );
            }
        }
    }
}
