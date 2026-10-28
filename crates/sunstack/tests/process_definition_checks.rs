#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/rule_source_fixture.rs"]
mod fixture;
mod support;
use serde_json::{Value, json};

fn definition_input(root: &std::path::Path) -> Value {
    json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":["set.json"]},
        "payload_schemas":[],"limits":{"max_file_bytes":65536},"output":"assembled_rules"})
}
#[tokio::test]
async fn definitions_check_and_print_without_any_target_binding_or_credential() {
    let (root, _) = fixture::source_fixture();
    let output =
        support::run_without_credential(root.path(), "check_rules", definition_input(root.path()))
            .await;
    let wire = support::response(&output, 0);
    let outcome = &wire["result"]["outcome"];
    assert_eq!(outcome["kind"], "checked");
    assert_eq!(outcome["checked_rule_count"], 1);
    assert_eq!(outcome["printed_rules"][0]["source"]["kind"], "markdown");
    assert_eq!(
        outcome["printed_rules"][0]["source"]["provenance"]["marker_id"],
        "K014"
    );
    assert_eq!(
        outcome["printed_rules"][0]["definition"]["rule_id"],
        "standard.visible"
    );
    assert!(!root.path().join("missing-reviewed.md").exists());
}
#[tokio::test]
async fn retired_bound_check_requests_are_refused_and_retired_fields_are_named() {
    let (root, checked) = fixture::source_fixture();
    let mut old = fixture::runtime_input(checked, "http://127.0.0.1:1/v1/systemone");
    for field in [
        "backend",
        "origin",
        "cursors",
        "execution",
        "attempts",
        "request_budget",
    ] {
        old.as_object_mut().unwrap().remove(field);
    }
    old["output"] = json!("assembled_rules");
    let output = support::run_without_credential(root.path(), "check_rules", old).await;
    let wire = support::response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "refused");
    for field in ["repositories", "discovery"] {
        let mut input = definition_input(root.path());
        input[field] = if field == "repositories" {
            json!([])
        } else {
            json!({"kind":"none"})
        };
        let output = support::run_without_credential(root.path(), "check_rules", input).await;
        let wire = support::response(&output, 2);
        assert!(
            wire["result"]["outcome"]["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["code"] == "unknown_field"
                    && item["location"]["field_path"]
                        .as_array()
                        .is_some_and(|path| path.contains(&json!(field))))
        );
    }
}
#[tokio::test]
async fn declaration_checks_reject_bad_selectors_without_acquiring_target_content() {
    for selector in [
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"["}),
        json!({"kind":"frontmatter_field","field":""}),
    ] {
        let (root, _) = fixture::source_fixture();
        let mut entries: Value = serde_json::from_slice(
            &std::fs::read(root.path().join("teaching.rules.json")).unwrap(),
        )
        .unwrap();
        entries["K014"]["target"]["select"] = selector;
        std::fs::write(root.path().join("teaching.rules.json"), entries.to_string()).unwrap();
        let output = support::run_without_credential(
            root.path(),
            "check_rules",
            definition_input(root.path()),
        )
        .await;
        let wire = support::response(&output, 2);
        let diagnostics = wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|item| item["code"] == "invalid_selector")
        );
        assert!(!root.path().join("missing-reviewed.md").exists());
    }
}

#[tokio::test]
async fn declared_payload_resources_are_compiled_even_without_selected_payload_evidence() {
    let (root, _) = fixture::source_fixture();
    let path = root.path().join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["settings"] = json!({"kind":"payload","schema":"settings"});
    std::fs::write(path, set.to_string()).unwrap();
    let schema_path = root.path().join("settings.schema.json");
    std::fs::write(&schema_path, r#"{"type":"object"}"#).unwrap();
    let mut input = definition_input(root.path());
    let missing = support::run_without_credential(root.path(), "check_rules", input.clone()).await;
    let wire = support::response(&missing, 2);
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "unknown_payload_schema")
    );
    input["payload_schemas"] = json!([{"name":"settings","path":"settings.schema.json"}]);
    let checked = support::run_without_credential(root.path(), "check_rules", input.clone()).await;
    assert_eq!(
        support::response(&checked, 0)["result"]["outcome"]["checked_rule_count"],
        1
    );
    std::fs::write(schema_path, r#"{"type":"impossible_type"}"#).unwrap();
    let invalid = support::run_without_credential(root.path(), "check_rules", input).await;
    let wire = support::response(&invalid, 2);
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "payload_schema_invalid")
    );
}
