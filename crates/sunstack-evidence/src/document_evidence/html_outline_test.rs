#![allow(clippy::unwrap_used)]
use super::{HtmlSection, html_sections};

#[test]
fn collects_lexical_sections_across_wrappers_and_skipped_levels() {
    let html = "<h1>A</h1>x<div><h3>B</h3>y</div>tail<h2>C</h2>z<h1>D</h1>!";
    assert_eq!(
        html_sections(html).unwrap(),
        vec![
            HtmlSection {
                heading_range: 0..10,
                section_range: 0..48,
                own_range: 0..16,
                enclosing_index: None,
                heading_level: 1,
            },
            HtmlSection {
                heading_range: 16..26,
                section_range: 16..37,
                own_range: 16..37,
                enclosing_index: Some(0),
                heading_level: 3,
            },
            HtmlSection {
                heading_range: 37..47,
                section_range: 37..48,
                own_range: 37..48,
                enclosing_index: Some(0),
                heading_level: 2,
            },
            HtmlSection {
                heading_range: 48..58,
                section_range: 48..59,
                own_range: 48..59,
                enclosing_index: None,
                heading_level: 1,
            },
        ]
    );
}

#[test]
fn handles_all_heading_levels_and_direct_outline_parents() {
    let html = "<h1>1</h1><h2>2</h2><h3>3</h3><h4>4</h4><h5>5</h5><h6>6</h6><h4>next</h4>end";
    let sections = html_sections(html).unwrap();
    assert_eq!(sections.len(), 7);
    assert_eq!(
        sections.iter().map(|s| s.heading_level).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 4]
    );
    assert_eq!(
        sections
            .iter()
            .map(|s| s.enclosing_index)
            .collect::<Vec<_>>(),
        [None, Some(0), Some(1), Some(2), Some(3), Some(4), Some(2)]
    );
    assert_eq!(
        &html[sections[3].section_range.clone()],
        "<h4>4</h4><h5>5</h5><h6>6</h6>"
    );
    assert_eq!(&html[sections[3].own_range.clone()], "<h4>4</h4>");
    assert_eq!(&html[sections[6].section_range.clone()], "<h4>next</h4>end");
}

#[test]
fn ignores_fake_headings_and_markdown_island_source() {
    let html = r#"<h2>Real</h2><div title="<h1>attribute</h1>">&lt;h1&gt;text&lt;/h1&gt;</div><!-- <h1>comment</h1> --><template><h1>template</h1></template><script>const example = '<h1>script</h1>';</script><style>.x::after { content: '<h1>style</h1>'; }</style><ex-markdown><h1>island</h1><script type="application/json" data-ex-markdown-source>{"text":"<h1>json</h1>"}</script></ex-markdown><h3>Child</h3>tail"#;
    let sections = html_sections(html).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(&html[sections[0].heading_range.clone()], "<h2>Real</h2>");
    assert_eq!(&html[sections[1].heading_range.clone()], "<h3>Child</h3>");
    assert_eq!(sections[1].enclosing_index, Some(0));
    assert_eq!(sections[0].own_range.end, sections[1].heading_range.start);
    assert_eq!(sections[1].section_range.end, html.len());
}

#[test]
fn preserves_original_utf8_byte_offsets_and_heading_spelling() {
    let html = "猫<H2 class='x'>Café</H2>🙂<h4>子</h4>終";
    let sections = html_sections(html).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].heading_range, 3..27);
    assert_eq!(
        &html[sections[0].heading_range.clone()],
        "<H2 class='x'>Café</H2>"
    );
    assert_eq!(
        &html[sections[0].own_range.clone()],
        "<H2 class='x'>Café</H2>🙂"
    );
    assert_eq!(&html[sections[1].section_range.clone()], "<h4>子</h4>終");
    assert_eq!(sections[1].enclosing_index, Some(0));
    assert_eq!(sections[1].section_range.end, html.len());
}

fn require_recovered_pair(html: &str, second_start: usize, child: bool) {
    let sections = html_sections(html).unwrap();
    assert_eq!(sections.len(), 2, "{html}");
    assert_eq!(
        sections[0].section_range,
        0..if child { html.len() } else { second_start }
    );
    assert_eq!(sections[0].own_range, 0..second_start);
    assert_eq!(sections[0].enclosing_index, None);
    assert_eq!(sections[1].section_range, second_start..html.len());
    assert_eq!(sections[1].own_range, second_start..html.len());
    assert_eq!(sections[1].enclosing_index, child.then_some(0));
}

#[test]
fn recovered_unclosed_parent_and_child_keep_both_lexical_sections() {
    require_recovered_pair("<h2>Parent<h3>Child", 10, true);
}

#[test]
fn recovered_unclosed_siblings_close_at_the_next_opening_tag() {
    require_recovered_pair("<h2>A<h2>B", 5, false);
}

#[test]
fn recovered_newline_neighbours_keep_original_ranges() {
    require_recovered_pair("<h2>Parent\n<h3>Child\n", 11, true);
}

#[test]
fn unclosed_parent_beside_closed_child_keeps_direct_parent() {
    require_recovered_pair("<h2>Parent<h3>Child</h3>", 10, true);
}

#[test]
fn closed_parent_beside_unclosed_child_keeps_direct_parent() {
    require_recovered_pair("<h2>Parent</h2><h3>Child", 15, true);
}

#[test]
fn recovery_does_not_admit_headings_from_unclosed_excluded_sources() {
    for html in [
        "<template><h2>Hidden<h3>Child",
        "<ex-markdown><h2>Hidden<h3>Child",
    ] {
        assert!(html_sections(html).unwrap().is_empty(), "{html}");
    }
    for html in [
        "<template><h2>Hidden<h3>Fake</template><h2>Real",
        "<ex-markdown><h2>Hidden<h3>Fake</ex-markdown><h2>Real",
    ] {
        let sections = html_sections(html).unwrap();
        let start = html.find("<h2>Real").unwrap();
        assert_eq!(sections.len(), 1, "{html}");
        assert_eq!(sections[0].section_range, start..html.len());
        assert_eq!(sections[0].enclosing_index, None);
    }
}

#[test]
fn self_closing_heading_openings_remain_placeable() {
    let html = "<h2/>Title";
    let sections = html_sections(html).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].heading_level, 2);
    assert_eq!(sections[0].section_range, 0..html.len());
    assert_eq!(sections[0].own_range, 0..html.len());
    assert_eq!(sections[0].enclosing_index, None);
}
