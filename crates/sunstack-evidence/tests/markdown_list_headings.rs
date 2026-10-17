#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_evidence::{
    BoundMarkdownText, MarkdownBindingRefusal, MarkdownDocument, MarkdownHostKind,
    MarkdownMarkerBinding, MarkerPrefix,
};

fn bindings(text: &str) -> Vec<MarkdownMarkerBinding> {
    MarkdownDocument::parse(text).bind_markers(&MarkerPrefix::new("K").unwrap(), 400)
}
fn bound(value: &MarkdownMarkerBinding) -> &BoundMarkdownText {
    let MarkdownMarkerBinding::Bound { binding } = value else {
        panic!("expected bound marker: {value:?}")
    };
    binding
}

#[test]
fn heading_on_item_line_binds_item_and_excludes_following_document_content() {
    let text = "- ## Heading<!-- K014 -->\n  body\n\nAfter the list.\n\n## Peer\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::ListItem);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (0, text.find("After the list.").unwrap())
    );
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "- ## Heading<!-- K014 -->\n  body\n\n"
    );
    assert_eq!(binding.question_text, "- Heading\n\nbody");
    assert_eq!(binding.filter_text, binding.question_text);
}

#[test]
fn later_heading_section_stops_at_its_enclosing_item_end() {
    let text = "- lead\n\n  ### Inner<!-- K014 -->\n\n  body\n\nAfter the list.\n\n## Peer\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (
            text.find("### Inner").unwrap(),
            text.find("After the list.").unwrap()
        )
    );
    assert_eq!(binding.question_text, "Inner\n\nbody");
}

#[test]
fn later_heading_section_stops_at_same_level_heading_in_the_same_item() {
    let text = "- lead\n\n  ### Inner<!-- K014 -->\n\n  body\n\n  ### Peer\n\n  excluded\n\nAfter the list.\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(binding.span.byte_end, text.find("### Peer").unwrap());
    assert_eq!(binding.question_text, "Inner\n\nbody");
}

#[test]
fn nested_item_heading_binds_child_item_without_taking_over_its_parent() {
    let text =
        "- parent<!-- K014 -->\n  - ## Child<!-- K015 -->\n    body\n- sibling\n\nOutside.\n";
    let found = bindings(text);
    assert_eq!(found.len(), 2);
    let parent = bound(&found[0]);
    let child = bound(&found[1]);
    assert_eq!(parent.host, MarkdownHostKind::ListItem);
    assert_eq!(child.host, MarkdownHostKind::ListItem);
    assert_eq!(child.span.byte_start, text.find("- ## Child").unwrap());
    assert_eq!(child.span.byte_end, text.find("- sibling").unwrap());
    assert!(parent.span.byte_start < child.span.byte_start);
    assert_eq!(parent.span.byte_end, child.span.byte_end);
    assert!(child.question_text.contains("Child"));
    assert!(child.question_text.contains("body"));
    assert!(!child.question_text.contains("parent"));
    assert!(!child.question_text.contains("sibling"));
    assert!(!child.question_text.contains("Outside"));
}

#[test]
fn top_level_heading_section_still_stops_at_its_document_peer() {
    let text = "## Heading<!-- K014 -->\n\nbody\n\n## Peer\n\nOutside.\n";
    let found = bindings(text);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (0, text.find("## Peer").unwrap())
    );
    assert_eq!(binding.question_text, "Heading\n\nbody");
}

#[test]
fn list_promotion_does_not_bypass_heading_spacing_or_h1_refusals() {
    for (text, reason) in [
        (
            "- ## Heading <!-- K014 -->\n",
            MarkdownBindingRefusal::SpacedHeading,
        ),
        (
            "- # Heading<!-- K014 -->\n",
            MarkdownBindingRefusal::ForbiddenHost,
        ),
    ] {
        let found = bindings(text);
        assert_eq!(found.len(), 1);
        assert!(
            matches!(&found[0],MarkdownMarkerBinding::Refused { reason:found,.. } if *found==reason),
            "{found:?}"
        );
    }
}

#[test]
fn later_line_heading_does_not_copy_a_bare_marker_outside_its_span() {
    let text = "-\n  ## Heading<!-- K014 -->\n\n  body\n\nOutside.\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!((binding.span.byte_start, binding.span.byte_end), (4, 37));
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "## Heading<!-- K014 -->\n\n  body\n\n"
    );
    assert_eq!(binding.question_text, "Heading\n\nbody");
    assert_eq!(binding.filter_text, "Heading\n\nbody");
}

#[test]
fn later_line_paragraph_does_not_copy_a_bare_marker_outside_its_span() {
    let text = "-\n  paragraph<!-- K014 -->\n\nOutside.\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Paragraph);
    assert_eq!(binding.span.byte_start, 4);
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "paragraph<!-- K014 -->\n"
    );
    assert_eq!(binding.question_text, "paragraph");
    assert_eq!(binding.filter_text, "paragraph");
}

#[test]
fn first_line_paragraph_and_heading_keep_separated_list_prefixes() {
    for (text, question) in [
        ("- text<!-- K014 -->\n", "- text"),
        ("- ## Heading<!-- K014 -->\n", "- Heading"),
    ] {
        let found = bindings(text);
        assert_eq!(found.len(), 1);
        let binding = bound(&found[0]);
        assert_eq!(binding.host, MarkdownHostKind::ListItem);
        assert_eq!(
            (binding.span.byte_start, binding.span.byte_end),
            (0, text.len())
        );
        assert_eq!(binding.question_text, question);
        assert_eq!(binding.filter_text, question);
    }
}

#[test]
fn section_keeps_bare_unordered_markers_inside_its_span() {
    let text = "## S<!-- K014 -->\n\n-\n  first\n- second\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (0, text.len())
    );
    assert_eq!(binding.question_text, "S\n\n- first\n- second");
    assert_eq!(binding.filter_text, binding.question_text);
}

#[test]
fn section_keeps_bare_ordered_markers_inside_its_span() {
    let text = "## S<!-- K014 -->\n\n1.\n   first\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (0, text.len())
    );
    assert_eq!(binding.question_text, "S\n\n1. first");
    assert_eq!(binding.filter_text, binding.question_text);
}

#[test]
fn section_preserves_nested_marker_indentation_with_later_line_content() {
    let text = "## S<!-- K014 -->\n\n- parent\n\n  -\n    child\n- sibling\n";
    let found = bindings(text);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(binding.question_text, "S\n\n- parent\n  - child\n- sibling");
    assert_eq!(binding.filter_text, binding.question_text);
}

#[test]
fn included_markers_use_one_separator_space_after_the_marker_text() {
    for (text, question) in [
        ("-   text<!-- K014 -->\n", "- text"),
        ("1.   text<!-- K014 -->\n", "1. text"),
    ] {
        let found = bindings(text);
        assert_eq!(found.len(), 1);
        let binding = bound(&found[0]);
        assert_eq!(binding.host, MarkdownHostKind::ListItem);
        assert_eq!(binding.question_text, question);
        assert_eq!(binding.filter_text, question);
    }
}
