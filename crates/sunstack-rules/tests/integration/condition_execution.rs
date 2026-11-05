#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/decision_http.rs"]
mod support;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn leading_payload_conditions_pass_stop_or_fail_without_eager_document_reads() {
    for (payload, expected) in [
        (json!({"enabled":true}), "judged"),
        (json!({"enabled":false}), "not_applicable"),
        (json!({}), "unresolved"),
    ] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source.md"), "U001: real source\n").unwrap();
        std::fs::write(root.path().join("schema.json"),r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"additionalProperties":false}"#).unwrap();
        std::fs::write(root.path().join("set.json"),r#"{"schema_version":1,"name":"conditions","description":"condition controls","inputs":{"subject":{"kind":"document"},"flags":{"kind":"payload","schema":"flags"}},"applicability":[{"kind":"condition","test":{"kind":"payload_field_equals","input":"flags","pointer":"/enabled","value":true}}],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]}"#).unwrap();
        std::fs::write(root.path().join("rules.jsonl"),r#"{"rule_id":"check","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"check real source"},"threshold":0.8}}"#).unwrap();
        let replies = if expected == "judged" {
            vec![support::HttpReply::json(
                json!({"model":"typesafe/test","answers":{"q000001":{"type":"noul","noul":0.9}},"usage":{"input_tokens":10,"output_tokens":2}}),
            )]
        } else {
            vec![]
        };
        let server = support::TestServer::start(replies).await;
        let input:RuleInvocationInput=serde_json::from_value(json!({"definition_root":root.path(),"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"},"flags":{"kind":"payload","value":payload}}}]}]},"repositories":[{"key":"repo","root":root.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],"payload_schemas":[{"name":"flags","path":"schema.json"}],"origin":{"kind":"direct"},"cursors":[],"execution":{"speculation":"off","max_concurrent_requests":2},"limits":{"max_files":2,"max_file_bytes":16384,"max_total_bytes":32768,"max_matches":20,"max_transcript_bytes":16384,"include_ignored":false},"backend":support::selection(&server.endpoint),"attempts":sunstack_decisions::AttemptSettings::default(),"request_budget":support::budget()})).unwrap();
        let outcome = evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(support::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            other => panic!("{other:?}"),
        };
        let serialized: Value = serde_json::to_value(&report).unwrap();
        match expected {
            "judged" => {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::Judged { .. }
                ));
                assert_eq!(report.measurements.files_read, 1);
                assert!(matches!(
                    report.groups[0].applicability,
                    ApplicabilityOutcome::Applicable
                ));
                assert_eq!(server.finish().await, 1);
            }
            "not_applicable" => {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::NotApplicable { .. }
                ));
                assert_eq!(report.measurements.files_read, 0);
                assert!(matches!(
                    report.groups[0].applicability,
                    ApplicabilityOutcome::NotApplicable { .. }
                ));
                assert_eq!(server.finish().await, 0);
            }
            _ => {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::EvaluationFailed { .. }
                ));
                assert_eq!(report.measurements.files_read, 0);
                assert!(matches!(
                    report.groups[0].applicability,
                    ApplicabilityOutcome::ApplicabilityFailed { .. }
                ));
                assert_eq!(server.finish().await, 0);
            }
        }
        assert!(
            serialized["groups"][0]["entries"][0]["result"]["basis"]
                .get("value")
                .is_none()
        );
    }
}
