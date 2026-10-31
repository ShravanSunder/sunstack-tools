#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn within_rejects_a_string_empty_list_and_repeated_kind() {
    for within in [
        serde_json::json!("a"),
        serde_json::json!([]),
        serde_json::json!(["a", "a"]),
    ] {
        let definition = serde_json::json!({"rule":"r","fields":{},"within":within});
        let text = definition.to_string();
        let mut decoder = serde_json::Deserializer::from_str(&text);
        let result: Result<EvidenceKindDefinition, _> =
            serde_path_to_error::deserialize(&mut decoder);
        let error = result.unwrap_err();
        assert_eq!(error.path().to_string(), "within");
        if within == serde_json::json!(["a", "a"]) {
            assert!(error.to_string().contains('a'));
        }
    }
}

#[test]
fn within_accepts_one_or_more_kinds_and_preserves_config_order() {
    for within in [serde_json::json!(["a"]), serde_json::json!(["z", "a"])] {
        let definition = serde_json::json!({"rule":"r","fields":{},"within":within});
        let parsed: EvidenceKindDefinition = serde_json::from_value(definition.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), definition);
    }
}

#[test]
fn within_schema_requires_a_nonempty_unique_array() {
    let schema = schemars::schema_for!(EvidenceKindDefinition);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    assert!(validator.is_valid(&serde_json::json!({"rule":"r","fields":{}})));
    for (within, valid) in [
        (serde_json::json!(null), false),
        (serde_json::json!("a"), false),
        (serde_json::json!([]), false),
        (serde_json::json!(["a", "a"]), false),
        (serde_json::json!(["a"]), true),
        (serde_json::json!(["z", "a"]), true),
    ] {
        assert_eq!(
            validator.is_valid(&serde_json::json!({"rule":"r","fields":{},"within":within})),
            valid,
            "{within}"
        );
    }
}

#[test]
fn field_source_is_closed_and_field_names_exclude_reserved_names() {
    for value in [
        "text",
        "where",
        "standard",
        "Upper",
        "bad__name",
        "",
        "with2",
    ] {
        assert!(FieldName::new(value).is_err(), "{value}");
    }
    assert!(FieldName::new("completion_line").is_ok());
    for value in [
        r#"{"from":"first_line","extra":true}"#,
        r#"{"from":"target","regex":"x"}"#,
        r#"{"from":"unknown"}"#,
    ] {
        assert!(serde_json::from_str::<FieldSource>(value).is_err());
    }
    let source: FieldSource =
        serde_json::from_str(r#"{"from":"target","regex":"x","resolve":"path"}"#).unwrap();
    assert_eq!(source.shape(), FieldShape::Targets);
}

#[test]
fn match_sources_refuse_both_neither_and_unknown_keys_without_losing_rule_compatibility() {
    for source in [
        r#"{"from":"inside","rule":"r","kind":"k"}"#,
        r#"{"from":"enclosing"}"#,
        r#"{"from":"same_file","kind":"k","extra":true}"#,
    ] {
        assert!(
            serde_json::from_str::<FieldSource>(source).is_err(),
            "{source}"
        );
    }
    for source in [
        r#"{"from":"inside","rule":"r"}"#,
        r#"{"from":"enclosing","kind":"k"}"#,
        r#"{"from":"remainder","exclude":{"kind":"k"}}"#,
    ] {
        let parsed: FieldSource = serde_json::from_str(source).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::from_str::<serde_json::Value>(source).unwrap()
        );
    }
}

#[test]
fn source_keys_reject_present_null_and_non_string_values() {
    for from in ["inside", "enclosing", "same_file"] {
        for keys in [
            serde_json::json!({"rule":null,"kind":"k"}),
            serde_json::json!({"rule":"r","kind":null}),
            serde_json::json!({"rule":null}),
            serde_json::json!({"kind":null}),
            serde_json::json!({"rule":17}),
            serde_json::json!({"kind":false}),
        ] {
            let mut value = keys;
            value["from"] = from.into();
            assert!(
                serde_json::from_value::<FieldSource>(value.clone()).is_err(),
                "{value}"
            );
        }
        for keys in [
            serde_json::json!({"rule":"r"}),
            serde_json::json!({"kind":"k"}),
        ] {
            let mut value = keys;
            value["from"] = from.into();
            assert!(serde_json::from_value::<FieldSource>(value).is_ok());
        }
    }
}

#[test]
fn candidate_keys_reject_empty_and_noncanonical_intake_without_changing_valid_wire_keys() {
    for value in [
        "",
        "c",
        "c0",
        "c01",
        "C1",
        "c-1",
        "c+1",
        "c1.0",
        "c 1",
        " c1",
        "c1 ",
        "c١",
        "candidate",
    ] {
        assert!(
            serde_json::from_value::<CandidateKey>(serde_json::json!(value)).is_err(),
            "{value:?}"
        );
    }
    for (position, value) in [(0, "c1"), (1, "c2"), (9, "c10")] {
        let key: CandidateKey = serde_json::from_value(serde_json::json!(value)).unwrap();
        assert_eq!(key, CandidateKey::at(position));
        assert_eq!(serde_json::to_string(&key).unwrap(), format!("\"{value}\""));
    }
}

#[test]
fn within_null_is_refused_while_omission_means_no_containment() {
    let omitted: EvidenceKindDefinition =
        serde_json::from_value(serde_json::json!({"rule":"r","fields":{}})).unwrap();
    assert!(omitted.within.is_none());
    let error = serde_json::from_value::<EvidenceKindDefinition>(
        serde_json::json!({"rule":"r","fields":{},"within":null}),
    )
    .unwrap_err();
    assert!(error.to_string().contains("within"), "{error}");
}

#[test]
fn neighbour_schema_and_decode_agree_on_rule_only_counts() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for from in ["next", "previous"] {
        for count in [0, 1, 2, 5, 6] {
            let value = serde_json::json!({"from": from, "rule": "paragraph", "count": count});
            assert_eq!(
                validator.is_valid(&value),
                (1..=5).contains(&count),
                "{value}"
            );
            assert_eq!(
                serde_json::from_value::<FieldSource>(value.clone()).is_ok(),
                validator.is_valid(&value),
                "{value}"
            );
        }
        for value in [
            serde_json::json!({"from":from,"kind":"existing"}),
            serde_json::json!({"from":from,"rule":"paragraph","kind":"existing"}),
        ] {
            assert!(!validator.is_valid(&value), "{value}");
        }
    }
}

#[test]
fn target_exclusions_accept_a_rule_and_refuse_kind_and_unknown_keys() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for (exclude, valid) in [
        (serde_json::json!({"rule":"section"}), true),
        (serde_json::json!({"kind":"section"}), false),
        (
            serde_json::json!({"rule":"section","kind":"section"}),
            false,
        ),
        (serde_json::json!({"rule":"section","unknown":true}), false),
    ] {
        let value = serde_json::json!({"from":"target","regex":"`([^`]+)`","resolve":"path","exclude":exclude});
        assert_eq!(validator.is_valid(&value), valid);
        assert_eq!(serde_json::from_value::<FieldSource>(value).is_ok(), valid);
    }
}
