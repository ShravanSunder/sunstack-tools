use super::*;
use serde_json::json;

#[test]
fn embedded_markdown_kinds_and_units_keep_original_html_offsets_and_source_order() {
    // Arrange: caller conventions choose the island, independently of its wrapper name.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/blocks.yml"), "id: html\nlanguage: Html\nrule: {kind: element, regex: '^<p>'}\n---\nid: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n---\nid: list\nlanguage: Markdown\nrule: {kind: list}\n---\nid: table\nlanguage: Markdown\nrule: {kind: pipe_table}\n---\nid: diagram\nlanguage: Markdown\nrule: {kind: fenced_code_block}\n").unwrap();
    let text = include_str!("../../fixtures/embedded-markdown.html");
    std::fs::write(root.path().join("page.html"), text).unwrap();
    let settings = serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
    let project =
        load_evidence_project(&root.path().join("sgconfig.yml"), &[], None, &settings).unwrap();
    let mut kinds = ["html", "paragraph", "list", "table", "diagram"]
        .into_iter()
        .map(|name| {
            (
                EvidenceKindName::new(name).unwrap(),
                serde_json::from_value(json!({"rule":name,"fields":{"body":{"from":"remainder"}}}))
                    .unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    kinds.insert(
        EvidenceKindName::new("unit").unwrap(),
        serde_json::from_value(
            json!({"markdown_units":{"mode":"unit","units":["paragraph","list_item"]},"fields":{}}),
        )
        .unwrap(),
    );

    // Act: real acquisition, HTML selection, Markdown parsing and piece construction.
    let catalog = build_pieces(
        &project,
        &kinds,
        &["page.html".into()],
        &mut RecordingSource::new(root.path()),
        limits(),
    );

    // Assert: source order crosses languages, and every byte still belongs to page.html.
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let blocks = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() != "unit")
        .collect::<Vec<_>>();
    assert_eq!(
        blocks
            .iter()
            .map(|piece| piece.kind.as_str())
            .collect::<Vec<_>>(),
        [
            "html",
            "paragraph",
            "list",
            "paragraph",
            "paragraph",
            "table",
            "diagram",
            "html"
        ]
    );
    for piece in &catalog.pieces {
        let span = &piece.location.span;
        assert_eq!(piece.text, text[span.byte_start..span.byte_end]);
        assert!(
            matches!(&span.source, SourceRef::Document {path, content_identity, ..} if path == Path::new("page.html") && content_identity == &content_digest(text.as_bytes()))
        );
        assert_eq!(
            span.start_line,
            text[..span.byte_start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1
        );
    }
    let paragraph = blocks[1];
    assert_eq!(
        paragraph.location.span.byte_start,
        text.find("Paragraph café.").unwrap()
    );
    assert_eq!(paragraph.text, "Paragraph café.\n");
    let table = blocks[5];
    assert_eq!(table.location.span.byte_start, text.find("| Name").unwrap());
    assert_eq!(
        table.location.span.byte_end,
        text.find("\n\n```mermaid").unwrap() + 1
    );
    let units = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "unit")
        .collect::<Vec<_>>();
    assert_eq!(units.len(), 5);
    assert!(units.iter().any(|piece| piece.text.starts_with("- First")));
}

#[test]
fn embedded_markdown_siblings_and_section_fields_stay_in_their_island() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/paragraph.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let text = "文<script type='text/markdown'>## First\n\nOne.\n\nTwo.\n</script><p>Outside</p><script type='text/markdown'>## Second\n\nThree.\n</script>";
    std::fs::write(root.path().join("page.html"), text).unwrap();
    let settings = serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
    let project =
        load_evidence_project(&root.path().join("sgconfig.yml"), &[], None, &settings).unwrap();
    let kinds = serde_json::from_value(json!({
        "paragraph":{"rule":"paragraph","fields":{"next":{"from":"next","rule":"paragraph"},"heading":{"from":"enclosing","kind":"outline","text":"heading"}}},
        "outline":{"markdown_units":{"mode":"unit","units":["section"]},"fields":{"heading":{"from":"section","text":"heading"},"own":{"from":"section","text":"own"}}}
    })).unwrap();
    let catalog = build_pieces(
        &project,
        &kinds,
        &["page.html".into()],
        &mut RecordingSource::new(root.path()),
        limits(),
    );
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let paragraphs = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "paragraph")
        .collect::<Vec<_>>();
    assert_eq!(paragraphs.len(), 3);
    assert!(
        matches!(field(paragraphs[0], "next"), FieldValue::Texts {values, spans} if values == &["Two.\n"] && spans[0].byte_start == text.find("Two.").unwrap())
    );
    assert!(
        matches!(field(paragraphs[1], "next"), FieldValue::Texts {values, ..} if values.is_empty())
    );
    assert!(
        matches!(field(paragraphs[2], "heading"), FieldValue::Texts {values, ..} if values == &["Second"])
    );
    let outlines = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "outline")
        .collect::<Vec<_>>();
    assert_eq!(outlines.len(), 2);
    assert!(
        matches!(field(outlines[0], "heading"), FieldValue::Text {value, ..} if value == "First")
    );
    assert!(
        matches!(field(outlines[0], "own"), FieldValue::Text {value, ..} if value == "## First\n\nOne.\n\nTwo.\n")
    );
    let limited = build_pieces(
        &project,
        &kinds,
        &["page.html".into()],
        &mut RecordingSource::new(root.path()),
        PieceBuildLimits {
            max_matches: 1,
            ..limits()
        },
    );
    assert!(
        limited
            .failures
            .iter()
            .any(|failure| failure.reason == "max_matches")
    );
    assert_eq!(
        limited.matches[Path::new("page.html")]
            .by_rule
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        1
    );
}
