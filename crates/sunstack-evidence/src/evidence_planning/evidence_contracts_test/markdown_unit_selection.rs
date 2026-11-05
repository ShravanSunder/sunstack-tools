#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

#[test]
fn markdown_selector_wire_is_flat_strict_and_keeps_modes_distinct() {
    let schema = serde_json::to_value(schemars::schema_for!(EvidenceSelection)).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for value in [
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"]}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["section"], "starts_with":"Use"}),
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph","list_item"], "pattern":"Use"}),
    ] {
        let parsed = serde_json::from_value::<EvidenceSelection>(value.clone()).unwrap();
        assert!(validator.is_valid(&value), "{value}");
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    for value in [
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph"]}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"], "pattern":"Use", "starts_with":"Use"}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"], "typo":true}),
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph"], "pattern":"Use", "typo":true}),
    ] {
        assert!(
            serde_json::from_value::<EvidenceSelection>(value.clone()).is_err(),
            "{value}"
        );
        assert!(!validator.is_valid(&value), "{value}\n{schema}");
    }
}
