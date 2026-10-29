#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::LoadTarget;

fn text(value: String) -> FieldValue {
    FieldValue::Text {
        value,
        spans: vec![],
    }
}
fn fields(value: FieldValue) -> BTreeMap<FieldName, FieldValue> {
    BTreeMap::from([(FieldName::new("body").unwrap(), value)])
}
fn bounded_size(view: &ModelView) -> usize {
    let ModelView::Bounded { fields, .. } = view else {
        panic!("expected bounded: {view:?}")
    };
    fields.values().map(model_value_chars).sum()
}

#[test]
fn scalar_cut_preserves_edges_and_raw_middle_with_unicode_character_accounting() {
    let value = format!("{}MIDDLE_TOKEN{}", "é".repeat(3500), "尾".repeat(3500));
    let raw = fields(text(value.clone()));
    let view = project_model_view(&raw, 6000);
    assert_eq!(bounded_size(&view), 6000);
    let ModelView::Bounded { fields, cut } = view else {
        panic!()
    };
    let ModelFieldValue::Text(cut_text) = &fields[&FieldName::new("body").unwrap()] else {
        panic!()
    };
    assert!(cut_text.starts_with('é') && cut_text.ends_with('尾'));
    assert!(cut_text.contains("[cut]") && !cut_text.contains("MIDDLE_TOKEN"));
    assert_eq!(cut, [FieldName::new("body").unwrap()]);
    assert_eq!(raw[&FieldName::new("body").unwrap()], text(value));
}

#[test]
fn eighty_item_list_reduces_strings_to_floor_then_trims_its_tail() {
    let raw = fields(FieldValue::Texts {
        values: (0..80)
            .map(|i| format!("{i}:{}", "x".repeat(300)))
            .collect(),
        spans: vec![],
    });
    let view = project_model_view(&raw, 6000);
    assert!(bounded_size(&view) <= 6000);
    let ModelView::Bounded { fields, .. } = &view else {
        panic!()
    };
    let ModelFieldValue::List(values) = &fields[&FieldName::new("body").unwrap()] else {
        panic!()
    };
    assert!(values.len() < 80);
    assert!(matches!(values.last(), Some(ModelFieldValue::Text(s)) if s.starts_with("[cut: ")));
    assert_eq!(view, project_model_view(&raw, 6000));
}

#[test]
fn target_openings_and_nested_skill_fields_are_bounded() {
    let raw = fields(FieldValue::Targets {
        values: (0..5)
            .map(|i| LoadTarget {
                name: format!("reference{i}"),
                path: format!("reference{i}.md").into(),
                exists: true,
                candidates: vec![],
                opening: Some("z".repeat(7000)),
                fields: Some(fields(text("nested".repeat(1000)))),
            })
            .collect(),
    });
    let view = project_model_view(&raw, 6000);
    assert!(bounded_size(&view) <= 6000);
    assert_eq!(view, project_model_view(&raw, 6000));
}

#[test]
fn reduction_ties_follow_field_order_and_scalar_floor_can_remain_too_large() {
    let raw = BTreeMap::from([
        (FieldName::new("alpha").unwrap(), text("a".repeat(700))),
        (FieldName::new("beta").unwrap(), text("b".repeat(700))),
    ]);
    let ModelView::Bounded { cut, .. } = project_model_view(&raw, 1000) else {
        panic!()
    };
    assert_eq!(cut, [FieldName::new("alpha").unwrap()]);
    let raw = (0..6)
        .map(|i| {
            (
                FieldName::new(format!("field_{}", char::from(b'a' + i))).unwrap(),
                text("x".repeat(201)),
            )
        })
        .collect();
    assert_eq!(
        project_model_view(&raw, 1000),
        ModelView::TooLarge {
            chars: 1200,
            limit: 1000
        }
    );
}
