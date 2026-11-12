#![allow(clippy::unwrap_used)]
use super::super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use serde_json::json;

#[test]
fn html_units_admit_section_fields_without_an_ast_rule() {
    let kinds = serde_json::from_value(json!({
        "outline": {
            "html_units":{"mode":"unit","units":["section"]},
            "fields": {
                "complete":{"from":"section","text":"full"},
                "own_body":{"from":"section","text":"own"},
                "parent_body":{"from":"section","text":"enclosing_own"}
            }
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn html_section_source_on_ast_kind_has_named_admission_refusal() {
    let kinds = serde_json::from_value(json!({
        "ast_section": {
            "rule":"completion",
            "fields":{"own_body":{"from":"section","text":"own"}}
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].field_path.to_string(),
        "evidence.kinds.ast_section.fields.own_body"
    );
    for expected in ["ast_section", "own_body", "section", "html_units"] {
        assert!(errors[0].message.contains(expected), "{errors:?}");
    }
}

#[test]
fn html_units_kind_may_share_a_name_with_an_existing_ast_rule() {
    let project = fixture::project();
    let ast_rule = sunstack_evidence::EvidenceRuleId::new("completion").unwrap();
    assert!(project.rule_ids.contains(&ast_rule));
    let kinds = serde_json::from_value(json!({
        "completion": {
            "html_units":{"mode":"unit","units":["section"]},
            "fields":{"own_body":{"from":"section","text":"own"}}
        },
        "ast_completion": {
            "rule":"completion",
            "fields":{"heading":{"from":"first_line"}}
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &project, 6000);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn html_units_refuse_ast_neighbour_fields_with_kind_and_field_context() {
    for from in ["next", "previous"] {
        let kinds = serde_json::from_value(json!({
            "outline": {
                "html_units":{"mode":"unit","units":["section"]},
                "fields":{"neighbour":{"from":from,"rule":"completion"}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert!(
            errors.iter().any(|error| {
                error.field_path.to_string() == "evidence.kinds.outline.fields.neighbour"
                    && ["outline", "neighbour", from, "AST", "html_units"]
                        .iter()
                        .all(|expected| error.message.contains(expected))
            }),
            "{from}: {errors:?}"
        );
    }
}

#[test]
fn references_to_native_kinds_use_html_language_despite_ast_rule_name_collision() {
    let kinds = serde_json::from_value(json!({
        "completion": {
            "html_units":{"mode":"unit","units":["section"]},
            "fields":{}
        },
        "ast_completion": {
            "rule":"completion",
            "fields":{"parent":{"from":"enclosing","kind":"completion"}}
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].field_path.to_string(),
        "evidence.kinds.ast_completion.fields.parent"
    );
    assert!(
        errors[0].message.contains("another evidence language"),
        "{errors:?}"
    );
}
