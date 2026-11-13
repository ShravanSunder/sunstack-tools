use super::{HtmlAskingText, html_asking_text, html_has_id, html_text_content};

#[test]
fn projects_html_text_without_interpreting_markdown_or_attributes() {
    let html = r#"<!DOCTYPE html><section id="hidden" title="secret">  Café <b>猫&amp;dog</b>c &lt;tag&gt; &#65; &#x1F642; &nbsp; **literal** <!-- hidden --> end </section>"#;
    assert_eq!(
        html_text_content(html),
        "Café 猫&dog c <tag> A 🙂 **literal** end"
    );
    assert_eq!(html_text_content("a<b>b</b>c"), "a b c");
}

#[test]
fn retains_unknown_entities_and_script_style_raw_text() {
    let html = "<p>&unknown; &amp;lt;</p><script>if (a < b) x = '&amp;';</script><style>a > b { color: red }</style>";
    assert_eq!(
        html_text_content(html),
        "&unknown; &lt; if (a < b) x = '&amp;'; a > b { color: red }"
    );
}

#[test]
fn projects_selected_attributes_in_element_order_and_decodes_once() {
    let html = r#"<p aria-label='前&amp;後' title="secret">Start<img alt="猫 &amp;lt; &#x1F642;"/><ex-mermaid label='Flow &amp; graph'><script type="text/plain">graph TD
A --> B &amp;</script></ex-mermaid><span label="hidden" aria-label=plain>End</span></p>"#;
    assert_eq!(
        html_text_content(html),
        "[aria-label: 前&後] Start [alt: 猫 &lt; 🙂] [label: Flow & graph] graph TD A --> B &amp; [aria-label: plain] End"
    );
}

#[test]
fn separates_adjacent_block_and_inline_elements() {
    let html = "<h2>A</h2><p>B<strong>C</strong>D</p><p>E</p>";
    assert_eq!(html_text_content(html), "A B C D E");
}

#[test]
fn retains_text_recovered_from_bare_ampersands_and_unclosed_elements() {
    assert_eq!(html_text_content("<p>A & B</p>"), "A & B");
    assert_eq!(
        html_text_content("<p title=\"hidden\">A & B <b>nested"),
        "A & B nested"
    );
}

#[test]
fn separates_heading_openings_inside_recovered_nodes() {
    assert_eq!(html_text_content("<h2>Parent<h3>Child"), "Parent Child");
    assert_eq!(
        html_text_content("Before<h2>Parent<h3>Child"),
        "Before Parent Child"
    );
}

#[test]
fn finds_only_exact_decoded_id_attributes_on_elements() {
    let html = r#"<!-- <p id="comment"> --><a name="legacy"></a><p title="id='attribute'" ID='café&amp;&#x732B;'></p><code>&lt;span id="example"&gt;</code><i id=plain></i>"#;
    assert!(html_has_id(html, "café&猫"));
    assert!(html_has_id(html, "plain"));
    for absent in ["comment", "legacy", "attribute", "example", "café", "PLAIN"] {
        assert!(!html_has_id(html, absent), "unexpected id: {absent}");
    }
}

#[test]
fn asking_recovers_the_real_workspace_hierarchy_prose_error_span() {
    use ast_grep_language::{LanguageExt, SupportLang};
    // Exact paragraph from design-page-snapshots 5c02896631399f04216fbdc30668cdeb0497be43,
    // pages/page-03.html:67. The literal greater-than signs produce parser ERROR nodes.
    let raw = r#"<p>Your words: "workspace -> Areas -> Projects"; "board is not a good name". An <strong>area</strong> replaces today's "board". A project is a body of work with a lead.</p>"#;
    let tree = SupportLang::Html.ast_grep(raw);
    let errors = tree
        .root()
        .dfs()
        .filter(|node| node.is_error())
        .map(|node| (node.range(), node.text().into_owned()))
        .collect::<Vec<_>>();
    assert!(!errors.is_empty(), "real input must reach ERROR recovery");
    println!(
        "real ERROR spans: {errors:?}; selector text: {:?}",
        html_text_content(raw)
    );
    assert_eq!(html_asking_text(raw), HtmlAskingText::Complete(
        "Your words: \"workspace -> Areas -> Projects\"; \"board is not a good name\". An area replaces today's \"board\". A project is a body of work with a lead.".into()
    ));
}

#[test]
fn asking_and_selection_keep_existing_ampersand_error_leaves_verbatim() {
    use ast_grep_language::{LanguageExt, SupportLang};
    let raw = "<p>Café & diagram &unknown; &amp; words</p>";
    let tree = SupportLang::Html.ast_grep(raw);
    assert!(
        tree.root().dfs().any(|node| node.is_error()
            && node.children().len() == 0
            && node.text().starts_with('&'))
    );
    let expected = "Café & diagram &unknown; & words";
    assert_eq!(html_text_content(raw), expected);
    assert_eq!(
        html_asking_text(raw),
        HtmlAskingText::Complete(expected.into())
    );
}

#[test]
fn asking_separates_visible_siblings_around_unrecoverable_markup() {
    assert_eq!(
        html_asking_text("Before</h2>Caf&eacute; &amp; 文"),
        HtmlAskingText::Incomplete {
            text: "Before é & 文".into(),
            reason: super::HtmlProjectionReason::UnrecoverableMarkup,
        }
    );
}

#[test]
fn asking_preserves_u1_own_fragment_with_structural_error_tags() {
    use ast_grep_language::{LanguageExt, SupportLang};
    // Native own text is a byte slice and can contain unmatched enclosing tags.
    let raw = r###"<h3>U1 · Work is organized as workspace → areas → projects</h3>
<p>Your words: "workspace -> Areas -> Projects"; "board is not a good name". An <strong>area</strong> replaces today's "board". A project is a body of work with a lead.</p>
<p>Why: today's names and levels (boards, workstreams, milestones) don't match how you think about the work.</p>
<p>Served by <a href="specification.html#R1">R1</a>.</p></section>
<section id="U2">"###;
    let tree = SupportLang::Html.ast_grep(raw);
    let errors = tree
        .root()
        .dfs()
        .filter(|node| node.is_error() && node.text() == "</section>")
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 1);
    println!(
        "U1 structural ERROR kinds: {:?}",
        errors[0]
            .dfs()
            .map(|node| node.kind().into_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(html_asking_text(raw), HtmlAskingText::Complete(r###"U1 · Work is organized as workspace → areas → projects Your words: "workspace -> Areas -> Projects"; "board is not a good name". An area replaces today's "board". A project is a body of work with a lead. Why: today's names and levels (boards, workstreams, milestones) don't match how you think about the work. Served by R1 ."###.into()));
}
