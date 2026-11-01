#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_rules::test_support::rule_fixture;
#[path = "../support/mod.rs"]
mod support;
use serde_json::{Value, json};
use support::*;

fn source_fixture() -> (tempfile::TempDir, Value) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let mut field = rule_fixture::rule("local.description");
    field["severity"] = json!("error");
    field["evidence"][0]["select"] = json!({"kind":"frontmatter_field","field":"description"});
    field["verdict"] = json!({"kind":"mechanical","explanation":"Require a description.","check":{"kind":"expect","expect":{"present":true}}});
    let mut peer = rule_fixture::rule("local.peer");
    peer["evidence"][0]["select"] =
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    peer["verdict"] = json!({"kind":"mechanical","explanation":"Require clear paragraphs.","check":{"kind":"expect","expect":{"starts_with":"Use"}}});
    rule_fixture::write_set(root, &[field, peer], vec![]);
    let mut value = serde_json::to_value(rule_fixture::input(root, "not-a-url")).unwrap();
    value["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["subject"]["path"] =
        json!("missing-reviewed.md");
    value["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"**"}]);
    (directory, value)
}
fn check_input(mut value: Value) -> Value {
    for key in [
        "package_bindings",
        "origin",
        "cursors",
        "execution",
        "backend",
        "attempts",
        "request_budget",
    ] {
        value.as_object_mut().unwrap().remove(key);
    }
    value["output"] = json!("assembled_rules");
    value
}
#[tokio::test]
async fn real_process_checks_and_prints_then_settles_frontmatter_without_credentials_or_a_constructible_backend()
 {
    let (directory, input) = source_fixture();
    let root = directory.path();
    let checked = response(
        &run_without_credential(root, "check_rules", check_input(input.clone())).await,
        0,
    );
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 2);
    assert!(
        checked["result"]["outcome"]["printed_rules"]
            .as_array()
            .unwrap()
            .iter()
            .all(|rule| rule["definition"]["verdict"]["kind"] == "mechanical")
    );
    let runtime = input;
    for (text, exit, state, result) in [
        (
            "---\ndescription: null\n---\n\nUse this peer.\n",
            0,
            "completed",
            Some("pass"),
        ),
        (
            "---\nname: other\n---\n\nUse this peer.\n",
            0,
            "completed",
            Some("fail"),
        ),
        ("Use this peer.\n", 3, "unresolved", None),
        (
            "---\ndescription: [\n---\n\nUse this peer.\n",
            3,
            "unresolved",
            None,
        ),
    ] {
        std::fs::write(root.join("missing-reviewed.md"), text).unwrap();
        let wire = response(
            &run_without_credential(root, "evaluate_rules", runtime.clone()).await,
            exit,
        );
        let report = &wire["result"]["outcome"]["report"];
        let field = report["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| result["key"]["rule_id"] == "local.description")
            .unwrap();
        assert_eq!(field["kind"], "checked");
        assert_eq!(field["declared_severity"], "error");
        assert_eq!(field["outcome"]["kind"], state);
        if let Some(result) = result {
            assert_eq!(field["outcome"]["result"], result);
        }
        let peers = report["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|result| result["key"]["rule_id"] == "local.peer")
            .collect::<Vec<_>>();
        assert!(!peers.is_empty());
        assert!(peers.iter().all(|peer| peer["outcome"]["result"] == "pass"));
        assert_eq!(report["measurements"]["model_requests"], 0);
        assert_eq!(report["measurements"]["inference_attempts"], 0);
        for request in report["requests"].as_array().unwrap() {
            assert_eq!(request["questions"], json!([]));
            assert_eq!(request["status"]["reason"]["kind"], "no_questions");
        }
        for field in ["judgment", "answers", "confidence", "receipt"] {
            assert!(field_absent_from_local_results(report, field));
        }
    }
}
fn field_absent_from_local_results(report: &Value, field: &str) -> bool {
    report["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .all(|result| result.get(field).is_none())
}

#[tokio::test]
async fn archived_marker_mechanical_example_is_refused_by_native_exec() {
    let root = workspace_root();
    let directory = root.join("docs/specs/2026-10-12-rust-rule-engine/examples");
    let check: Value = serde_json::from_slice(
        &std::fs::read(directory.join("mechanical.check.request.json")).unwrap(),
    )
    .unwrap();
    let checked = response(
        &run_without_credential(&root, "check_rules", check["input"].clone()).await,
        2,
    );
    assert_eq!(checked["result"]["outcome"]["kind"], "refused");
    let input: Value = serde_json::from_slice(
        &std::fs::read(directory.join("mechanical.invocation.json")).unwrap(),
    )
    .unwrap();
    let wire = response(
        &run_without_credential(&root, "evaluate_rules", input).await,
        2,
    );
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
}
