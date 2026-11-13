#![allow(clippy::unwrap_used)]
use super::super::*;
use serde_json::json;

#[test]
fn html_units_kind_round_trips_exclusive_unit_matchers_and_schema() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for selection in [
        json!({"mode":"unit","units":["section"]}),
        json!({"mode":"unit","units":["section"],"starts_with":"<h2>"}),
        json!({"mode":"unit","units":["section"],"pattern":"(?i)diagram"}),
    ] {
        let wire = json!({"html_units":selection,"fields":{}});
        let parsed: EvidenceKindDefinition = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
        assert!(validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn html_units_kind_refuses_competing_matchers_and_invalid_selection_wire() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for selection in [
        json!(null),
        json!({"mode":"call_site","units":["section"],"pattern":"x"}),
        json!({"mode":"unit","units":[]}),
        json!({"mode":"unit","units":["paragraph"]}),
        json!({"mode":"unit","units":["section"],"starts_with":"x","pattern":"x"}),
        json!({"mode":"unit","units":["section"],"starts_with":null}),
        json!({"mode":"unit","units":["section"],"pattern":null}),
        json!({"mode":"unit","units":["section"],"starts_with":""}),
        json!({"mode":"unit","units":["section"],"pattern":""}),
        json!({"mode":"unit","units":["section"],"unknown":true}),
    ] {
        let wire = json!({"html_units":selection,"fields":{}});
        assert!(
            serde_json::from_value::<EvidenceKindDefinition>(wire.clone()).is_err(),
            "{wire}"
        );
        assert!(!validator.is_valid(&wire), "{wire}");
    }
    for competing in [
        json!({"rule":"r"}),
        json!({"inline_rule":"rule: {}"}),
        json!({"rule":"r","inline_rule":"rule: {}"}),
    ] {
        let mut wire = competing;
        wire["html_units"] = json!({"mode":"unit","units":["section"]});
        wire["fields"] = json!({});
        let error = serde_json::from_value::<EvidenceKindDefinition>(wire.clone()).unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
        assert!(!validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn html_units_kind_refuses_invalid_and_empty_matching_patterns() {
    for pattern in ["[", "a*", "\\b"] {
        let wire = json!({
            "html_units":{"mode":"unit","units":["section"],"pattern":pattern},
            "fields":{}
        });
        assert!(
            serde_json::from_value::<EvidenceKindDefinition>(wire).is_err(),
            "{pattern}"
        );
    }
}

#[test]
fn html_section_fields_round_trip_closed_text_modes() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for text in ["full", "own", "enclosing_own", "heading"] {
        let wire = json!({"from":"section","text":text});
        let parsed: FieldSource = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.shape(), FieldShape::Text);
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
        assert!(validator.is_valid(&wire), "{wire}");
    }
    for wire in [
        json!({"from":"section"}),
        json!({"from":"section","text":null}),
        json!({"from":"section","text":"first_line"}),
        json!({"from":"section","text":"own","rule":"r"}),
    ] {
        assert!(
            serde_json::from_value::<FieldSource>(wire.clone()).is_err(),
            "{wire}"
        );
        assert!(!validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn matcher_ast_rule_identity_is_absent_for_native_html_units() {
    let kind_name = EvidenceKindName::new("completion").unwrap();
    for (matcher, expected) in [
        (json!({"rule":"paragraph"}), Some("paragraph")),
        (json!({"inline_rule":"rule: {}"}), Some("completion")),
        (
            json!({"html_units":{"mode":"unit","units":["section"]}}),
            None,
        ),
    ] {
        let mut wire = matcher;
        wire["fields"] = json!({});
        let parsed: EvidenceKindDefinition = serde_json::from_value(wire).unwrap();
        assert_eq!(
            parsed
                .matcher
                .ast_rule_id(&kind_name)
                .as_ref()
                .map(EvidenceRuleId::as_str),
            expected
        );
    }
}

#[test]
fn heading_context_fields_round_trip_with_their_schema() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for from in ["enclosing", "same_file"] {
        let wire = json!({"from":from,"kind":"outline","text":"heading"});
        let parsed: FieldSource = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.shape(), FieldShape::Texts);
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
        assert!(validator.is_valid(&wire), "{wire}");
    }
}
