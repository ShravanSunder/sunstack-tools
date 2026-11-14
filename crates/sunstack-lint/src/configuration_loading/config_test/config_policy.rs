#![allow(clippy::unwrap_used)]
use crate::test_support::lint_fixture as fixture;
use crate::*;
use serde_json::json;

#[test]
fn missing_model_names_its_purpose_and_mechanical_zero_request_behavior() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config.as_object_mut().unwrap().remove("decision_model");
    fixture::config_file(root.path(), &config);
    let error = load_config(&discover_config(root.path(), None).unwrap()).unwrap_err();
    assert!(error.message.contains("missing field `decision_model`"));
    assert!(
        error.message.contains("asking rules and selections"),
        "{}",
        error.message
    );
    assert!(
        error
            .message
            .contains("mechanical-only rules send no model requests"),
        "{}",
        error.message
    );
}
#[test]
fn old_policy_and_backend_keys_are_refused_naming_their_replacements() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    for (key, replacement) in [
        ("rule_sets", "rulebook"),
        ("rules", "severity"),
        ("overrides", "severity"),
        ("ignore_patterns", "check.ignore"),
        ("backend", "decision_model"),
    ] {
        let mut value = baseline.clone();
        value[key] = json!({});
        fixture::config_file(root.path(), &value);
        let error = load_config(&discover_config(root.path(), None).unwrap()).unwrap_err();
        assert_eq!(error.code, LintInvalidCode::UnknownField);
        assert!(
            error.message.contains(key) && error.message.contains(replacement),
            "{}",
            error.message
        );
    }
}
