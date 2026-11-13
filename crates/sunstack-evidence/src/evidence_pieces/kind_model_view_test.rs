#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::{
    ConfigPath, InlineRule, LoadTarget, RepositoryId, SourceRef, SourceSpan, load_evidence_project,
};
use serde_json::json;
use std::path::PathBuf;

fn project() -> EvidenceProject {
    project_with_settings(&Default::default())
}

fn project_with_settings(settings: &crate::DocumentEvidenceSettings) -> EvidenceProject {
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
    load_evidence_project(&root, &rules, None, settings).unwrap()
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
    let settings = serde_json::from_value(json!({"html":{
        "text_attributes":{"ex-mermaid":["label"]}
    }}))
    .unwrap();
    let project = project_with_settings(&settings);
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
        json!([
            "Whole",
            "Before [cut: html_projection_unrecoverable_markup]"
        ])
    );
    assert_eq!(view["fields"]["targets"][0]["opening"], "Opening");
    assert_eq!(
        view["fields"]["targets"][0]["fields"]["nested"],
        "Before [cut: html_projection_unrecoverable_markup]"
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
        "Before [cut: html_projection_unrecoverable_markup]"
    );
    assert_eq!(fields, original);
}

#[test]
fn asking_names_ambiguous_markup_instead_of_silent_completion() {
    for raw in [
        "Before</h2>After",
        "Before</h2><![CDATA[Visible > literal]]>After",
        "Before</h2><script/>Hidden &amp; More</script>After",
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["cut"], json!(["body"]), "{raw}");
        assert!(
            view["fields"]["body"]
                .as_str()
                .unwrap()
                .ends_with("[cut: html_projection_unrecoverable_markup]")
        );
    }
}

#[test]
fn asking_keeps_real_siblings_after_an_unrecoverable_error_span() {
    let view =
        html_asking_view("Before</h2><style/><![CDATA[Hidden</style><p>After</p><!-- ]]> -->Tail");
    assert_eq!(view["cut"], json!(["body"]));
    assert_eq!(
        view["fields"]["body"],
        "Before After Tail [cut: html_projection_unrecoverable_markup]"
    );
}

#[test]
fn asking_marks_all_ambiguous_review_probes_incomplete_by_name() {
    // C10-C12 replace lexical guessing: these mixed-markup errors must never be silent complete.
    for (name, raw) in [
        (
            r###"svg-text-complete"###,
            r###"<svg><text><![CDATA[Visible > literal]]></text></svg>"###,
        ),
        (
            r###"svg-text-unterminated"###,
            r###"<svg><text><![CDATA[Visible > literal</text></svg>"###,
        ),
        (
            r###"stray-complete"###,
            r###"Before</h2><![CDATA[Visible > literal]]>After"###,
        ),
        (
            r###"stray-unterminated"###,
            r###"Before</h2><![CDATA[Visible > literal After"###,
        ),
        (
            r###"svg-without-gt"###,
            r###"<svg><text><![CDATA[Visible literal]]></text></svg>"###,
        ),
        (
            r###"script-nonvoid-slash"###,
            r###"Before</h2><script/>Hidden</script>After"###,
        ),
        (
            r###"style-nonvoid-slash"###,
            r###"Before</h2><style/>Hidden</style>After"###,
        ),
        (
            r###"literal-cdata"###,
            r###"Before</h2><![CDATA[A &amp; <b>literal</b>]]>After"###,
        ),
        (
            r###"unicode-offset"###,
            r###"猫 Café</h2><![CDATA[文 &amp; <b>literal</b>]]>Après &amp; tail"###,
        ),
        (
            r###"two-blocks"###,
            r###"Before</h2><![CDATA[First &amp; <b>x</b>]]>Middle<![CDATA[Second > block]]>After"###,
        ),
        (r###"empty-block"###, r###"Before</h2><![CDATA[]]>After"###),
        (
            r###"suffix-attribute"###,
            r###"Before</h2><![CDATA[A &amp; <b>literal</b>]]><span aria-label="Suffix &amp; label">After</span>Tail"###,
        ),
        (
            r###"suffix-inside-element"###,
            r###"Before</h2><![CDATA[A <b>inside]]>Outside</b>Tail"###,
        ),
        (
            r###"suffix-script-looking"###,
            r###"Before</h2><![CDATA[A <script>inside]]>Outside</script>Tail"###,
        ),
        (
            r###"template-slash"###,
            r###"<h2>Before<h3><template/>Hidden</template>After"###,
        ),
        (
            r###"island-slash"###,
            r###"<h2>Before<h3><ex-markdown/>Hidden</ex-markdown>After"###,
        ),
        (
            r###"cdata-script-literal"###,
            r###"Before</h2><![CDATA[<script/>Literal</script>]]>After"###,
        ),
        (
            r###"script-entity"###,
            r###"Before</h2><script/>Hidden &amp; More</script><p>After</p>"###,
        ),
        (
            r###"script-cdata"###,
            r###"Before</h2><script/><![CDATA[Literal &amp; text]]>Hidden</script><p>After</p>"###,
        ),
        (
            r###"script-unsafe"###,
            r###"Before</h2><script/><![CDATA[Hidden</script><p>After</p>"###,
        ),
        (
            r###"script-cross-close-cdata"###,
            r###"Before</h2><script/><![CDATA[Hidden</script><p>After</p><!-- ]]> -->Tail"###,
        ),
        (
            r###"script-cross-close-comment"###,
            r###"Before</h2><script/><!--Hidden</script><p>After</p><!-- -->Tail"###,
        ),
        (
            r###"style-entity"###,
            r###"Before</h2><style/>Hidden &amp; More</style><p>After</p>"###,
        ),
        (
            r###"style-cdata"###,
            r###"Before</h2><style/><![CDATA[Literal &amp; text]]>Hidden</style><p>After</p>"###,
        ),
        (
            r###"style-unsafe"###,
            r###"Before</h2><style/><![CDATA[Hidden</style><p>After</p>"###,
        ),
        (
            r###"style-cross-close-cdata"###,
            r###"Before</h2><style/><![CDATA[Hidden</style><p>After</p><!-- ]]> -->Tail"###,
        ),
        (
            r###"style-cross-close-comment"###,
            r###"Before</h2><style/><!--Hidden</style><p>After</p><!-- -->Tail"###,
        ),
        (
            r###"template-entity"###,
            r###"<h2>Before<h3><template/>Hidden &amp; More</template><p>After</p>"###,
        ),
        (
            r###"template-cdata"###,
            r###"<h2>Before<h3><template/><![CDATA[Literal &amp; text]]>Hidden</template><p>After</p>"###,
        ),
        (
            r###"template-unsafe"###,
            r###"<h2>Before<h3><template/><![CDATA[Hidden</template><p>After</p>"###,
        ),
        (
            r###"template-cross-close-cdata"###,
            r###"<h2>Before<h3><template/><![CDATA[Hidden</template><p>After</p><!-- ]]> -->Tail"###,
        ),
        (
            r###"template-cross-close-comment"###,
            r###"<h2>Before<h3><template/><!--Hidden</template><p>After</p><!-- -->Tail"###,
        ),
        (
            r###"ex-markdown-entity"###,
            r###"<h2>Before<h3><ex-markdown/>Hidden &amp; More</ex-markdown><p>After</p>"###,
        ),
        (
            r###"ex-markdown-cdata"###,
            r###"<h2>Before<h3><ex-markdown/><![CDATA[Literal &amp; text]]>Hidden</ex-markdown><p>After</p>"###,
        ),
        (
            r###"ex-markdown-unsafe"###,
            r###"<h2>Before<h3><ex-markdown/><![CDATA[Hidden</ex-markdown><p>After</p>"###,
        ),
        (
            r###"ex-markdown-cross-close-cdata"###,
            r###"<h2>Before<h3><ex-markdown/><![CDATA[Hidden</ex-markdown><p>After</p><!-- ]]> -->Tail"###,
        ),
        (
            r###"ex-markdown-cross-close-comment"###,
            r###"<h2>Before<h3><ex-markdown/><!--Hidden</ex-markdown><p>After</p><!-- -->Tail"###,
        ),
        (r###"stray"###, r###"Before</h2>After"###),
        (r###"nested"###, r###"Before</h2>After</h3>Tail"###),
        (
            r###"unterminated"###,
            r###"Before <broken attribute="unterminated"###,
        ),
        (r###"bare-open"###, r###"Before <"###),
        (r###"missing-end"###, r###"Before <broken"###),
    ] {
        let view = html_asking_view(raw);
        assert_eq!(view["cut"], json!(["body"]), "{name}");
        assert!(
            view["fields"]["body"]
                .as_str()
                .unwrap()
                .ends_with("[cut: html_projection_unrecoverable_markup]"),
            "{name}"
        );
    }
}

#[test]
fn asking_projects_plain_error_text_without_a_cut() {
    let view = html_asking_view("Before > literal &amp; 文");
    assert_eq!(view["fields"]["body"], "Before > literal & 文");
    assert_eq!(view["cut"], json!([]));
}

#[test]
fn asking_separates_structural_error_tags_without_a_cut() {
    let view = html_asking_view("<p>Before</p></section><p>After</p>");
    assert_eq!(view["fields"]["body"], "Before After");
    assert_eq!(view["cut"], json!([]));
}

#[path = "kind_model_view_test/project_settings.rs"]
mod project_settings;
