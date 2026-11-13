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

#[test]
fn native_html_sections_admit_own_and_enclosing_heading_fields() {
    let kinds = serde_json::from_value(json!({
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
        "candidate":{
            "html_units":{"mode":"unit","units":["section"]},
            "fields":{
                "heading":{"from":"section","text":"heading"},
                "heading_path":{"from":"enclosing","kind":"outline","text":"heading"}
            }
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn enclosing_heading_refuses_ast_kind_and_rule_references_at_the_field() {
    for reference in [json!({"kind":"ast_outline"}), json!({"rule":"completion"})] {
        let mut heading_path = reference;
        heading_path["from"] = json!("enclosing");
        heading_path["text"] = json!("heading");
        let kinds = serde_json::from_value(json!({
            "ast_outline":{"rule":"completion","fields":{}},
            "candidate":{"rule":"completion","fields":{"heading_path":heading_path}}
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(
            errors[0].field_path.to_string(),
            "evidence.kinds.candidate.fields.heading_path"
        );
        assert_eq!(
            errors[0].message,
            "kind candidate field heading_path: enclosing heading requires a native section kind (html_units or markdown_units with unit mode and units exactly [section])"
        );
    }
}

#[test]
fn same_file_heading_is_refused_while_legacy_context_text_stays_admitted() {
    for text in ["first_line", "full", "heading"] {
        let kinds = serde_json::from_value(json!({
            "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
            "candidate":{
                "html_units":{"mode":"unit","units":["section"]},
                "fields":{"context":{"from":"same_file","kind":"outline","text":text}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        if text == "heading" {
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert_eq!(
                errors[0].field_path.to_string(),
                "evidence.kinds.candidate.fields.context"
            );
            assert_eq!(
                errors[0].message,
                "kind candidate field context: same_file does not support heading text"
            );
        } else {
            assert!(errors.is_empty(), "{text}: {errors:?}");
        }
    }
}

#[test]
fn target_as_kind_refuses_heading_fields_and_preserves_legacy_fields() {
    for heading_source in [
        json!({"from":"section","text":"heading"}),
        json!({"from":"enclosing","kind":"outline","text":"heading"}),
        json!({"from":"section","text":"own"}),
    ] {
        let has_heading = heading_source["text"] == "heading";
        let kinds = serde_json::from_value(json!({
            "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
            "nested":{
                "html_units":{"mode":"unit","units":["section"]},
                "fields":{
                    "legacy":{"from":"section","text":"own"},
                    "context":heading_source
                }
            },
            "candidate":{
                "html_units":{"mode":"unit","units":["section"]},
                "fields":{"linked":{"from":"target","regex":"(.+)","resolve":"path","as_kind":"nested"}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        if has_heading {
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert_eq!(
                errors[0].field_path.to_string(),
                "evidence.kinds.candidate.fields.linked"
            );
            assert_eq!(errors[0].message, "as_kind must name a heading-free kind");
        } else {
            assert!(errors.is_empty(), "{errors:?}");
        }
    }
}
