#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/markdown_rule_fixture.rs"]
mod markdown_fixture;
#[path = "support/decision_http.rs"]
mod transport;
use markdown_fixture::input;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

#[tokio::test]
async fn generated_separator_matches_expand_to_four_section_keys_instead_of_false_zero_matches() {
    let root = tempfile::tempdir().unwrap();
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let request = input(root.path(), endpoint);
    std::fs::write(
        root.path().join("gated/source.md"),
        "## Heading\n\nFirst\n\nSecond\n",
    )
    .unwrap();
    let path = root.path().join("gated/rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["select"] = json!({"kind":"markdown_units","mode":"call_site","units":["section","paragraph"],"pattern":"\\n"});
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    let path = root.path().join("gated/set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["applicability"] = json!([]);
    std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
    let outcome = prepare_rules(
        request,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let RulePreparationOutcome::Prepared { preparation } = outcome else {
        panic!("{outcome:?}")
    };
    let wire = serde_json::to_value(preparation).unwrap();
    let evaluation = wire["rule_evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["key"]["rule_id"] == "many")
        .unwrap();
    assert_eq!(evaluation["kind"], "expanded");
    let children = evaluation["occurrences"].as_array().unwrap();
    assert_eq!(children.len(), 4);
    let ranges = children
        .iter()
        .map(|child| serde_json::to_string(&child["target"]).unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ranges.len(), 4);
    let owners = wire["planned_requests"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|request| request["questions"].as_array().unwrap())
        .filter(|question| question["owner"]["key"]["rule_id"] == "many")
        .count();
    assert_eq!(owners, 4);
}
