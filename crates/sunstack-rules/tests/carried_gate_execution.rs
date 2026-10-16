#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/decision_http.rs"]
mod support;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn same_source_gate_rides_once_and_its_nonpass_discards_the_rule_answer() {
    for gate_probability in [0.9, 0.1] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source.md"), "U001: shared original\n").unwrap();
        std::fs::write(root.path().join("set.json"),r#"{"schema_version":1,"name":"carried","description":"same source","inputs":{"subject":{"kind":"document"}},"applicability":[{"kind":"decision","question":{"type":"noul","instructions":"gate only"},"evidence":[{"id":"context","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"threshold":0.6}],"common_ruleset_instructions":"COMMON_RULE_ONLY","rule_paths":["rules.jsonl"]}"#).unwrap();
        std::fs::write(root.path().join("rules.jsonl"),r#"{"rule_id":"check","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"rule only"},"threshold":0.8}}"#).unwrap();
        let mut server=support::TestServer::start(vec![support::HttpReply::json(json!({"model":"typesafe/test","answers":{"q000001":{"type":"noul","noul":gate_probability},"q000002":{"type":"noul","noul":0.9}},"usage":{"input_tokens":50,"output_tokens":4,"cost":0.002}}))]).await;
        let input:RuleInvocationInput=serde_json::from_value(json!({"definition_root":root.path(),"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"}}}]}]},"repositories":[{"key":"repo","root":root.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"origin":{"kind":"direct"},"cursors":[],"execution":{"speculation":"off","max_concurrent_requests":2},"limits":{"max_files":2,"max_file_bytes":16384,"max_total_bytes":32768,"max_matches":20,"max_transcript_bytes":16384,"include_ignored":false},"backend":support::selection(&server.endpoint),"attempts":sunstack_decisions::AttemptSettings::default(),"request_budget":support::budget()})).unwrap();
        let outcome = evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(support::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.requests.len(), 1);
        assert!(matches!(report.requests[0].mode, RequestMode::Speculative));
        assert_eq!(report.measurements.files_read, 1);
        if gate_probability >= 0.6 {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            ));
            assert!(report.requests[0].discarded.is_empty());
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable { .. }
            ));
            assert_eq!(report.requests[0].discarded.len(), 1);
            assert_eq!(
                report.requests[0].discarded[0].question_name.as_str(),
                "q000002"
            );
        }
        let wire: Value =
            serde_json::from_slice(&server.captures.recv().await.unwrap().body).unwrap();
        assert_eq!(wire["questions"].as_object().unwrap().len(), 2);
        assert_eq!(
            wire["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        assert!(
            wire["questions"]["q000001"]["instructions"]
                .get("common_ruleset_instructions")
                .is_none()
        );
        assert_eq!(
            wire["questions"]["q000002"]["instructions"]["common_ruleset_instructions"],
            "COMMON_RULE_ONLY"
        );
        assert_eq!(wire["state"]["evidence"].as_object().unwrap().len(), 2);
        assert_eq!(wire["state"]["contents"].as_object().unwrap().len(), 1);
        assert_eq!(server.finish().await, 1);
    }
}
