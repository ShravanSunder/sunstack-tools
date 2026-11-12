use super::*;
use serde_json::json;

fn setup() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/html.yml"), "id: row\nlanguage: Html\nrule: {kind: element}\n---\nid: doc\nlanguage: Html\nrule: {kind: document}\n").unwrap();
    root
}
fn build(
    root: &Path,
    files: &[&str],
    definition: serde_json::Value,
    cap: usize,
) -> EvidenceCatalog {
    let project = load_evidence_project(&root.join("sgconfig.yml"), &[], None).unwrap();
    let kinds = BTreeMap::from([(
        EvidenceKindName::new("row").unwrap(),
        serde_json::from_value(definition).unwrap(),
    )]);
    let mut source = FilesystemCheckedSource::new(root).unwrap();
    build_pieces(
        &project,
        &kinds,
        &files.iter().map(PathBuf::from).collect::<Vec<_>>(),
        &mut source,
        PieceBuildLimits {
            max_files: cap,
            ..limits()
        },
    )
}

#[test]
fn html_model_projection_strips_markup_but_preserves_raw_fields_and_spans() {
    let root = setup();
    let raw = "<p data-secret=\"attribute-only\">Café &amp; <b>Unicode 文</b>  body.</p>";
    std::fs::write(root.path().join("source.html"), raw).unwrap();
    let catalog = build(
        root.path(),
        &["source.html"],
        json!({"rule":"row","fields":{"body":{"from":"remainder"}}}),
        2,
    );
    let piece = catalog
        .pieces
        .iter()
        .find(|piece| piece.text == raw)
        .unwrap();
    assert_eq!(
        field(piece, "body"),
        &FieldValue::Text {
            value: raw.into(),
            spans: vec![piece.location.span.clone()]
        }
    );
    assert_eq!(piece.location.span.byte_start, 0);
    assert_eq!(piece.location.span.byte_end, raw.len());
    let ModelView::Bounded { fields, .. } = &piece.model_view else {
        panic!()
    };
    assert_eq!(
        fields[&FieldName::new("body").unwrap()],
        ModelFieldValue::Text("Café & Unicode 文 body.".into())
    );
}

#[test]
fn document_fields_read_matching_sibling_pieces_and_share_acquisition_budget() {
    let root = setup();
    std::fs::create_dir(root.path().join("pages")).unwrap();
    std::fs::write(root.path().join("pages/source.html"), "<p>R1</p>").unwrap();
    let target = "<p>R1 covered</p><p>R10 covered</p>";
    std::fs::write(root.path().join("pages/other.html"), target).unwrap();
    let definition = json!({"rule":"row","fields":{"coverage":{"from":"document","path":"other.html","rule":"row"},"again":{"from":"document","path":"./other.html","rule":"row"}}});
    let catalog = build(root.path(), &["pages/source.html"], definition.clone(), 2);
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let FieldValue::Texts { values, spans } = field(&catalog.pieces[0], "coverage") else {
        panic!()
    };
    assert_eq!(values, &["<p>R1 covered</p>", "<p>R10 covered</p>"]);
    assert_eq!(spans.len(), 2);
    assert!(spans.iter().all(|span|matches!(&span.source,SourceRef::Document{path,..} if path==Path::new("pages/other.html"))));
    assert_eq!(
        field(&catalog.pieces[0], "coverage"),
        field(&catalog.pieces[0], "again")
    );
    let limited = build(root.path(), &["pages/source.html"], definition, 1);
    assert!(matches!(
        field(&limited.pieces[0], "coverage"),
        FieldValue::Unavailable { .. }
    ));
    assert!(
        limited
            .failures
            .iter()
            .any(|failure| failure.reason == "max_files")
    );
    std::fs::remove_file(root.path().join("pages/other.html")).unwrap();
    let missing = build(
        root.path(),
        &["pages/source.html"],
        json!({"rule":"row","fields":{"coverage":{"from":"document","path":"other.html","rule":"row"}}}),
        2,
    );
    assert!(matches!(
        field(&missing.pieces[0], "coverage"),
        FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable
        }
    ));
}

#[test]
fn target_fragments_use_full_budgeted_document_not_opening_or_escaped_markup() {
    let root = setup();
    let text = "<p><a href=\"target.html#far\">x</a><a href=\"target.html#fake\">y</a><a href=\"#own\">z</a></p><p id=\"own\">own</p>";
    std::fs::write(root.path().join("source.html"), text).unwrap();
    std::fs::write(
        root.path().join("target.html"),
        format!(
            "{}&lt;p id=\"fake\"&gt;example&lt;/p&gt;<p id=\"far\">real</p>",
            " ".repeat(100)
        ),
    )
    .unwrap();
    let catalog = build(
        root.path(),
        &["source.html"],
        json!({"rule":"row","fields":{"links":{"from":"target","regex":"href=\"([^\"]+)\"","resolve":"path","opening_chars":5}}}),
        2,
    );
    let piece = catalog
        .pieces
        .iter()
        .find(|piece| piece.text.starts_with("<p><a"))
        .unwrap();
    let FieldValue::Targets { values } = field(piece, "links") else {
        panic!()
    };
    assert_eq!(values.len(), 3);
    assert!(values.iter().all(|target| target.exists));
    let values = serde_json::to_value(values).unwrap();
    assert_eq!(values[0]["path"], "target.html");
    assert_eq!(
        values[0]["anchor"],
        json!({"status":"found","fragment":"far"})
    );
    assert_eq!(
        values[1]["anchor"],
        json!({"status":"missing","fragment":"fake"})
    );
    assert_eq!(values[2]["path"], "source.html");
    assert_eq!(
        values[2]["anchor"],
        json!({"status":"found","fragment":"own"})
    );
}

#[test]
fn empty_target_capture_is_a_named_unavailable_input() {
    let root = setup();
    std::fs::write(
        root.path().join("source.html"),
        "<p><a href=\"\">empty</a></p>",
    )
    .unwrap();
    let catalog = build(
        root.path(),
        &["source.html"],
        json!({"rule":"row","fields":{"links":{"from":"target","regex":"href=\"([^\"]*)\"","resolve":"path"}}}),
        2,
    );
    let piece = catalog
        .pieces
        .iter()
        .find(|piece| piece.text.starts_with("<p>"))
        .unwrap();
    assert_eq!(
        serde_json::to_value(field(piece, "links")).unwrap(),
        json!({"state":"unavailable","reason":"empty_target"})
    );
}

#[test]
fn absolute_inside_targets_resolve_but_outside_targets_are_unavailable_without_reading() {
    let root = setup();
    let outside = tempfile::tempdir().unwrap();
    let inside_path = root.path().join("inside.html");
    std::fs::write(&inside_path, "<p id=\"yes\">inside</p>").unwrap();
    let outside_path = outside.path().join("private.html");
    std::fs::write(&outside_path, [255]).unwrap();
    for (target, expected) in [(&inside_path, "targets"), (&outside_path, "unavailable")] {
        std::fs::write(
            root.path().join("source.html"),
            format!("<p><a href=\"{}#yes\">link</a></p>", target.display()),
        )
        .unwrap();
        let catalog = build(
            root.path(),
            &["source.html"],
            json!({"rule":"row","fields":{"links":{"from":"target","regex":"href=\"([^\"]+)\"","resolve":"path"},"coverage":{"from":"document","path":target,"rule":"row"}}}),
            2,
        );
        let piece = catalog
            .pieces
            .iter()
            .find(|piece| piece.text.starts_with("<p>"))
            .unwrap();
        let link = serde_json::to_value(field(piece, "links")).unwrap();
        assert_eq!(link["state"], expected);
        if expected == "unavailable" {
            assert_eq!(link["reason"], "outside_root");
            assert_eq!(
                serde_json::to_value(field(piece, "coverage")).unwrap(),
                json!({"state":"unavailable","reason":"outside_root"})
            );
        } else {
            assert_eq!(link["values"][0]["path"], "inside.html");
            assert_eq!(link["values"][0]["anchor"]["status"], "found");
        }
    }
}

#[test]
fn document_rule_scopes_rows_to_the_named_containers_own_opening() {
    let root = setup();
    std::fs::write(root.path().join("source.html"), "<p>requirement</p>").unwrap();
    std::fs::write(root.path().join("target.html"),"<html><body><table id=\"scope\"><tr><td>R1 inside</td></tr></table><table id=\"other\"><tr><td>R99 decoy</td></tr></table></body></html>").unwrap();
    std::fs::write(root.path().join("rules/scoped.yml"),"id: scoped\nlanguage: Html\nrule:\n  kind: element\n  regex: '^<tr(?: |>)'\n  inside:\n    kind: element\n    regex: '^<table id=\"scope\"'\n    has:\n      kind: start_tag\n      regex: 'id=\"scope\"'\n    stopBy: end\n").unwrap();
    let catalog = build(
        root.path(),
        &["source.html"],
        json!({"rule":"row","fields":{"rows":{"from":"document","path":"target.html","rule":"scoped"}}}),
        2,
    );
    let FieldValue::Texts { values, .. } = field(&catalog.pieces[0], "rows") else {
        panic!()
    };
    assert_eq!(values, &["<tr><td>R1 inside</td></tr>"]);
    assert!(!values.iter().any(|value| value.contains("R99")));
}
