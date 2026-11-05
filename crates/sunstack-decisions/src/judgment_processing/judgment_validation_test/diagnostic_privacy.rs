#![allow(clippy::unwrap_used, clippy::panic)]
use crate::test_support as support;
use crate::*;
use serde_json::json;
use support::*;

#[test]
fn request_decode_errors_hide_arbitrary_keys_values_and_keep_safe_locations() {
    let mut value = serde_json::to_value(mixed_input("http://localhost/")).unwrap();
    value["request"]["questions"]["SENTINEL_AUTHOR_KEY"] =
        json!({"type":"noul","instructions":null,"SENTINEL_UNKNOWN_FIELD":"SENTINEL_VALUE"});
    let diagnostic =
        sunstack_decisions::judgment_processing::judgment_validation::decode_run_input(
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap_err();
    assert_eq!(diagnostic.code, DecisionDiagnosticCode::UnknownField);
    assert!(
        matches!(diagnostic.location,DecisionDiagnosticLocation::Field {ref field_path} if field_path==&vec!["request".to_string(),"questions".to_string()])
    );
    for rendered in [
        format!("{diagnostic}"),
        format!("{diagnostic:?}"),
        serde_json::to_string(&diagnostic).unwrap(),
    ] {
        assert!(!rendered.contains("SENTINEL"));
    }
    let body = serde_json::to_string(&mixed_input("http://localhost/")).unwrap();
    let duplicate = body.replacen("\"questions\":{", "\"questions\":{},\"questions\":{", 1);
    assert_eq!(
        sunstack_decisions::judgment_processing::judgment_validation::decode_run_input(
            duplicate.as_bytes()
        )
        .unwrap_err()
        .code,
        DecisionDiagnosticCode::DuplicateField
    );
}
