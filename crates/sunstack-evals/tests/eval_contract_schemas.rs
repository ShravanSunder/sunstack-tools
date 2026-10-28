#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::{Value, json};
use sunstack_evals::*;

fn published(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas")
        .join(name);
    let mut value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("$id");
    value
}

#[test]
fn published_eval_contracts_equal_the_native_closed_shapes() {
    for (name, native) in [
        (
            "score-eval-input.schema.json",
            schemars::schema_for!(EvalRunInput),
        ),
        (
            "score-eval-outcome.schema.json",
            schemars::schema_for!(EvalRunOutcome),
        ),
        (
            "case-set-manifest.schema.json",
            schemars::schema_for!(CaseSetManifest),
        ),
    ] {
        assert_eq!(
            published(name),
            serde_json::to_value(native).unwrap(),
            "{name}"
        );
    }
    let input = published("score-eval-input.schema.json");
    assert_eq!(input["additionalProperties"], false);
    for field in ["definitions", "definition_root", "payload_schemas"] {
        assert!(input["properties"].get(field).is_none());
    }
}

#[test]
fn manifest_roundtrip_retains_entry_and_mapping_and_refuses_missing_or_extra_fields() {
    let root = tempfile::tempdir().unwrap();
    let frozen = support::freeze(
        root.path(),
        vec![support::case(1, Label::Yes, Split::HeldOut)],
    );
    let valid = serde_json::to_value(frozen.manifest()).unwrap();
    assert_eq!(
        valid["entry"],
        json!({"kind":"rule_sets","rule_sets":["set.json"]})
    );
    assert_eq!(valid["payload_schemas"], json!([]));
    let roundtrip: CaseSetManifest = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(roundtrip).unwrap(), valid);
    for field in ["entry", "payload_schemas"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<CaseSetManifest>(missing).is_err());
    }
    let mut extra = valid;
    extra["entry"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<CaseSetManifest>(extra).is_err());
}
