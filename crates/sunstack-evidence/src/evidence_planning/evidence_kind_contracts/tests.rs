#![allow(clippy::unwrap_used)]
use super::*;

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
