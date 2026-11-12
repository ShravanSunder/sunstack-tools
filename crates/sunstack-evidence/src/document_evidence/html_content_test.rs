use super::{html_has_id, html_text_content};

#[test]
fn projects_html_text_without_interpreting_markdown_or_attributes() {
    let html = r#"<!DOCTYPE html><section id="hidden" title="secret">  Café <b>猫&amp;dog</b>c &lt;tag&gt; &#65; &#x1F642; &nbsp; **literal** <!-- hidden --> end </section>"#;
    assert_eq!(
        html_text_content(html),
        "Café 猫&dogc <tag> A 🙂 **literal** end"
    );
    assert_eq!(html_text_content("a<b>b</b>c"), "abc");
}

#[test]
fn retains_unknown_entities_and_script_style_raw_text() {
    let html = "<p>&unknown; &amp;lt;</p><script>if (a < b) x = '&amp;';</script><style>a > b { color: red }</style>";
    assert_eq!(
        html_text_content(html),
        "&unknown; &lt;if (a < b) x = '&amp;';a > b { color: red }"
    );
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
fn finds_only_exact_decoded_id_attributes_on_elements() {
    let html = r#"<!-- <p id="comment"> --><a name="legacy"></a><p title="id='attribute'" ID='café&amp;&#x732B;'></p><code>&lt;span id="example"&gt;</code><i id=plain></i>"#;
    assert!(html_has_id(html, "café&猫"));
    assert!(html_has_id(html, "plain"));
    for absent in ["comment", "legacy", "attribute", "example", "café", "PLAIN"] {
        assert!(!html_has_id(html, absent), "unexpected id: {absent}");
    }
}
