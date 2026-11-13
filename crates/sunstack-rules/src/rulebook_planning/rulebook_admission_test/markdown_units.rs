#![allow(clippy::unwrap_used)]
use super::super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use serde_json::json;

#[test]
fn markdown_units_admit_section_fields_without_an_ast_rule() {
    let kinds = serde_json::from_value(json!({
        "outline": {
            "markdown_units":{"mode":"unit","units":["section"]},
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
fn markdown_units_admit_all_unit_kinds_and_ordinary_markdown_fields() {
    for units in [
        json!(["section"]),
        json!(["paragraph"]),
        json!(["list_item"]),
        json!(["section", "paragraph", "list_item"]),
    ] {
        let kinds = serde_json::from_value(json!({
            "completion": {
                "markdown_units":{"mode":"unit","units":units},
                "fields": {
                    "heading":{"from":"first_line"},
                    "metadata":{"from":"yaml","key":"description"},
                    "context":{"from":"same_file","rule":"completion"}
                }
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert!(errors.is_empty(), "{units}: {errors:?}");
    }
}

#[test]
fn markdown_section_source_requires_exactly_one_section_unit() {
    for units in [
        json!(["paragraph"]),
        json!(["list_item"]),
        json!(["section", "paragraph"]),
        json!(["section", "section"]),
    ] {
        let kinds = serde_json::from_value(json!({
            "outline": {
                "markdown_units":{"mode":"unit","units":units},
                "fields":{"own_body":{"from":"section","text":"own"}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert_eq!(errors.len(), 1, "{units}: {errors:?}");
        assert_eq!(
            errors[0].field_path.to_string(),
            "evidence.kinds.outline.fields.own_body"
        );
        for expected in ["outline", "own_body", "section", "markdown_units"] {
            assert!(errors[0].message.contains(expected), "{errors:?}");
        }
    }
}

#[test]
fn markdown_units_refuse_call_site_at_named_admission_boundary() {
    let kinds = serde_json::from_value(json!({
        "outline": {
            "markdown_units":{"mode":"call_site","units":["paragraph"],"pattern":"Use"},
            "fields":{}
        }
    }))
    .unwrap();
    let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].field_path.to_string(),
        "evidence.kinds.outline.markdown_units.mode"
    );
    for expected in ["outline", "markdown_units", "unit", "call_site"] {
        assert!(errors[0].message.contains(expected), "{errors:?}");
    }
}

#[test]
fn markdown_units_refuse_empty_units_and_invalid_filters_at_admission() {
    for selection in [
        json!({"mode":"unit","units":[]}),
        json!({"mode":"unit","units":["section"],"starts_with":""}),
        json!({"mode":"unit","units":["section"],"pattern":"["}),
        json!({"mode":"unit","units":["section"],"pattern":"a*"}),
        json!({"mode":"unit","units":["section"],"pattern":"\\b"}),
    ] {
        let kinds = serde_json::from_value(json!({
            "outline":{"markdown_units":selection,"fields":{}}
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert_eq!(errors.len(), 1, "{selection}: {errors:?}");
        assert_eq!(
            errors[0].field_path.to_string(),
            "evidence.kinds.outline.markdown_units"
        );
        assert!(
            errors[0]
                .message
                .contains("invalid markdown_units selection"),
            "{errors:?}"
        );
    }
}

#[test]
fn markdown_units_refuse_ast_neighbour_fields_with_kind_and_field_context() {
    for from in ["next", "previous"] {
        let kinds = serde_json::from_value(json!({
            "outline": {
                "markdown_units":{"mode":"unit","units":["section"]},
                "fields":{"neighbour":{"from":from,"rule":"completion"}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(
            errors[0].field_path.to_string(),
            "evidence.kinds.outline.fields.neighbour"
        );
        for expected in ["outline", "neighbour", from, "AST", "markdown_units"] {
            assert!(errors[0].message.contains(expected), "{errors:?}");
        }
    }
}

#[test]
fn native_markdown_kind_references_use_markdown_language() {
    for from in ["inside", "enclosing", "same_file", "remainder"] {
        let source = if from == "remainder" {
            json!({"from":from,"exclude":{"kind":"outline"}})
        } else {
            json!({"from":from,"kind":"outline"})
        };
        for matcher in [
            json!({"rule":"completion"}),
            json!({"html_units":{"mode":"unit","units":["section"]}}),
        ] {
            let is_html = matcher.get("html_units").is_some();
            let mut referring = matcher;
            referring["fields"] = json!({"context":source});
            let kinds = serde_json::from_value(json!({
                "outline":{
                    "markdown_units":{"mode":"unit","units":["section"]},
                    "fields":{}
                },
                "referring":referring
            }))
            .unwrap();
            let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
            if is_html {
                assert_eq!(errors.len(), 1, "{from}: {errors:?}");
                assert_eq!(
                    errors[0].field_path.to_string(),
                    "evidence.kinds.referring.fields.context"
                );
                assert!(
                    errors[0].message.contains("another evidence language"),
                    "{errors:?}"
                );
            } else {
                assert!(errors.is_empty(), "{from}: {errors:?}");
            }
        }
    }
}

#[test]
fn native_markdown_sections_admit_own_and_enclosing_heading_fields() {
    let kinds = serde_json::from_value(json!({
        "outline":{"markdown_units":{"mode":"unit","units":["section"]},"fields":{}},
        "candidate":{
            "markdown_units":{"mode":"unit","units":["section"]},
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
fn enclosing_heading_refuses_non_section_markdown_kinds_at_the_field() {
    for selection in [
        json!({"mode":"unit","units":["paragraph"]}),
        json!({"mode":"unit","units":["section","paragraph"]}),
        json!({"mode":"call_site","units":["section"],"pattern":"Use"}),
    ] {
        let call_site = selection["mode"] == "call_site";
        let kinds = serde_json::from_value(json!({
            "outline":{"markdown_units":selection,"fields":{}},
            "candidate":{
                "markdown_units":{"mode":"unit","units":["section"]},
                "fields":{"heading_path":{"from":"enclosing","kind":"outline","text":"heading"}}
            }
        }))
        .unwrap();
        let errors = admit_evidence_kinds(&kinds, &fixture::project(), 6000);
        assert_eq!(errors.len(), if call_site { 2 } else { 1 }, "{errors:?}");
        let heading_error = errors
            .iter()
            .find(|error| {
                error.field_path.to_string() == "evidence.kinds.candidate.fields.heading_path"
            })
            .unwrap();
        assert_eq!(
            heading_error.message,
            "kind candidate field heading_path: enclosing heading requires a native section kind (html_units or markdown_units with unit mode and units exactly [section])"
        );
    }
}
