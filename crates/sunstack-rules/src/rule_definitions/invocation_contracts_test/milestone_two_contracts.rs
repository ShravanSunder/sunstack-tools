#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::test_support::rule_fixture as fixture;
use crate::*;
use serde_json::json;

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
