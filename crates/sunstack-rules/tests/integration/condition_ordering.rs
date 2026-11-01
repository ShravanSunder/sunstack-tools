#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
fn add_payload(root: &std::path::Path, input: &mut Value, value: Value) {
    let path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["flags"] = json!({"kind":"payload","schema":"flags"});
    std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
    std::fs::write(root.join("schema.json"), r#"{"type":"object"}"#).unwrap();
    input["payload_schemas"] = json!([{"name":"flags","path":"schema.json"}]);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["flags"] =
        json!({"kind":"payload","value":value});
}
#[tokio::test]
async fn nonleading_payload_condition_never_prunes_its_earlier_carried_decision() {
    for (probability, missing) in [(0.9, false), (0.1, false), (0.9, true), (0.1, true)] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![
                fixture::decision("subject"),
                json!({"kind":"condition","test":{"kind":"payload_field_equals","input":"flags","pointer":"/enabled","value":true}}),
            ],
        );
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        add_payload(
            root.path(),
            &mut input,
            if missing {
                json!({})
            } else {
                json!({"enabled":false})
            },
        );
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let request = server.next().await;
        assert_eq!(
            request.body["questions"].as_object().unwrap().len(),
            2,
            "D must ride with the rule before C is evaluated"
        );
        request.respond(200,json!({"model":"typesafe/conditions","answers":{"q000001":{"type":"noul","noul":probability},"q000002":{"type":"noul","noul":0.9}},"usage":{"input_tokens":10,"output_tokens":3,"cost":0.002}}));
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::IncompleteReport { report } if missing && probability >= 0.6 => {
                report
            }
            RuleInvocationOutcome::CompleteReport { report } if !missing || probability < 0.6 => {
                report
            }
            other => panic!("{other:?}"),
        };
        if missing && probability >= 0.6 {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Condition { .. },
                    ..
                }
            ));
            assert!(matches!(
                report.groups[0].entries[1].result,
                EntryResult::Condition {
                    outcome: ConditionOutcome::Unresolved { .. },
                    ..
                }
            ));
            assert_eq!(server.finish().await, 1);
            continue;
        }
        let RuleOccurrenceResult::NotApplicable { stopped_at, .. } = &report.occurrences[0] else {
            panic!("{report:?}")
        };
        assert_eq!(stopped_at.position, usize::from(probability >= 0.6));
        if probability < 0.6 {
            assert!(matches!(
                report.groups[0].entries[1].result,
                EntryResult::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped
                }
            ));
        } else {
            assert!(matches!(
                report.groups[0].entries[1].result,
                EntryResult::Condition {
                    outcome: ConditionOutcome::DidNotPass,
                    ..
                }
            ));
        }
        assert_eq!(report.requests[0].discarded.len(), 1);
        assert_eq!(server.finish().await, 1);
    }
}
#[tokio::test]
async fn payload_presence_distinguishes_value_null_and_missing_without_reading_stopped_rules() {
    for (value, expected) in [
        (json!({"field":0}), "judged"),
        (json!({"field":null}), "judged"),
        (json!({}), "unresolved"),
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![
                json!({"kind":"condition","test":{"kind":"payload_field_present","input":"flags","pointer":"/field"}}),
            ],
        );
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        add_payload(root.path(), &mut input, value);
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let outcome=tokio::time::timeout(Duration::from_secs(3),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>request.noul(0.9)}}}).await.unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report } if expected != "unresolved" => report,
            RuleInvocationOutcome::IncompleteReport { report } if expected == "unresolved" => {
                report
            }
            _ => panic!("{expected}: {outcome:?}"),
        };
        match expected {
            "judged" => assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            )),
            "not_applicable" => assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable { .. }
            )),
            _ => assert!(matches!(
                report.groups[0].applicability,
                ApplicabilityOutcome::ApplicabilityFailed { .. }
            )),
        }
        assert_eq!(
            report.measurements.files_read,
            usize::from(expected == "judged")
        );
        assert_eq!(server.finish().await, usize::from(expected == "judged"));
    }
}
