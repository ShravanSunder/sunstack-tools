#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::test_support::rule_fixture as fixture;
use crate::*;
use serde_json::{Value, json};
fn conversation() -> Value {
    json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"excluded"},"hook_injected":"excluded"})
}
fn base(root: &std::path::Path, endpoint: &str) -> (Value, Value, Value) {
    std::fs::write(root.join("session.jsonl"),b"{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"SOURCE_CONTROL_TEXT\"}}\n").unwrap();
    let set = json!({"schema_version":1,"name":"admission","description":"source boundary","inputs":{"subject":{"kind":"conversation"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]});
    let mut rule = fixture::rule("one");
    rule["evidence"][0]["select"] = conversation();
    let mut input = serde_json::to_value(fixture::input(root, endpoint)).unwrap();
    input["repositories"] = json!([]);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] = json!({"subject":{"kind":"conversation","transcript_path":root.join("session.jsonl"),"format":"counting_claude"}});
    (set, rule, input)
}
#[test]
fn raw_nested_duplicate_keys_are_refused_before_any_typed_map_can_erase_them() {
    for raw in [
        r#"{"schema_version":1,"inputs":{"subject":{"kind":"document"},"subject":{"kind":"document"}}}"#,
        r#"{"rule_id":"one","questions":{"detail":{"type":"noul","instructions":"a"},"detail":{"type":"noul","instructions":"b"}}}"#,
        r#"{"rule_id":"one","verdict":{"kind":"noul","question":{"type":"noul","instructions":{"PRIVATE_AUTHOR_KEY":1,"PRIVATE_AUTHOR_KEY":2}},"threshold":0.8}}"#,
    ] {
        let diagnostics = if raw.contains("schema_version") {
            decode_rule_set(raw.as_bytes(), "set.json".as_ref()).unwrap_err()
        } else {
            decode_rule_definition(raw.as_bytes(), "rules.jsonl".as_ref(), 1).unwrap_err()
        };
        let safe = serde_json::to_string(&diagnostics).unwrap();
        assert!(safe.contains("duplicate_field"));
        assert!(!safe.contains("PRIVATE_AUTHOR_KEY"));
    }
    let root = tempfile::tempdir().unwrap();
    let (_, _, input) = base(root.path(), "http://127.0.0.1:1");
    let raw=input.to_string().replace("\"bindings\":{","\"bindings\":{\"subject\":{\"kind\":\"conversation\",\"transcript_path\":\"PRIVATE_AUTHOR_VALUE\",\"format\":\"counting_claude\"},");
    let diagnostics = decode_rule_invocation(raw.as_bytes()).unwrap_err();
    assert!(
        serde_json::to_string(&diagnostics)
            .unwrap()
            .contains("duplicate_field")
    );
    assert!(!format!("{diagnostics:?}").contains("PRIVATE_AUTHOR_VALUE"));
}
