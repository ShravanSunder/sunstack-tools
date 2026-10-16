#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}

#[test]
fn omitted_discovery_and_mappings_have_explicit_native_defaults() {
    let root = tempfile::tempdir().unwrap();
    let value = serde_json::to_value(fixture::input(root.path(), "http://127.0.0.1:1/")).unwrap();
    assert_eq!(value["discovery"], json!({"kind":"none"}));
    assert_eq!(value["package_bindings"], json!([]));
}

#[test]
fn bounded_discovery_and_package_mappings_round_trip_the_governing_tables() {
    let root = tempfile::tempdir().unwrap();
    let mut value =
        serde_json::to_value(fixture::input(root.path(), "http://127.0.0.1:1/")).unwrap();
    let discovery = json!({"kind":"bounded","limits":{"max_files":30,"max_file_bytes":10000,"max_total_bytes":100000,"max_matches":100,"include_ignored":false,"max_unit_visits":100,"max_relationship_candidates":100,"max_fan_out":10,"max_history_commits":30,"max_local_time_ms":10000}});
    let mappings = json!([{"importing_repository_id":"repo","module_prefix":"@library/","target_repository_id":"library","target_path_prefix":"src","language":"typescript"}]);
    value["discovery"] = discovery.clone();
    value["package_bindings"] = mappings.clone();
    let input: RuleInvocationInput = serde_json::from_value(value).unwrap();
    let serialized = serde_json::to_value(input).unwrap();
    assert_eq!(serialized["discovery"], discovery);
    assert_eq!(serialized["package_bindings"], mappings);
}

#[tokio::test]
async fn full_scope_preparation_and_real_http_report_expose_selection_without_resolving_git() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    let mut server = controlled::ControlledServer::start().await;
    let input = fixture::input(root.path(), &server.endpoint);
    let prepared = prepare_rules(input.clone(), context(&server.endpoint)).await;
    let RulePreparationOutcome::Prepared { preparation } = prepared else {
        panic!("{prepared:?}")
    };
    let prepared = serde_json::to_value(preparation).unwrap();
    assert_eq!(
        prepared["occurrences"][0]["status"],
        json!({"kind":"planned","selection":{"kind":"selected","selected_by":["full_scope"]}})
    );
    assert_eq!(prepared["change_selections"], json!([]));
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let request = server.next().await;
    assert_eq!(
        request.body["state"]["evidence"]["rule:one/target"]["coverage"]["kind"],
        "single_source"
    );
    request.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let report: Value = serde_json::to_value(report).unwrap();
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["full_scope"])
    );
    assert_eq!(report["change_selections"], json!([]));
    assert_eq!(server.finish().await, 1);
}

#[test]
fn selected_reasons_are_closed_nonempty_unique_and_in_the_canonical_order() {
    for (value, valid) in [
        (json!([]), false),
        (json!(["full_scope"]), true),
        (
            json!([
                "target_changed",
                "context_changed",
                "before_analysis_unknown"
            ]),
            true,
        ),
        (json!(["full_scope", "target_changed"]), true),
        (json!(["context_changed", "target_changed"]), false),
        (json!(["new_rule", "new_rule"]), false),
        (json!(["unknown"]), false),
    ] {
        assert_eq!(
            serde_json::from_value::<SelectedReasons>(value).is_ok(),
            valid
        );
    }
    assert!(
        serde_json::from_value::<OccurrenceSelection>(
            json!({"kind":"comparison_pending","selected_by":["full_scope"]})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<OccurrenceSelection>(json!({"kind":"selected","selected_by":[]}))
            .is_err()
    );
}

#[tokio::test]
async fn payload_context_keeps_its_authored_input_and_group_in_single_source_coverage() {
    let root = tempfile::tempdir().unwrap();
    let mut rule = fixture::rule("one");
    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"flags","input":"flags","requirement":"required","select":{"kind":"whole_payload"}}));
    fixture::write_set(root.path(), &[rule], vec![]);
    let mut set: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    set["inputs"]["flags"] = json!({"kind":"payload","schema":"flags"});
    std::fs::write(root.path().join("set.json"), set.to_string()).unwrap();
    std::fs::write(root.path().join("flags.schema.json"),r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"],"additionalProperties":false}"#).unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut value = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    value["payload_schemas"] = json!([{"name":"flags","path":"flags.schema.json"}]);
    let groups = value["definitions"]["rule_sets"][0]["binding_groups"]
        .as_array_mut()
        .unwrap();
    let mut second = groups[0].clone();
    groups[0]["bindings"]["flags"] = json!({"kind":"payload","value":{"enabled":true}});
    second["bindings"]["flags"] = json!({"kind":"payload","value":{"enabled":false}});
    groups.push(second);
    let task = tokio::spawn(evaluate_rules(
        serde_json::from_value(value).unwrap(),
        context(&server.endpoint),
    ));
    let mut observed = Vec::new();
    for _ in 0..2 {
        let request = server.next().await;
        let attribution = &request.body["state"]["evidence"]["rule:one/flags"];
        assert_eq!(attribution["input"], "flags");
        let capture = &attribution["coverage"]["capture"];
        assert_eq!(attribution["coverage"]["kind"], "single_source");
        assert_eq!(capture["source"]["input"], "flags");
        let content = &request.body["state"]["contents"][attribution["content"].as_str().unwrap()];
        observed.push((
            capture["source"]["group_position"].as_u64().unwrap(),
            content["enabled"].as_bool().unwrap(),
        ));
        request.noul(0.9);
    }
    observed.sort();
    assert_eq!(observed, vec![(0, true), (1, false)]);
    assert!(matches!(
        task.await.unwrap(),
        RuleInvocationOutcome::CompleteReport { .. }
    ));
    assert_eq!(server.finish().await, 2);
}
