#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::EvidencePlanInput;
use serde_json::{Value, json};

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
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
