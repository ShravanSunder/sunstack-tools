#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn containing_schema_agrees_with_the_decoder_on_omission_and_null() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for from in ["same_file", "document"] {
        for containing in [None, Some(json!("identifier")), Some(json!(null))] {
            let mut value = json!({"from":from,"rule":"rows"});
            if from == "document" {
                value["path"] = json!("sibling.html");
            }
            if let Some(containing) = containing {
                value["containing"] = containing;
            }
            assert_eq!(
                validator.is_valid(&value),
                serde_json::from_value::<FieldSource>(value.clone()).is_ok(),
                "{value}"
            );
        }
    }
}

#[test]
fn containing_references_are_typed_acyclic_and_independent_of_field_name_order() {
    let valid: EvidenceKindDefinition = serde_json::from_value(json!({"rule":"r","fields":{
        "a_rows":{"from":"same_file","rule":"r","containing":"b_rows"},
        "b_rows":{"from":"document","path":"sibling.md","rule":"r","containing":"zid"},
        "zid":{"from":"capture","regex":"(U1)"}
    }}))
    .unwrap();
    assert_eq!(
        valid
            .derived_field_order()
            .unwrap()
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>(),
        ["b_rows", "a_rows"]
    );
    assert!(valid.field_needs_document(&FieldName::new("a_rows").unwrap()));
    for (fields, message) in [
        (
            json!({"rows":{"from":"same_file","rule":"r","containing":"missing"}}),
            "defined text",
        ),
        (
            json!({"rows":{"from":"same_file","rule":"r","containing":"rows"}}),
            "cycle",
        ),
        (
            json!({"rows":{"from":"same_file","rule":"r","containing":"other"},"other":{"from":"document","path":"x","rule":"r","containing":"rows"}}),
            "cycle",
        ),
        (
            json!({"rows":{"from":"same_file","rule":"r","containing":"links"},"links":{"from":"target","regex":"(x)","resolve":"path"}}),
            "defined text",
        ),
    ] {
        let definition: EvidenceKindDefinition =
            serde_json::from_value(json!({"rule":"r","fields":fields})).unwrap();
        let refused = crate::EvidenceKindPlan::derive(&BTreeMap::from([(
            crate::EvidenceKindName::new("subject").unwrap(),
            definition,
        )]))
        .unwrap_err();
        assert!(
            refused.iter().any(|error| error.message.contains(message)),
            "{refused:?}"
        );
    }
    for from in ["same_file", "document"] {
        for containing in [json!(null), json!(""), json!(["identifier"]), json!(12)] {
            let mut value = json!({"from":from,"rule":"r","containing":containing});
            if from == "document" {
                value["path"] = json!("sibling.md");
            }
            assert!(serde_json::from_value::<FieldSource>(value).is_err());
        }
    }
}

#[test]
fn captures_schema_and_decoder_agree_and_named_inputs_are_typed_and_acyclic() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for of in [
        None,
        Some(json!("body")),
        Some(json!(null)),
        Some(json!("")),
        Some(json!(["body"])),
    ] {
        let mut value = json!({"from":"captures","regex":"U[0-9]+"});
        if let Some(of) = of {
            value["of"] = of;
        }
        assert_eq!(
            validator.is_valid(&value),
            serde_json::from_value::<FieldSource>(value.clone()).is_ok(),
            "{value}"
        );
    }
    for (fields, valid) in [
        (json!({"ids":{"from":"captures","regex":"U[0-9]+"}}), true),
        (
            json!({"ids":{"from":"captures","of":"body","regex":"U[0-9]+"},"body":{"from":"first_line"}}),
            true,
        ),
        (
            json!({"ids":{"from":"captures","of":"missing","regex":"U"}}),
            false,
        ),
        (
            json!({"ids":{"from":"captures","of":"links","regex":"U"},"links":{"from":"target","regex":"(x)","resolve":"path"}}),
            false,
        ),
        (
            json!({"ids":{"from":"captures","of":"rows","regex":"U"},"rows":{"from":"document","path":"x","rule":"r","containing":"ids"}}),
            false,
        ),
        (
            json!({"ids":{"from":"captures","of":"ids","regex":"U"}}),
            false,
        ),
    ] {
        let definition: EvidenceKindDefinition =
            serde_json::from_value(json!({"rule":"r","fields":fields})).unwrap();
        assert_eq!(
            crate::EvidenceKindPlan::derive(&BTreeMap::from([(
                crate::EvidenceKindName::new("subject").unwrap(),
                definition
            )]))
            .is_ok(),
            valid,
            "{fields}"
        );
    }
}

#[test]
fn attachment_schema_and_decoder_require_a_positive_cap_and_closed_envelope() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for from in ["same_file", "document"] {
        for (attachment, valid) in [
            (json!({"until":"boundary","max_siblings":12}), true),
            (json!({"until":"boundary","max_siblings":0}), false),
            (json!({"until":"boundary","max_siblings":-1}), false),
            (json!({"until":"boundary","max_siblings":1.5}), false),
            (json!({"until":"boundary"}), false),
            (
                json!({"until":"boundary","max_siblings":12,"extra":true}),
                false,
            ),
            (json!(null), false),
        ] {
            let mut source = json!({"from":from,"rule":"r","attach":attachment});
            if from == "document" {
                source["path"] = json!("other.md");
            }
            assert_eq!(
                serde_json::from_value::<FieldSource>(source.clone()).is_ok(),
                valid,
                "{source}"
            );
            assert_eq!(validator.is_valid(&source), valid, "{source}");
        }
    }
}
