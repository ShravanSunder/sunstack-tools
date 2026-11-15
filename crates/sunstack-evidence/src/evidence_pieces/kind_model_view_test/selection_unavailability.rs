use super::*;

#[test]
fn unavailable_selection_preserves_independent_html_projection_and_budget_cuts() {
    // Arrange selected fields beside healthy HTML, including one field that exceeds the budget.
    let project = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let definition = serde_json::from_value(json!({
        "rule":"html_kind", "fields":{
            "body":{"from":"first_line"},
            "long_body":{"from":"first_line"}
        }
    }))
    .unwrap();
    let fields = BTreeMap::from([
        (
            field_name("body"),
            text_field("<p>Healthy &amp; visible</p>"),
        ),
        (
            field_name("long_body"),
            text_field(&format!("<p>{}</p>", "x".repeat(400))),
        ),
        (
            field_name("selected"),
            FieldValue::Unavailable {
                reason: crate::FieldUnavailable::SelectionUnavailable,
            },
        ),
        (
            field_name("pending"),
            FieldValue::Unavailable {
                reason: crate::FieldUnavailable::SelectionPending,
            },
        ),
    ]);
    let original = fields.clone();

    // Act through the ordinary HTML projection and common budget owner.
    let view = serde_json::to_value(project_kind_model_view(
        &project,
        &kind,
        &definition,
        &fields,
        230,
    ))
    .unwrap();

    // Assert unavailable fields retain markers without erasing healthy projection or cut facts.
    assert_eq!(view["state"], "bounded");
    assert_eq!(view["fields"]["body"], "Healthy & visible");
    assert_eq!(view["fields"]["selected"], json!({"unavailable":true}));
    assert_eq!(view["fields"]["pending"], json!({"unavailable":true}));
    assert_eq!(view["cut"], json!(["long_body"]));
    let long_body = view["fields"]["long_body"].as_str().unwrap();
    assert_eq!(long_body.chars().count(), 213);
    assert!(long_body.contains("[cut]"));
    assert!(!long_body.contains('<'));
    assert_eq!(fields, original);
}

#[test]
fn wholly_blocked_fields_keep_unavailable_distinct_from_pending() {
    // Arrange a wholly blocked view, with no independent field to project.
    let project = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let definition = serde_json::from_value(json!({"rule":"html_kind","fields":{}})).unwrap();
    for (reasons, expected) in [
        (
            vec![crate::FieldUnavailable::SelectionPending],
            ModelView::SelectionPending,
        ),
        (
            vec![crate::FieldUnavailable::SelectionUnavailable],
            ModelView::SelectionUnavailable,
        ),
        (
            vec![
                crate::FieldUnavailable::SelectionPending,
                crate::FieldUnavailable::SelectionUnavailable,
            ],
            ModelView::SelectionUnavailable,
        ),
    ] {
        let fields = reasons
            .into_iter()
            .enumerate()
            .map(|(position, reason)| {
                let name = if position == 0 { "first" } else { "second" };
                (field_name(name), FieldValue::Unavailable { reason })
            })
            .collect();

        // Act and assert the terminal marker, including unavailable-over-pending priority.
        assert_eq!(
            project_kind_model_view(&project, &kind, &definition, &fields, 230),
            expected
        );
    }
}
