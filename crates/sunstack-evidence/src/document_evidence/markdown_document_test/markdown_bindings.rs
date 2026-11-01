#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;

fn source() -> SourceRef {
    SourceRef::Document {
        repository: RepositoryId::new("markdown_cases").unwrap(),
        path: "skill.md".into(),
        content_identity: "frozen_fixture".into(),
    }
}

fn bindings(text: &str, cap: usize) -> Vec<MarkdownMarkerBinding> {
    MarkdownDocument::parse(text).bind_markers(&MarkerPrefix::new("K").unwrap(), cap)
}

fn bound(binding: &MarkdownMarkerBinding) -> &BoundMarkdownText {
    let MarkdownMarkerBinding::Bound { binding } = binding else {
        panic!("expected bound text: {binding:?}")
    };
    binding
}

#[test]
fn only_exact_end_of_line_markers_outside_code_are_discovered() {
    let text = "prose <!-- K014 --> trailing\n\nprose <!-- K0014 -->\n\nprose <!-- K014 extra -->\n\nprose <!-- Z014 -->\n\n`example <!-- K014 -->`\n\n`multiline <!-- K015 -->\nend`\n\n```md\n<!-- K016 -->\n```\n\n    <!-- K017 -->\n\nreal <!-- K018 -->  \t\r\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 1);
    let result = bound(&found[0]);
    assert_eq!(result.marker.id.as_str(), "K018");
    assert_eq!(result.question_text, "real");
    assert_eq!(
        &text[result.marker.span.byte_start..result.marker.span.byte_end],
        "<!-- K018 -->"
    );
}

#[test]
fn all_allowed_hosts_bind_exact_ranges_and_lists_win_over_paragraphs() {
    for level in 2..=6 {
        let text = format!(
            "{} Heading<!-- K014 -->\n\nbody\n\n# Stop\n",
            "#".repeat(level)
        );
        let result = bindings(&text, 400);
        let binding = bound(&result[0]);
        assert_eq!(binding.host, MarkdownHostKind::Section);
        assert_eq!(
            &text[binding.span.byte_start..binding.span.byte_end],
            &text[..text.find("# Stop").unwrap()]
        );
        assert_eq!(binding.question_text, "Heading\n\nbody");
    }
    let text = "before\n\nwrapped\nparagraph<!-- K014 -->\n\nafter\n";
    let found = bindings(text, 400);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Paragraph);
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "wrapped\nparagraph<!-- K014 -->\n"
    );

    let text = "- parent<!-- K014 -->\n  - child\n    continued\n- sibling\n";
    let found = bindings(text, 400);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::ListItem);
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "- parent<!-- K014 -->\n  - child\n    continued\n"
    );
    assert!(binding.question_text.contains("child"));
    assert!(!binding.question_text.contains("sibling"));
}

#[test]
fn forbidden_containers_refuse_even_when_they_contain_an_allowed_paragraph() {
    for text in [
        "# Heading<!-- K014 -->\n",
        "> text<!-- K014 -->\n",
        "> - item<!-- K014 -->\n",
        "| column |\n| --- |\n| cell<!-- K014 -->\n",
        "<div>\ntext<!-- K014 -->\n</div>\n",
        "---\nname: text<!-- K014 -->\n---\n",
        "[ref]: /path <!-- K014 -->\n",
        "[^foot]: text<!-- K014 -->\n",
        "<!-- K014 -->\n",
    ] {
        let found = bindings(text, 400);
        assert_eq!(found.len(), 1, "{text:?}: {found:?}");
        assert!(
            matches!(
                &found[0],
                MarkdownMarkerBinding::Refused {
                    reason: MarkdownBindingRefusal::ForbiddenHost,
                    ..
                }
            ),
            "{text:?}: {found:?}"
        );
    }
}

#[test]
fn spaced_heading_and_every_duplicate_occurrence_are_refused_with_marker_locations() {
    let text = "## Heading <!-- K014 -->\n";
    assert!(matches!(
        &bindings(text, 400)[0],
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::SpacedHeading,
            ..
        }
    ));
    let text = "first<!-- K014 -->\n\nsecond<!-- K014 -->\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 2);
    for (index, result) in found.iter().enumerate() {
        let MarkdownMarkerBinding::Refused {
            marker,
            reason: MarkdownBindingRefusal::DuplicateId,
        } = result
        else {
            panic!("{result:?}")
        };
        assert_eq!(marker.span.start_line, 1 + 2 * index);
        assert!(
            serde_json::to_value(&marker.span)
                .unwrap()
                .get("source")
                .is_none()
        );
    }
}

#[test]
fn link_and_footnote_mentions_inside_prose_are_allowed() {
    for text in [
        "Read [x] here<!-- K014 -->\n",
        "A note [^1] here<!-- K014 -->\n",
    ] {
        assert!(matches!(
            &bindings(text, 400)[0],
            MarkdownMarkerBinding::Bound { .. }
        ));
    }
}

#[test]
fn leading_bracket_prose_is_not_a_definition_prefix() {
    for text in [
        "[first] and [later]: ordinary prose<!-- K014 -->\n",
        "[^first] and [later]: ordinary prose<!-- K014 -->\n",
    ] {
        let found = bindings(text, 400);
        assert_eq!(found.len(), 1);
        assert_eq!(bound(&found[0]).host, MarkdownHostKind::Paragraph);
    }
}

#[test]
fn escaped_brackets_in_definition_labels_still_refuse() {
    let found = bindings("[label\\]tail]: /path <!-- K014 -->\n", 400);
    assert_eq!(found.len(), 1);
    assert!(matches!(
        &found[0],
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::ForbiddenHost,
            ..
        }
    ));
}

#[test]
fn sections_include_subsections_stop_at_peers_and_nested_markers_overlap() {
    let text = "## Parent<!-- K014 -->\n\nintro\n\n### Child<!-- K015 -->\n\nchild\n\n## Peer\n\nexcluded\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 2);
    let parent = bound(&found[0]);
    let child = bound(&found[1]);
    assert_eq!(parent.span.byte_start, 0);
    assert_eq!(parent.span.byte_end, text.find("## Peer").unwrap());
    assert_eq!(child.span.byte_start, text.find("### Child").unwrap());
    assert_eq!(child.span.byte_end, parent.span.byte_end);
    assert_eq!(parent.question_text, "Parent\n\nintro\n\nChild\n\nchild");
    assert_eq!(child.question_text, "Child\n\nchild");
}

#[test]
fn setext_heading_binds_its_section_through_body_and_stops_at_peer() {
    let text = "Heading<!-- K014 -->\n---\n\nBody is required.\n\n## Peer\n\nOutside.\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Section);
    assert_eq!(
        (binding.span.byte_start, binding.span.byte_end),
        (0, text.find("## Peer").unwrap())
    );
    assert_eq!(binding.question_text, "Heading\n\nBody is required.");
}

#[test]
fn setext_heading_requires_an_adjacent_marker() {
    let found = bindings("Heading <!-- K014 -->\n---\n\nBody.\n", 400);
    assert_eq!(found.len(), 1);
    assert!(matches!(
        &found[0],
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::SpacedHeading,
            ..
        }
    ));
}

#[test]
fn setext_h1_marker_is_still_a_forbidden_host() {
    let found = bindings("Heading<!-- K014 -->\n===\n\nBody.\n", 400);
    assert_eq!(found.len(), 1);
    assert!(matches!(
        &found[0],
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::ForbiddenHost,
            ..
        }
    ));
}

#[test]
fn escaped_comment_markers_are_literal_and_do_not_duplicate_a_real_marker() {
    assert!(bindings("Literal \\<!-- K014 -->\n", 400).is_empty());
    let found = bindings("Literal \\<!-- K014 -->\n\nActual<!-- K014 -->\n", 400);
    assert_eq!(found.len(), 1);
    assert_eq!(bound(&found[0]).question_text, "Actual");
    let even_escape = bindings("Literal \\\\<!-- K014 -->\n", 400);
    assert_eq!(even_escape.len(), 1);
    assert_eq!(bound(&even_escape[0]).question_text, "Literal \\");
}

#[test]
fn nested_list_markers_bind_their_own_items_with_overlapping_parent_content() {
    let text = "- parent<!-- K014 -->\n  - child<!-- K015 -->\n- sibling\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 2);
    let parent = bound(&found[0]);
    let child = bound(&found[1]);
    assert_eq!(parent.host, MarkdownHostKind::ListItem);
    assert_eq!(child.host, MarkdownHostKind::ListItem);
    assert_eq!(
        &text[parent.span.byte_start..parent.span.byte_end],
        "- parent<!-- K014 -->\n  - child<!-- K015 -->\n"
    );
    assert_eq!(
        &text[child.span.byte_start..child.span.byte_end],
        "- child<!-- K015 -->\n"
    );
    assert!(parent.span.byte_start < child.span.byte_start);
    assert_eq!(parent.span.byte_end, child.span.byte_end);
}

#[test]
fn continuation_paragraph_marker_binds_only_its_own_paragraph() {
    let text = "- first\n\n  second<!-- K014 -->\n";
    let found = bindings(text, 400);
    assert_eq!(found.len(), 1);
    let binding = bound(&found[0]);
    assert_eq!(binding.host, MarkdownHostKind::Paragraph);
    assert_eq!(binding.span.byte_start, text.find("second").unwrap());
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "second<!-- K014 -->\n"
    );
    assert_eq!(binding.question_text, "second");
}

#[test]
fn question_keeps_visible_words_code_tables_and_alt_text_but_strips_hidden_sources() {
    let text = "## Visible<!-- K014 -->\n\nA *strong* **standard** with `inline code`, [label](https://hidden.example) and ![alt words](secret.png).\n\n<!-- hidden comment -->\n\n<div>hidden block</div>\n\nRaw <span>visible</span>.\n\n[ref]: https://secret.example \"hidden title\"\n\n```rust\nlet answer = 42;\n```\n\n| Field | Value |\n| --- | --- |\n| name | text |\n\n## Stop\n";
    let found = bindings(text, 400);
    let binding = bound(&found[0]);
    assert_eq!(
        binding.question_text,
        "Visible\n\nA strong standard with inline code, label and alt words.\n\nRaw visible.\n\nlet answer = 42;\n\n| Field | Value |\n| --- | --- |\n| name | text |"
    );
    assert!(!binding.filter_text.contains("inline code"));
    assert!(binding.question_text.contains("inline code"));
    for hidden in ["K014", "hidden", "secret", "<span>", "[ref]"] {
        assert!(!binding.question_text.contains(hidden), "{hidden}");
    }
}

#[test]
fn question_word_cap_is_inclusive_and_uses_visible_words_and_is_deterministic() {
    let text = "one *two* `three`<!-- K014 -->\n";
    assert!(matches!(
        &bindings(text, 3)[0],
        MarkdownMarkerBinding::Bound { .. }
    ));
    assert!(matches!(
        &bindings(text, 2)[0],
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::QuestionTooLarge {
                word_count: 3,
                max_question_words: 2
            },
            ..
        }
    ));
    assert_eq!(bindings(text, 400), bindings(text, 400));
}

#[test]
fn question_retains_list_structure_and_continuation_indentation() {
    let text = "## Lists<!-- K014 -->\n\n- parent\n  - child\n    continuation\n- sibling\n";
    let found = bindings(text, 400);
    assert_eq!(
        bound(&found[0]).question_text,
        "Lists\n\n- parent\n  - child\n    continuation\n- sibling"
    );
}

#[test]
fn visible_inline_projection_decodes_entities_escapes_and_resolves_reference_links() {
    let text = "## Inline<!-- K014 -->\n\nA &amp; B &#x3a9; \\*literal\\* [label][ref], [unresolved], <https://example.com>.\n\n[ref]: /hidden\n";
    let found = bindings(text, 400);
    assert_eq!(
        bound(&found[0]).question_text,
        "Inline\n\nA & B Ω *literal* label, [unresolved], https://example.com."
    );
}

#[test]
fn comments_with_other_content_are_not_markers() {
    for text in [
        "prose <!-- before K014 -->\n",
        "<!-- K014 after -->\n",
        "<!--\nK014\n-->\n",
    ] {
        assert!(bindings(text, 400).is_empty(), "{text:?}");
    }
}

#[test]
fn table_cells_strip_hidden_inline_content_and_filter_code_spans() {
    let text = "## Table<!-- K014 -->\n\n| Field | Value |\n| --- | --- |\n| name<!-- secret --> | `literal` <span>visible</span> |\n";
    let found = bindings(text, 400);
    let binding = bound(&found[0]);
    assert!(!binding.question_text.contains("secret"));
    assert!(!binding.question_text.contains("<span>"));
    assert!(binding.question_text.contains("literal"));
    assert!(!binding.filter_text.contains("literal"));
}

#[test]
fn unicode_and_crlf_markers_keep_source_coordinates_and_raw_bytes() {
    let text = "intro\r\n\r\nÉlan<!-- K014 -->\r\n";
    let found = bindings(text, 400);
    let binding = bound(&found[0]);
    assert_eq!(
        (
            binding.marker.span.start_line,
            binding.marker.span.start_column
        ),
        (3, 5)
    );
    assert_eq!(binding.marker.span.byte_start, text.find("<!--").unwrap());
    assert_eq!(
        &text[binding.span.byte_start..binding.span.byte_end],
        "Élan<!-- K014 -->\r\n"
    );
    assert_eq!(binding.question_text, "Élan");
    let evidence_span = source_span(
        &source(),
        text,
        binding.marker.span.byte_start..binding.marker.span.byte_end,
    );
    assert_eq!(
        (
            binding.marker.span.start_line,
            binding.marker.span.start_column,
            binding.marker.span.end_line,
            binding.marker.span.end_column
        ),
        (
            evidence_span.start_line,
            evidence_span.start_column,
            evidence_span.end_line,
            evidence_span.end_column
        )
    );
}

#[test]
fn definition_binding_spans_carry_coordinates_without_evidence_source_identity() {
    let found = bindings("ordinary<!-- K014 -->\n", 400);
    let binding = bound(&found[0]);
    for value in [
        serde_json::to_value(&binding.span).unwrap(),
        serde_json::to_value(&binding.marker.span).unwrap(),
    ] {
        assert!(value.get("source").is_none(), "{value}");
        assert!(value.get("byte_start").is_some());
        assert!(value.get("start_column").is_some());
    }
}
