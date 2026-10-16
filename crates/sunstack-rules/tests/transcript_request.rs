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
fn select(user: bool) -> serde_json::Value {
    json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":if user {"included"} else {"excluded"},"assistant_messages":if user {"excluded"} else {"all"},"tool_use":{"kind":"excluded"},"hook_injected":"excluded"})
}
#[tokio::test]
async fn transcript_projections_keep_owner_labels_deduplicate_content_and_carry_an_alias_gate() {
    let root = tempfile::tempdir().unwrap();
    let mut first = fixture::rule("first");
    first["evidence"][0]["select"] = select(true);
    let mut second = fixture::rule("second");
    second["evidence"] = json!([{"id":"target","input":"alias","select":select(false)},{"id":"user_context","input":"alias","select":select(true)}]);
    let mut gate = fixture::decision("alias");
    gate["evidence"][0]["select"] = select(true);
    fixture::write_set(root.path(), &[first, second], vec![gate]);
    let path = root.path().join("set.json");
    let mut set: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"] = json!({"subject":{"kind":"conversation"},"alias":{"kind":"conversation"}});
    std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
    std::fs::write(root.path().join("session.jsonl"), b"{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"USER_ONLY\"}}\n{\"type\":\"assistant\",\"uuid\":\"a1\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"ASSISTANT_ONLY\"}]}}\n").unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut input = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    input["repositories"] = json!([]);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] = json!({"subject":{"kind":"conversation","transcript_path":root.path().join("session.jsonl"),"format":"claude_code_jsonl"},"alias":{"kind":"conversation","transcript_path":root.path().join("./session.jsonl"),"format":"claude_code_jsonl"}});
    let execution = tokio::spawn(evaluate_rules(
        serde_json::from_value(input).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    let request = server.next().await;
    let body = &request.body;
    assert_eq!(
        body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["q000001", "q000002", "q000003"]
    );
    let evidence = &body["state"]["evidence"];
    assert_eq!(
        evidence["rule:first/target"]["content"],
        evidence["rule:second/user_context"]["content"]
    );
    assert_ne!(
        evidence["rule:first/target"]["content"],
        evidence["rule:second/target"]["content"]
    );
    assert_eq!(body["state"]["contents"].as_object().unwrap().len(), 2);
    assert_eq!(
        serde_json::to_string(body)
            .unwrap()
            .matches("USER_ONLY")
            .count(),
        1
    );
    assert_eq!(
        serde_json::to_string(body)
            .unwrap()
            .matches("ASSISTANT_ONLY")
            .count(),
        1
    );
    assert_eq!(
        body["questions"]["q000002"]["instructions"]["reads"],
        json!(["rule:first/target"])
    );
    assert_eq!(
        body["questions"]["q000003"]["instructions"]["reads"],
        json!(["rule:second/target", "rule:second/user_context"])
    );
    for field in [
        "threshold",
        "severity",
        "weight",
        "block_at_or_below",
        "routes",
    ] {
        assert!(!serde_json::to_string(body).unwrap().contains(field));
    }
    request.noul(0.9);
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.measurements.model_requests, 1);
    assert!(
        report
            .occurrences
            .iter()
            .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
    );
    assert_eq!(server.finish().await, 1);
}
