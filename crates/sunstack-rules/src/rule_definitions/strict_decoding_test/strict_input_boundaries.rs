#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::test_support::rule_fixture as fixture;
use crate::*;
use serde_json::{Value, json};

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn assert_unknown_field<T: std::fmt::Debug>(result: Result<T, DiagnosticList>) {
    let diagnostics = result.unwrap_err();
    assert_eq!(diagnostics.items.len(), 1);
    assert_eq!(diagnostics.omitted_count, 0);
    let ReportDiagnostic::Rule(diagnostic) = &diagnostics.items[0] else {
        panic!("expected the native rule decoder's diagnostic")
    };
    assert_eq!(diagnostic.code, RuleDiagnosticCode::UnknownField);
    assert!(matches!(
        diagnostic.location,
        RuleDiagnosticLocation::Definition { .. }
    ));
}
#[test]
fn invocation_decoder_refuses_nested_origin_and_agent_extras() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("control")], vec![]);
    let base = serde_json::to_value(fixture::input(root.path(), "http://127.0.0.1:9")).unwrap();
    for origin in [
        json!({"kind":"direct"}),
        json!({"kind":"hook","host":"test","event":"test","agent":{"kind":"main"}}),
        json!({"kind":"hook","host":"test","event":"test","agent":{"kind":"unknown"}}),
    ] {
        let mut valid = base.clone();
        valid["origin"] = origin;
        decode_rule_invocation(&bytes(&valid)).unwrap();
        let pointer = if valid["origin"]["kind"] == "direct" {
            "/origin"
        } else {
            "/origin/agent"
        };
        valid.pointer_mut(pointer).unwrap()["unexpected"] = json!(1);
        assert_unknown_field(decode_rule_invocation(&bytes(&valid)));
    }
}
