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

fn html_asking_view(raw: &str) -> serde_json::Value {
    let project = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let definition = serde_json::from_value(json!({
        "rule":"html_kind", "fields":{"body":{"from":"first_line"}}
    }))
    .unwrap();
    let fields = BTreeMap::from([(field_name("body"), text_field(raw))]);
    let original = fields.clone();
    let view = project_kind_model_view(&project, &kind, &definition, &fields, 1000);
    assert_eq!(
        fields, original,
        "asking recovery must preserve raw fields and spans"
    );
    serde_json::to_value(view).unwrap()
}

#[test]
fn asking_retains_words_after_stray_closing_tag() {
    // Arrange: the parser stores the closing tag and following word in ERROR nodes.
    let raw = "Before</h2>After";
    // Act: use the same projection boundary as rule, selection and filter requests.
    let view = html_asking_view(raw);
    // Assert: the sentence survives, without exposing the malformed tag as prose.
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_retains_words_after_nested_stray_tags() {
    let view = html_asking_view("Before</h2>After</h3>Tail");
    assert_eq!(view["fields"]["body"], "Before After Tail");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_decodes_entities_in_recovered_words_once() {
    let view = html_asking_view("Before</h2>Après &amp; &#x732B; &amp;lt;");
    assert_eq!(view["fields"]["body"], "Before Après & 猫 &lt;");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_names_unterminated_markup_and_keeps_the_safe_prefix() {
    let view = html_asking_view("Before <broken attribute=\"unterminated");
    assert_eq!(
        view["fields"]["body"],
        "Before [cut: html_projection_unterminated_markup]"
    );
    assert_eq!(view["cut"], json!(["body"]));
}

#[test]
fn asking_does_not_recover_script_or_style_error_payload_as_prose() {
    for raw in [
        "Before<script>Hidden</h2>After",
        "Before<style>Hidden</h2>After",
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["fields"]["body"], "Before", "{raw}");
        assert_eq!(view["cut"], json!([]));
    }
}

#[test]
fn asking_skips_recovered_comments_and_keeps_following_words() {
    let view = html_asking_view("Before</h2><!-- Hidden -->After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_marks_missing_tag_terminators_incomplete() {
    for raw in ["Before <", "Before <broken"] {
        let view = html_asking_view(raw);
        assert_eq!(
            view["fields"]["body"], "Before [cut: html_projection_unterminated_markup]",
            "{raw}"
        );
        assert_eq!(view["cut"], json!(["body"]));
    }
}

#[test]
fn asking_keeps_template_and_markdown_islands_inside_error_spans_excluded() {
    for raw in [
        "Before<template>Hidden</h2>After<",
        "Before<ex-markdown>Hidden</h2>After<",
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["fields"]["body"], "Before", "{raw}");
        assert_eq!(view["cut"], json!([]));
    }
}

#[test]
fn asking_keeps_well_formed_script_style_attributes_and_text_unchanged() {
    let view = html_asking_view(
        r#"<p aria-label="Flow &amp; words">Café <b>猫</b> **literal**</p><script>x = '&amp;';</script><style>a > b { color: red }</style>"#,
    );
    assert_eq!(
        view["fields"]["body"],
        "[aria-label: Flow & words] Café 猫 **literal** x = '&amp;'; a > b { color: red }"
    );
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_cuts_nested_target_and_list_fields_without_mutating_raw_evidence() {
    let project = project();
    let kind = EvidenceKindName::new("html_kind").unwrap();
    let definition = serde_json::from_value(json!({
        "rule":"html_kind", "fields":{"list":{"from":"same_file","kind":"html_kind"},
        "targets":{"from":"target","regex":"(x)","resolve":"path"}}
    }))
    .unwrap();
    let raw = "Before <broken";
    let fields = BTreeMap::from([
        (
            field_name("list"),
            FieldValue::Texts {
                values: vec!["<p>Whole</p>".into(), raw.into()],
                spans: vec![],
            },
        ),
        (
            field_name("targets"),
            FieldValue::Targets {
                values: vec![LoadTarget {
                    name: "page".into(),
                    path: "page.html".into(),
                    exists: true,
                    anchor: None,
                    candidates: vec![],
                    opening: Some("<p>Opening</p>".into()),
                    opening_cut: false,
                    opening_last_line: None,
                    fields: Some(BTreeMap::from([(field_name("nested"), text_field(raw))])),
                }],
            },
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
    assert_eq!(view["cut"], json!(["list", "targets"]));
    assert_eq!(
        view["fields"]["list"],
        json!(["Whole", "Before [cut: html_projection_unterminated_markup]"])
    );
    assert_eq!(view["fields"]["targets"][0]["opening"], "Opening");
    assert_eq!(
        view["fields"]["targets"][0]["fields"]["nested"],
        "Before [cut: html_projection_unterminated_markup]"
    );
    assert_eq!(fields, original);
}

#[test]
fn asking_cuts_a_target_with_only_an_unterminated_opening() {
    let project = project();
    let kind = EvidenceKindName::new("json_kind").unwrap();
    let definition = serde_json::from_value(json!({"rule":"json_kind","fields":{
        "targets":{"from":"target","regex":"(x)","resolve":"path"}
    }}))
    .unwrap();
    let raw = "Before <broken";
    let fields = BTreeMap::from([(
        field_name("targets"),
        FieldValue::Targets {
            values: vec![LoadTarget {
                name: "page".into(),
                path: "page.xhtml".into(),
                exists: true,
                anchor: None,
                candidates: vec![],
                opening: Some(raw.into()),
                fields: None,
                opening_cut: false,
                opening_last_line: None,
            }],
        },
    )]);
    let original = fields.clone();
    let view = serde_json::to_value(project_kind_model_view(
        &project,
        &kind,
        &definition,
        &fields,
        1000,
    ))
    .unwrap();
    assert_eq!(view["cut"], json!(["targets"]));
    assert_eq!(
        view["fields"]["targets"][0]["opening"],
        "Before [cut: html_projection_unterminated_markup]"
    );
    assert_eq!(fields, original);
}

#[test]
fn asking_preserves_selected_attributes_on_recognized_tags_inside_errors() {
    let view = html_asking_view(r#"<h2 aria-label="Heading &amp; words">Parent<h3>Child"#);
    assert_eq!(
        view["fields"]["body"],
        "[aria-label: Heading & words] Parent Child"
    );
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_preserves_complete_cdata_in_recovered_spans() {
    for (raw, expected) in [
        (
            "<svg><text><![CDATA[Visible > literal]]></text></svg>",
            "Visible > literal",
        ),
        (
            "<svg><text><![CDATA[Visible literal]]></text></svg>",
            "Visible literal",
        ),
        (
            "Before</h2><![CDATA[Visible > literal]]>After",
            "Before Visible > literal After",
        ),
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["fields"]["body"], expected, "{raw}");
        assert_eq!(view["cut"], json!([]));
    }
}

#[test]
fn asking_keeps_cdata_entities_and_tag_like_text_literal() {
    let view = html_asking_view("Before</h2><![CDATA[A &amp; <b>literal</b>]]>After");
    assert_eq!(
        view["fields"]["body"],
        "Before A &amp; <b>literal</b> After"
    );
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_marks_unterminated_cdata_incomplete() {
    for (raw, expected) in [
        (
            "<svg><text><![CDATA[Visible > literal</text></svg>",
            "[cut: html_projection_unterminated_markup]",
        ),
        (
            "Before</h2><![CDATA[Visible > literal After",
            "Before [cut: html_projection_unterminated_markup]",
        ),
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["fields"]["body"], expected, "{raw}");
        assert_eq!(view["cut"], json!(["body"]));
    }
}

#[test]
fn asking_excludes_script_slash_payload_in_recovered_spans() {
    let view = html_asking_view("Before</h2><script/>Hidden</script>After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_excludes_style_slash_payload_in_recovered_spans() {
    let view = html_asking_view("Before</h2><style/>Hidden</style>After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_excludes_template_slash_payload_in_recovered_spans() {
    let raw = "<h2>Before<h3><template/>Hidden</template>After";
    assert_recovery_span_contains(raw, "<template/>");
    let view = html_asking_view(raw);
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_excludes_markdown_island_slash_payload_in_recovered_spans() {
    let raw = "<h2>Before<h3><ex-markdown/>Hidden</ex-markdown>After";
    assert_recovery_span_contains(raw, "<ex-markdown/>");
    let view = html_asking_view(raw);
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

fn assert_recovery_span_contains(raw: &str, token: &str) {
    use ast_grep_language::{LanguageExt, SupportLang};
    let tree = SupportLang::Html.ast_grep(raw);
    assert!(
        tree.root()
            .dfs()
            .any(|node| node.is_error() && node.text().contains(token)),
        "fixture must put {token} inside an ERROR recovery span"
    );
}

#[test]
fn asking_ignores_unterminated_cdata_inside_recovered_script() {
    let view = html_asking_view("Before</h2><script/><![CDATA[Unterminated");
    assert_eq!(view["fields"]["body"], "Before");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_ignores_unterminated_cdata_inside_recovered_style() {
    let view = html_asking_view("Before</h2><style/><![CDATA[Unterminated");
    assert_eq!(view["fields"]["body"], "Before");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_keeps_script_exclusion_across_entity_nodes() {
    let view = html_asking_view("Before</h2><script/>Hidden &amp; More</script>After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_keeps_style_exclusion_across_entity_nodes() {
    let view = html_asking_view("Before</h2><style/>Hidden &amp; More</style>After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_keeps_script_exclusion_across_cdata_and_following_nodes() {
    let view =
        html_asking_view("Before</h2><script/><![CDATA[Literal &amp; text]]>Hidden</script>After");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_continues_after_unterminated_cdata_in_an_excluded_block() {
    for raw in [
        "Before</h2><script/><![CDATA[Hidden</script><p>After</p>",
        "Before</h2><style/><![CDATA[Hidden</style><p>After</p>",
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["fields"]["body"], "Before After", "{raw}");
        assert_eq!(view["cut"], json!([]));
    }
}
