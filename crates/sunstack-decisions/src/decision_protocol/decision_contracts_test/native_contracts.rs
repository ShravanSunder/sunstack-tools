#![allow(clippy::unwrap_used)]
use crate::*;
use serde_json::json;

#[test]
fn numeric_and_boolean_entry_roots_are_refused_but_nested_values_are_allowed() {
    for entry in [json!(true), json!(1)] {
        assert!(serde_json::from_value::<StructuredEntry>(entry).is_err());
    }
    for state in [json!(true), json!(1), json!(null)] {
        assert!(serde_json::from_value::<SharedEvidenceState>(state).is_err());
    }
}

#[test]
fn missing_fields_partial_noul_unknown_fields_and_empty_identifiers_are_refused() {
    for question in [
        json!({"type":"noul"}),
        json!({"type":"noul","instructions":null,"criteria":null}),
        json!({"type":"noul","instructions":null,"criteria":{"true":null}}),
        json!({"type":"noul","instructions":null,"criteria":{"false":null}}),
        json!({"type":"noul","instructions":null,"extra":"sentinel"}),
        json!({"type":"choice","instructions":null,"criteria":{"":null}}),
        json!({"type":"other","instructions":null}),
    ] {
        assert!(
            serde_json::from_value::<JudgmentQuestion>(question.clone()).is_err(),
            "accepted {question}"
        );
    }
    assert!(QuestionName::new("").is_err());
    let omitted: JudgmentQuestion =
        serde_json::from_value(json!({"type":"noul","instructions":null})).unwrap();
    assert_eq!(
        serde_json::to_value(omitted).unwrap(),
        json!({"type":"noul","instructions":null})
    );
}
