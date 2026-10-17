#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_evidence::*;

fn assert_legacy_paragraph(markdown: &str, expected: &str) {
    let document = CapturedDocument {
        interval: CaptureInterval {
            started_at: Default::default(),
            completed_at: Default::default(),
        },
        source: SourceRef::Document {
            repository: RepositoryId::new("legacy_markdown").unwrap(),
            path: "legacy.md".into(),
            content_identity: content_digest(markdown.as_bytes()),
        },
        text: markdown.into(),
    };
    let SelectionContent::Paragraph { text, span } =
        resolve_paragraph(&document, &TargetId::new("U001").unwrap()).unwrap()
    else {
        panic!("expected a legacy paragraph")
    };
    // Literals observed at 9a83303: legacy Parser::new has no table/metadata extensions.
    assert_eq!(text, expected);
    let start = markdown.find(expected).unwrap();
    assert_eq!(
        (span.byte_start, span.byte_end),
        (start, start + expected.len())
    );
    assert_eq!(&markdown[span.byte_start..span.byte_end], expected);
}

#[test]
fn table_shaped_legacy_target_keeps_the_complete_raw_paragraph() {
    assert_legacy_paragraph(
        "U001: heading | Other\n--- | ---\nvalue | value\n",
        "U001: heading | Other\n--- | ---\nvalue | value",
    );
}

#[test]
fn frontmatter_shaped_legacy_target_remains_an_eligible_raw_paragraph() {
    assert_legacy_paragraph("---\n\nU001: inside\n\nother: yes\n---\n", "U001: inside");
}
