#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_source_fixture.rs"]
mod source_fixture;
mod support;
use serde_json::{Value, json};
use support::*;

fn write_rules(root: &std::path::Path) {
    std::fs::write(
        root.join("teaching.md"),
        "Require a description.<!-- K014 -->\n\nRequire clear paragraphs.<!-- K015 -->\n",
    )
    .unwrap();
    std::fs::write(root.join("teaching.rules.json"),serde_json::to_vec(&json!({
        "K014":{"name":"local.description","target":{"input":"skill","select":{"kind":"frontmatter_field","field":"description"}},"expect":{"present":true},"severity":"block"},
        "K015":{"name":"local.peer","target":{"input":"skill","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},"expect":{"starts_with":"Use"}}
    })).unwrap()).unwrap();
}

#[tokio::test]
async fn a_nested_ast_grep_typo_is_skipped_while_a_valid_local_peer_still_runs() {
    let (directory, input) = source_fixture::source_fixture();
    let root = directory.path();
    write_rules(root);
    let mut rules: Value =
        serde_json::from_slice(&std::fs::read(root.join("teaching.rules.json")).unwrap()).unwrap();
    rules["K014"].as_object_mut().unwrap().remove("expect");
    rules["K014"]["target"]["select"] =
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    rules["K014"]["ast_grep"] =
        json!({"kind":"paragraph","inside":{"kind":"list_item","stop_by":"end"}});
    std::fs::write(
        root.join("teaching.rules.json"),
        serde_json::to_vec_pretty(&rules).unwrap(),
    )
    .unwrap();
    std::fs::write(root.join("missing-reviewed.md"), "Use the valid peer.\n").unwrap();
    let wire = response(
        &run_without_credential(
            root,
            "evaluate_rules",
            source_fixture::runtime_input(input, "not-a-url"),
        )
        .await,
        0,
    );
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["rule_coverage"], "partial");
    assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
    let skipped = &report["skipped_rules"][0];
    assert_eq!(skipped["kind"], "entry");
    assert_eq!(skipped["key"], "K014");
    assert!(
        skipped["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "unknown_field"
                && item["location"]["field_path"] == json!(["ast_grep", "inside", "stop_by"]))
    );
    assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
    assert_eq!(report["occurrences"][0]["key"]["rule_id"], "local.peer");
    assert_eq!(report["occurrences"][0]["outcome"]["result"], "pass");
    assert_eq!(report["measurements"]["model_requests"], 0);
}
#[tokio::test]
async fn real_process_checks_and_prints_then_settles_frontmatter_without_credentials_or_a_constructible_backend()
 {
    let (directory, input) = source_fixture::source_fixture();
    let root = directory.path();
    write_rules(root);
    let checked = response(
        &run_without_credential(root, "check_rules", input.clone()).await,
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
    let runtime = source_fixture::runtime_input(input, "not-a-url");
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
        assert_eq!(field["declared_severity"], "block");
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
async fn reviewed_mechanical_example_checks_prints_and_evaluates_without_a_credential() {
    let root = workspace_root();
    let directory = root.join("docs/specs/2026-10-12-rust-rule-engine/examples");
    let check: Value = serde_json::from_slice(
        &std::fs::read(directory.join("mechanical.check.request.json")).unwrap(),
    )
    .unwrap();
    let checked = response(
        &run_without_credential(&root, "check_rules", check["input"].clone()).await,
        0,
    );
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 2);
    let input: Value = serde_json::from_slice(
        &std::fs::read(directory.join("mechanical.invocation.json")).unwrap(),
    )
    .unwrap();
    let wire = response(
        &run_without_credential(&root, "evaluate_rules", input).await,
        0,
    );
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["measurements"]["model_requests"], 0);
    assert_eq!(report["occurrences"].as_array().unwrap().len(), 2);
    assert!(
        report["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|result| result["kind"] == "checked" && result["outcome"]["result"] == "pass")
    );
}
