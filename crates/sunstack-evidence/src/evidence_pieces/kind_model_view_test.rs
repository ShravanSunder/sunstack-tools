#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::{
    ConfigPath, InlineRule, LoadTarget, RepositoryId, SourceRef, SourceSpan, load_evidence_project,
};
use serde_json::json;
use std::path::PathBuf;

fn project() -> EvidenceProject {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sunstack/tests/fixtures/rulebook-lint/evidence/sgconfig.yml");
    let rules = [
        ("html_kind", "Html", "element"),
        ("json_kind", "Json", "pair"),
    ]
    .into_iter()
    .map(|(name, language, node)| InlineRule {
        kind: EvidenceKindName::new(name).unwrap(),
        text: format!("language: {language}\nrule: {{kind: {node}}}\n"),
        origin: ConfigPath::keys(["test", name]),
    })
    .collect::<Vec<_>>();
    load_evidence_project(&root, &rules, None).unwrap()
}

fn field_name(name: &str) -> FieldName {
    FieldName::new(name).unwrap()
}

fn text_field(value: &str) -> FieldValue {
    FieldValue::Text {
        value: value.into(),
        spans: vec![SourceSpan {
            source: SourceRef::Document {
                repository: RepositoryId::new("fixture").unwrap(),
                path: "page.html".into(),
                content_identity: "fixture".into(),
            },
            byte_start: 0,
            byte_end: value.len(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: value.len() + 1,
        }],
    }
}

#[test]
fn native_and_ast_html_kinds_project_before_budgeting_and_preserve_raw_fields() {
    let project = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let raw = format!(
        r#"<h2 data-unused="{}">Heading</h2><p>Body</p><img alt="Flow"><ex-mermaid label="Steps"></ex-mermaid>"#,
        "x".repeat(1200)
    );
    let fields = BTreeMap::from([(field_name("body"), text_field(&raw))]);
    let original = fields.clone();
    for matcher in [
        json!({"html_units":{"mode":"unit","units":["section"]}}),
        json!({"rule":"html_kind"}),
        json!({"inline_rule":"language: Html\nrule: {kind: element}\n"}),
    ] {
        let mut wire = matcher;
        wire["fields"] = json!({"body":{"from":"first_line"}});
        let definition = serde_json::from_value(wire).unwrap();
        assert_eq!(
            evidence_kind_language(&project, &kind, &definition),
            Some(EvidenceLanguage::Html)
        );
        let view = project_kind_model_view(&project, &kind, &definition, &fields, 80);
        let value = serde_json::to_value(view).unwrap();
        assert_eq!(value["state"], "bounded");
        assert_eq!(
            value["fields"]["body"],
            "Heading Body [alt: Flow] [label: Steps]"
        );
        assert_eq!(value["cut"], json!([]));
        assert_eq!(fields, original);
    }
}

#[test]
fn non_html_and_unknown_kinds_preserve_literal_markup() {
    let project = project();
    let kind = EvidenceKindName::new("example").unwrap();
    let raw = "<h2>Literal</h2><p>syntax</p>";
    let fields = BTreeMap::from([(field_name("body"), text_field(raw))]);
    for rule in ["completion", "json_kind", "not_a_rule"] {
        let definition =
            serde_json::from_value(json!({"rule":rule,"fields":{"body":{"from":"first_line"}}}))
                .unwrap();
        let view = project_kind_model_view(&project, &kind, &definition, &fields, 1000);
        assert_eq!(
            serde_json::to_value(view).unwrap()["fields"]["body"],
            raw,
            "{rule}"
        );
    }
}

#[test]
fn referenced_document_languages_and_target_paths_own_their_projection() {
    let project = project();
    let kind = EvidenceKindName::new("example").unwrap();
    let definition = serde_json::from_value(json!({
        "rule":"completion", "fields":{
            "html_document":{"from":"document","path":"page.html","rule":"html_kind"},
            "markdown_document":{"from":"document","path":"page.md","rule":"completion"},
            "targets":{"from":"target","regex":"(x)","resolve":"path"}
        }
    }))
    .unwrap();
    let raw = "<p>One</p><p>Two</p>";
    let targets = ["page.html", "page.md", "page.xhtml"]
        .into_iter()
        .map(|path| LoadTarget {
            name: path.into(),
            path: path.into(),
            exists: true,
            anchor: None,
            candidates: vec![],
            opening: Some(raw.into()),
            fields: Some(BTreeMap::from([(
                field_name("nested"),
                FieldValue::Texts {
                    values: vec![r#"<img alt="Nested">"#.into()],
                    spans: vec![],
                },
            )])),
            opening_cut: false,
            opening_last_line: None,
        })
        .collect();
    let fields = BTreeMap::from([
        (field_name("html_document"), text_field(raw)),
        (field_name("markdown_document"), text_field(raw)),
        (
            field_name("targets"),
            FieldValue::Targets { values: targets },
        ),
    ]);
    let original = fields.clone();
    let view = serde_json::to_value(project_kind_model_view(
        &project,
        &kind,
        &definition,
        &fields,
        1000,
    ))
    .unwrap();
    assert_eq!(view["fields"]["html_document"], "One Two");
    assert_eq!(view["fields"]["markdown_document"], raw);
    assert_eq!(view["fields"]["targets"][0]["opening"], "One Two");
    assert_eq!(
        view["fields"]["targets"][0]["fields"]["nested"],
        json!(["[alt: Nested]"])
    );
    assert_eq!(view["fields"]["targets"][1]["opening"], raw);
    assert_eq!(
        view["fields"]["targets"][1]["fields"]["nested"],
        json!([r#"<img alt="Nested">"#])
    );
    assert_eq!(view["fields"]["targets"][2]["opening"], "One Two");
    assert_eq!(
        view["fields"]["targets"][2]["fields"]["nested"],
        json!(["[alt: Nested]"])
    );
    assert_eq!(fields, original);
}

#[test]
fn pending_fields_stay_pending_and_projected_budget_cuts_name_the_field() {
    let project = project();
    let kind = EvidenceKindName::new("outline").unwrap();
    let definition = serde_json::from_value(
        json!({"html_units":{"mode":"unit","units":["section"]},"fields":{}}),
    )
    .unwrap();
    let fields = BTreeMap::from([(
        field_name("body"),
        FieldValue::Unavailable {
            reason: crate::FieldUnavailable::SelectionPending,
        },
    )]);
    assert_eq!(
        project_kind_model_view(&project, &kind, &definition, &fields, 200),
        ModelView::SelectionPending
    );
    let raw = format!("<p>{}</p>", "x".repeat(400));
    let fields = BTreeMap::from([(field_name("body"), text_field(&raw))]);
    let view = serde_json::to_value(project_kind_model_view(
        &project,
        &kind,
        &definition,
        &fields,
        200,
    ))
    .unwrap();
    assert_eq!(view["cut"], json!(["body"]));
    let projected = view["fields"]["body"].as_str().unwrap();
    assert_eq!(projected.chars().count(), 200);
    assert!(projected.contains("[cut]"));
    assert!(!projected.contains('<'));
    assert_eq!(fields[&field_name("body")], text_field(&raw));
}
