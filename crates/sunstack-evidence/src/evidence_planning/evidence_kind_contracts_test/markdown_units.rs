#![allow(clippy::unwrap_used)]
use super::super::*;
use serde_json::json;

#[test]
fn markdown_units_kind_round_trips_existing_unit_selection_wire_and_schema() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for selection in [
        json!({"mode":"unit","units":["section"]}),
        json!({"mode":"unit","units":["paragraph"]}),
        json!({"mode":"unit","units":["list_item"]}),
        json!({"mode":"unit","units":["section","paragraph","list_item"]}),
        json!({"mode":"unit","units":["section"],"starts_with":"Heading"}),
        json!({"mode":"unit","units":["section"],"pattern":"(?i)diagram"}),
    ] {
        let wire = json!({"markdown_units":selection,"fields":{}});
        let parsed: EvidenceKindDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
        assert!(validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn markdown_units_kind_refuses_malformed_selection_wire_and_schema() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for selection in [
        json!(null),
        json!({"mode":"unit","units":["unknown"]}),
        json!({"mode":"unit","units":["section"],"starts_with":"x","pattern":"x"}),
        json!({"mode":"unit","units":["section"],"starts_with":null}),
        json!({"mode":"unit","units":["section"],"pattern":null}),
        json!({"mode":"unit","units":["section"],"unknown":true}),
        json!({"mode":"call_site","units":["section"]}),
    ] {
        let wire = json!({"markdown_units":selection,"fields":{}});
        assert!(
            serde_json::from_value::<EvidenceKindDefinition>(wire.clone()).is_err(),
            "{wire}"
        );
        assert!(!validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn kind_matcher_keys_are_exclusive_in_serde_and_schema() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    let matchers = [
        ("rule", json!("completion")),
        (
            "inline_rule",
            json!("language: Markdown\nrule: {kind: paragraph}"),
        ),
        ("html_units", json!({"mode":"unit","units":["section"]})),
        ("markdown_units", json!({"mode":"unit","units":["section"]})),
    ];
    for mask in 0_u8..16 {
        let mut wire = json!({"fields":{}});
        for (index, (key, value)) in matchers.iter().enumerate() {
            if mask & (1 << index) != 0 {
                wire[*key] = value.clone();
            }
        }
        let valid = mask.count_ones() == 1;
        assert_eq!(
            serde_json::from_value::<EvidenceKindDefinition>(wire.clone()).is_ok(),
            valid,
            "{wire}"
        );
        assert_eq!(validator.is_valid(&wire), valid, "{wire}");
    }
    for (null_key, _) in &matchers {
        for (other_key, other_value) in &matchers {
            let mut wire = json!({"fields":{}});
            wire[*other_key] = other_value.clone();
            wire[*null_key] = json!(null);
            assert!(
                serde_json::from_value::<EvidenceKindDefinition>(wire.clone()).is_err(),
                "{wire}"
            );
            assert!(!validator.is_valid(&wire), "{wire}");
        }
    }
}

#[test]
fn native_markdown_matcher_has_no_ast_rule_identity() {
    let kind_name = EvidenceKindName::new("completion").unwrap();
    let parsed: EvidenceKindDefinition = serde_json::from_value(json!({
        "markdown_units":{"mode":"unit","units":["section"]},
        "fields":{}
    }))
    .unwrap();
    assert_eq!(parsed.matcher.ast_rule_id(&kind_name), None);
}

#[test]
fn rulebook_markdown_units_schema_limits_modes_without_changing_low_level_call_site() {
    let selection = json!({"mode":"call_site","units":["paragraph"],"pattern":"Use"});
    let low_level: crate::MarkdownUnitSelection =
        serde_json::from_value(selection.clone()).unwrap();
    assert_eq!(serde_json::to_value(low_level).unwrap(), selection);
    let wire = json!({"markdown_units":selection,"fields":{}});
    let parsed: EvidenceKindDefinition = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    assert!(!validator.is_valid(&wire), "{wire}");
    for selection in [
        json!({"mode":"unit","units":[]}),
        json!({"mode":"unit","units":["section"],"starts_with":""}),
        json!({"mode":"unit","units":["section"],"pattern":""}),
    ] {
        let wire = json!({"markdown_units":selection,"fields":{}});
        assert!(!validator.is_valid(&wire), "{wire}");
    }
}
