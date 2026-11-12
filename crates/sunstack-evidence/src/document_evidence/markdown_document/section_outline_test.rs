use super::*;

// Assertions use literal source slices rather than reconstructing selector ranges.
fn assert_section(
    source: &str,
    section: &MarkdownSection,
    full: &str,
    own: &[&str],
    enclosing: Option<usize>,
    level: u8,
) {
    assert_eq!(&source[section.section_range.clone()], full);
    assert_eq!(
        section
            .own_ranges
            .iter()
            .map(|range| &source[range.clone()])
            .collect::<Vec<_>>(),
        own
    );
    assert_eq!(section.enclosing_index, enclosing);
    assert_eq!(section.heading_level, level);
}

#[test]
fn ordinary_outline_handles_skipped_levels_and_setext() {
    let source = "Root\n====\n\nintro\n\n### Deep\n\ndeep\n\n## Middle\n\nmid\n\n# Peer\nend\n";
    let sections = MarkdownDocument::parse(source).section_outline();
    assert_eq!(sections.len(), 4);
    assert_section(
        source,
        &sections[0],
        "Root\n====\n\nintro\n\n### Deep\n\ndeep\n\n## Middle\n\nmid\n\n",
        &["Root\n====\n\nintro\n\n"],
        None,
        1,
    );
    assert_section(
        source,
        &sections[1],
        "### Deep\n\ndeep\n\n",
        &["### Deep\n\ndeep\n\n"],
        Some(0),
        3,
    );
    assert_section(
        source,
        &sections[2],
        "## Middle\n\nmid\n\n",
        &["## Middle\n\nmid\n\n"],
        Some(0),
        2,
    );
    assert_section(
        source,
        &sections[3],
        "# Peer\nend\n",
        &["# Peer\nend\n"],
        None,
        1,
    );
}

#[test]
fn list_child_is_clamped_and_outer_own_retains_tail() {
    let source =
        "# Outer\n\nbefore\n\n- lead\n\n  ### Inner\n\n  body\n\nAfter the list.\n\n# Peer\n";
    let sections = MarkdownDocument::parse(source).section_outline();
    assert_eq!(sections.len(), 3);
    assert_section(
        source,
        &sections[0],
        "# Outer\n\nbefore\n\n- lead\n\n  ### Inner\n\n  body\n\nAfter the list.\n\n",
        &["# Outer\n\nbefore\n\n- lead\n\n  ", "After the list.\n\n"],
        None,
        1,
    );
    assert_section(
        source,
        &sections[1],
        "### Inner\n\n  body\n\n",
        &["### Inner\n\n  body\n\n"],
        Some(0),
        3,
    );
    assert_section(source, &sections[2], "# Peer\n", &["# Peer\n"], None, 1);
}

#[test]
fn quote_heading_does_not_close_outer_or_clamp_to_quote() {
    let source = "# Outer\n\n> # Quoted\n> quote\n\nAfter quote.\n\n# Peer\n";
    let sections = MarkdownDocument::parse(source).section_outline();
    assert_eq!(sections.len(), 3);
    assert_section(
        source,
        &sections[0],
        "# Outer\n\n> # Quoted\n> quote\n\nAfter quote.\n\n",
        &["# Outer\n\n> "],
        None,
        1,
    );
    assert_section(
        source,
        &sections[1],
        "# Quoted\n> quote\n\nAfter quote.\n\n",
        &["# Quoted\n> quote\n\nAfter quote.\n\n"],
        None,
        1,
    );
    assert_section(source, &sections[2], "# Peer\n", &["# Peer\n"], None, 1);
}

#[test]
fn preserves_unicode_and_crlf_source_bytes() {
    let source = "# Élan\r\n\r\n茶\r\n\r\n## 子\r\n終\r\n";
    let sections = MarkdownDocument::parse(source).section_outline();
    assert_eq!(sections.len(), 2);
    assert_section(
        source,
        &sections[0],
        source,
        &["# Élan\r\n\r\n茶\r\n\r\n"],
        None,
        1,
    );
    assert_section(
        source,
        &sections[1],
        "## 子\r\n終\r\n",
        &["## 子\r\n終\r\n"],
        Some(0),
        2,
    );
}
