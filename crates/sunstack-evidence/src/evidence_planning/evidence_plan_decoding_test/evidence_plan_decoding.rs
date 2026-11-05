#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::{Diagnostic, decode_evidence_plan_input};
use serde_json::{Value, json};

fn valid_input() -> Value {
    json!({
        "repositories":[{"key":"repo","root":"/unopened","input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],
        "sources":{"subject":{"kind":"document","repository":"repo","path":"source.md"}},
        "payload_schemas":[],"cursors":[],"native_queries":[],"package_bindings":[],"discovery":{"kind":"none"},
        "declarations":[{"id":"target","input":"subject","requirement":"required","select":{"kind":"whole_document"}}],
        "limits":{"max_files":10,"max_file_bytes":10000,"max_total_bytes":100000,"max_matches":100,"max_transcript_bytes":10000,"include_ignored":false}
    })
}
fn input_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn assert_request_diagnostic(diagnostic: Box<Diagnostic>, code: &str, path: &[&str]) {
    let wire = serde_json::to_value(diagnostic).unwrap();
    assert_eq!(wire["code"], code);
    assert_eq!(wire["origin"], "request");
    assert_eq!(wire["retry_advice"]["kind"], "repair_request");
    assert_eq!(wire["location"], json!({"kind":"field","field_path":path}));
    assert!(wire["guidance"].as_str().unwrap().chars().count() <= 512);
    assert!(!wire["guidance"].as_str().unwrap().contains("selector"));
    let serialized = wire.to_string();
    for sentinel in [
        "author_secret_source",
        "AUTHOR_SECRET_KEY",
        "AUTHOR_SECRET_VALUE",
    ] {
        assert!(!serialized.contains(sentinel));
    }
}
#[test]
fn valid_plan_decodes_without_source_io_and_preserves_its_wire() {
    let valid = valid_input();
    let decoded = decode_evidence_plan_input(&input_bytes(&valid)).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), valid);
    let encoded = serde_json::to_vec(&decoded).unwrap();
    assert_eq!(
        serde_json::to_vec(&decode_evidence_plan_input(&encoded).unwrap()).unwrap(),
        encoded
    );
}
#[test]
fn unknown_scope_field_reports_request_class_and_scope_path() {
    let mut input = valid_input();
    input["repositories"][0]["change_scope"]["AUTHOR_SECRET_KEY"] = json!("AUTHOR_SECRET_VALUE");
    assert_request_diagnostic(
        decode_evidence_plan_input(&input_bytes(&input)).unwrap_err(),
        "unknown_field",
        &["repositories", "change_scope"],
    );
}
#[test]
fn duplicate_field_reports_request_class_and_containing_path() {
    let wire = String::from_utf8(input_bytes(&valid_input()))
        .unwrap()
        .replace("\"max_files\":10", "\"max_files\":10,\"max_files\":11");
    assert_request_diagnostic(
        decode_evidence_plan_input(wire.as_bytes()).unwrap_err(),
        "duplicate_field",
        &["limits"],
    );
}
#[test]
fn invalid_value_reports_request_class_and_owned_field_path() {
    let mut input = valid_input();
    input["limits"]["max_files"] = json!("AUTHOR_SECRET_VALUE");
    assert_request_diagnostic(
        decode_evidence_plan_input(&input_bytes(&input)).unwrap_err(),
        "invalid_request",
        &["limits", "max_files"],
    );
}
#[test]
fn trailing_json_reports_invalid_request_at_the_root() {
    let mut wire = input_bytes(&valid_input());
    wire.extend_from_slice(b" {}");
    assert_request_diagnostic(
        decode_evidence_plan_input(&wire).unwrap_err(),
        "invalid_request",
        &[],
    );
}
#[test]
fn caller_source_keys_never_become_location_segments_even_when_they_match_schema_keys() {
    for source_name in [
        "author_secret_source",
        "repositories",
        "limits",
        "max_files",
        "change_scope",
    ] {
        let mut input = valid_input();
        input["sources"] = json!({source_name:{"kind":"document","repository":"repo","path":"source.md","AUTHOR_SECRET_KEY":"AUTHOR_SECRET_VALUE"}});
        input["declarations"][0]["input"] = json!(source_name);
        assert_request_diagnostic(
            decode_evidence_plan_input(&input_bytes(&input)).unwrap_err(),
            "unknown_field",
            &["sources"],
        );
    }
}

#[test]
fn malformed_json_reports_invalid_request_at_the_root() {
    assert_request_diagnostic(
        decode_evidence_plan_input(b"{").unwrap_err(),
        "invalid_request",
        &[],
    );
}
#[test]
fn non_object_root_reports_invalid_request_at_the_root() {
    assert_request_diagnostic(
        decode_evidence_plan_input(b"[]").unwrap_err(),
        "invalid_request",
        &[],
    );
}
#[test]
fn arbitrary_payload_with_schema_like_keys_remains_open() {
    let mut valid = valid_input();
    valid["sources"]["payload_control"] = json!({"kind":"payload","schema":"flags","value":{"kind":"full","unexpected":1,"repositories":{"limits":{"max_files":"author data"}}}});
    let decoded = decode_evidence_plan_input(&input_bytes(&valid)).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);
}
