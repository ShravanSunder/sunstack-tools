#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/rule_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use std::path::Path;
use sunstack_evidence::EvidencePlanInput;
use sunstack_rules::*;

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
fn evidence_plan_refuses_nested_unit_extras_before_preparation() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.md"), "U001: input control\n").unwrap();
    let valid = json!({
        "repositories":[{"key":"repo","root":root.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],
        "sources":{"subject":{"kind":"document","repository":"repo","path":"source.md"}},
        "payload_schemas":[],"cursors":[],"native_queries":[],"package_bindings":[],"discovery":{"kind":"none"},
        "declarations":[{"id":"target","input":"subject","requirement":"required","select":{"kind":"whole_document"}}],
        "limits":{"max_files":10,"max_file_bytes":10000,"max_total_bytes":100000,"max_matches":100,"max_transcript_bytes":10000,"include_ignored":false}
    });
    let _: EvidencePlanInput = serde_json::from_slice(&bytes(&valid)).unwrap();
    for pointer in [
        "/repositories/0/change_scope",
        "/discovery",
        "/declarations/0/select",
    ] {
        let mut invalid = valid.clone();
        invalid.pointer_mut(pointer).unwrap()["unexpected"] = json!(1);
        assert!(
            serde_json::from_slice::<EvidencePlanInput>(&bytes(&invalid)).is_err(),
            "accepted {pointer}"
        );
    }
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
#[test]
fn rule_set_decoder_refuses_each_unit_input_declaration_extra() {
    for kind in ["document", "conversation", "change_set"] {
        let mut valid = json!({"schema_version":1,"name":"control","description":"control","inputs":{"subject":{"kind":kind}},"applicability":[],"common_ruleset_instructions":"control","rule_paths":["rules.jsonl"]});
        decode_rule_set(&bytes(&valid), Path::new("set.json")).unwrap();
        valid["inputs"]["subject"]["unexpected"] = json!(1);
        assert_unknown_field(decode_rule_set(&bytes(&valid), Path::new("set.json")));
    }
}
#[test]
fn router_decoder_refuses_nested_input_declaration_extra() {
    let mut valid = json!({"schema_version":1,"name":"control","description":"control","inputs":{"subject":{"kind":"document"}},"evidence":[fixture::declaration("subject","context")],"question":{"type":"choice","instructions":"route","criteria":{"chosen":"match"}},"routes":{"chosen":[{"rule_set":"set.json","inputs":{"subject":"subject"}}]},"confidence_floor":0.6});
    decode_router(&bytes(&valid), Path::new("router.json")).unwrap();
    valid["inputs"]["subject"]["unexpected"] = json!(1);
    assert_unknown_field(decode_router(&bytes(&valid), Path::new("router.json")));
}
#[test]
fn rule_decoder_refuses_nested_conversation_projection_extras() {
    let mut valid = fixture::rule("conversation-control");
    valid["evidence"][0]["select"] = json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"included"},"hook_injected":"excluded"});
    decode_rule_definition(&bytes(&valid), Path::new("rules.jsonl"), 3).unwrap();
    for pointer in [
        "/evidence/0/select/turns",
        "/evidence/0/select/tool_use/tools",
    ] {
        let mut invalid = valid.clone();
        invalid.pointer_mut(pointer).unwrap()["unexpected"] = json!(1);
        assert_unknown_field(decode_rule_definition(
            &bytes(&invalid),
            Path::new("rules.jsonl"),
            3,
        ));
    }
}
