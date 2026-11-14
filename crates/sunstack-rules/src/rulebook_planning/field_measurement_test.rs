#![allow(clippy::unwrap_used)]
use super::*;
use crate::{CountBound, DecisionMechanicalTool};
use serde_json::json;
use sunstack_evidence::project_model_view;

#[test]
fn count_bounds_are_exclusive_nonnegative_integers_and_test_exact_edges() {
    for (bound, below, equal, above) in [
        (json!({"at_least":2}), false, true, true),
        (json!({"at_most":2}), true, true, false),
        (json!({"greater_than":2}), false, false, true),
    ] {
        let bound: CountBound = serde_json::from_value(bound).unwrap();
        assert_eq!(
            (bound.matches(1), bound.matches(2), bound.matches(3)),
            (below, equal, above)
        );
    }
    for bound in [
        json!({}),
        json!({"at_least":-1}),
        json!({"at_most":1.5}),
        json!({"greater_than":null}),
        json!({"at_least":1,"at_most":2}),
        json!({"at_least":1,"extra":2}),
    ] {
        assert!(
            serde_json::from_value::<DecisionMechanicalTool>(
                json!({"tool":"word_count","field":"body","bound":bound})
            )
            .is_err()
        );
    }
}

#[test]
fn count_values_include_empty_pieces_but_refuse_unavailable_and_partial_text() {
    let mut piece = super::super::rulebook_test_support::piece();
    let field = FieldName::new("completion").unwrap();
    piece.fields.insert(
        field.clone(),
        FieldValue::Texts {
            values: vec!["One two".into(), "".into(), "three".into()],
            spans: vec![],
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    assert_eq!(
        FieldMeasure::Words.read(&piece, &field, &Default::default()),
        Ok(3)
    );
    assert_eq!(
        FieldMeasure::Pieces.read(&piece, &field, &Default::default()),
        Ok(3)
    );
    assert_eq!(
        render_measured_finding(
            "{{word_count:completion}} / {{piece_count:completion}} / {{word_count:completion}}",
            &piece,
            &Default::default()
        )
        .unwrap(),
        "3 / 3 / 3"
    );
    if let ModelView::Bounded { cut, .. } = &mut piece.model_view {
        cut.push(field.clone());
    }
    assert_eq!(
        FieldMeasure::Words.read(&piece, &field, &Default::default()),
        Err(field.clone())
    );
    assert_eq!(
        FieldMeasure::Pieces.read(&piece, &field, &Default::default()),
        Ok(3)
    );
    for value in [
        FieldValue::Unavailable {
            reason: sunstack_evidence::FieldUnavailable::SelectionPending,
        },
        FieldValue::MissingDocument { candidates: vec![] },
    ] {
        piece.fields.insert(field.clone(), value);
        assert_eq!(
            FieldMeasure::Pieces.read(&piece, &field, &Default::default()),
            Err(field.clone())
        );
        assert_eq!(
            FieldMeasure::Words.read(&piece, &field, &Default::default()),
            Err(field.clone())
        );
    }
    piece.fields.insert(field.clone(), FieldValue::Absent);
    piece.model_view = project_model_view(&piece.fields, 6000);
    assert_eq!(
        FieldMeasure::Pieces.read(&piece, &field, &Default::default()),
        Ok(0)
    );
    assert_eq!(
        FieldMeasure::Words.read(&piece, &field, &Default::default()),
        Ok(0)
    );
}
