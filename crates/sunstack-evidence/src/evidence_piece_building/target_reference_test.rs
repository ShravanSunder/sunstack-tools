#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn markdown_urls_decode_utf8_paths_and_fragments_once_without_entities() {
    let reference = TargetReference::parse(
        "docs/a%20b.html#caf%C3%A9%2520&amp;x",
        TargetReferenceSyntax::MarkdownUrl,
    )
    .unwrap();
    assert_eq!(reference.file, PathBuf::from("docs/a b.html"));
    assert_eq!(reference.fragment.as_deref(), Some("café%20&amp;x"));
}

#[test]
fn markdown_urls_refuse_percent_encoded_non_utf8_bytes() {
    assert!(matches!(
        TargetReference::parse("file.md#%FF", TargetReferenceSyntax::MarkdownUrl),
        Err(FieldUnavailable::Unreadable)
    ));
}

#[test]
fn html_urls_keep_existing_entity_and_percent_decoding() {
    let reference = TargetReference::parse(
        "docs/a%20b.html#A&amp;B%20C",
        TargetReferenceSyntax::HtmlUrl,
    )
    .unwrap();
    assert_eq!(reference.file, PathBuf::from("docs/a b.html"));
    assert_eq!(reference.fragment.as_deref(), Some("A&B C"));
}

#[test]
fn non_url_sources_preserve_literal_percent_and_entity_text() {
    let reference =
        TargetReference::parse("a%20b.md#A%20B&amp;C", TargetReferenceSyntax::Literal).unwrap();
    assert_eq!(reference.file, PathBuf::from("a%20b.md"));
    assert_eq!(reference.fragment.as_deref(), Some("A%20B&amp;C"));
}

#[test]
fn path_or_skill_recognizes_markdown_fragments_and_keeps_skill_names() {
    assert!(TargetReference::uses_path(
        "guide.md#heading",
        &EvidenceTargetResolution::PathOrSkill
    ));
    assert!(TargetReference::uses_path(
        "guide.md",
        &EvidenceTargetResolution::PathOrSkill
    ));
    assert!(!TargetReference::uses_path(
        "other-skill",
        &EvidenceTargetResolution::PathOrSkill
    ));
    assert!(!TargetReference::uses_path(
        "guide.md#heading",
        &EvidenceTargetResolution::Skill
    ));
}
