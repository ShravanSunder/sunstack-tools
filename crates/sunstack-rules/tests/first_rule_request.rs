#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/decision_http.rs"]
mod support;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::{CancellationToken, JudgmentAnswer, QuestionName};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn real_rules_evidence_and_http_preserve_owner_labels_verdict_collision_and_locations() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("source.md"),
        "U001: selected original paragraph\n",
    )
    .unwrap();
    std::fs::write(root.path().join("set.json"),r#"{"schema_version":1,"name":"checks","description":"not model instructions","inputs":{"subject":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":{"context":"own common instructions"},"rule_paths":["rules.jsonl"]}"#).unwrap();
    std::fs::write(root.path().join("rules.jsonl"),r#"{"rule_id":"check","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{"verdict":{"type":"noul","instructions":"informational native question"}},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"own verdict"},"threshold":0.8,"block_at_or_below":0.3},"severity":"error"}"#).unwrap();
    let mut server=support::TestServer::start(vec![support::HttpReply::json(json!({"model":"typesafe/test-model","answers":{"q000001":{"type":"noul","noul":0.25},"q000002":{"type":"noul","noul":0.9}},"usage":{"input_tokens":50,"output_tokens":8,"cost":0.002}}))]).await;
    let input:RuleInvocationInput=serde_json::from_value(json!({
        "definition_root":root.path(),"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"}}}]}]},
        "repositories":[{"key":"repo","root":root.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"origin":{"kind":"direct"},"cursors":[],"execution":{"speculation":"off","max_concurrent_requests":2},"limits":{"max_files":2,"max_file_bytes":16384,"max_total_bytes":32768,"max_matches":20,"max_transcript_bytes":16384,"include_ignored":false},"backend":support::selection(&server.endpoint),"attempts":sunstack_decisions::AttemptSettings::default(),"request_budget":support::budget()
    })).unwrap();
    let context = RuleExecutionContext {
        backend: Arc::new(support::backend(&server.endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    };
    let outcome = evaluate_rules(input, context).await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.occurrences.len(), 1);
    assert_eq!(report.requests.len(), 1);
    assert_eq!(report.measurements.files_read, 1);
    assert_eq!(report.measurements.model_requests, 1);
    let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[0] else {
        panic!("not judged")
    };
    assert_eq!(judgment.verdict.value, 0.25);
    assert!(matches!(
        judgment.verdict.result,
        VerdictResult::Fail {
            activation: Activation::Error
        }
    ));
    assert!(
        matches!(&judgment.target,sunstack_evidence::TargetLocation::Paragraph{target_id,span} if target_id.as_str()=="U001"&&span.start_line==1)
    );
    let info = judgment
        .answers
        .get(&QuestionName::new("verdict").unwrap())
        .unwrap();
    assert_eq!(info.question_name.as_str(), "q000002");
    assert!(matches!(
        report.answer(info),
        Some(JudgmentAnswer::Noul {
            yes_probability: 0.9
        })
    ));
    let capture = server.captures.recv().await.unwrap();
    let wire: Value = serde_json::from_slice(&capture.body).unwrap();
    assert_eq!(
        wire["questions"]["q000001"]["instructions"]["owner"],
        "rule:check"
    );
    assert_eq!(
        wire["questions"]["q000001"]["instructions"]["reads"],
        json!(["rule:check/target"])
    );
    assert_eq!(
        wire["questions"]["q000002"]["instructions"]["question"],
        "informational native question"
    );
    assert!(wire["state"]["evidence"].get("rule:check/target").is_some());
    assert_eq!(
        String::from_utf8(capture.body)
            .unwrap()
            .matches("selected original paragraph")
            .count(),
        1
    );
    for policy in [
        "threshold",
        "block_at_or_below",
        "weight",
        "severity",
        "routes",
    ] {
        assert!(!serde_json::to_string(&wire).unwrap().contains(policy));
    }
    assert_eq!(server.finish().await, 1);
}
