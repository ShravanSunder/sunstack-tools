use super::MarkdownDocument;

#[test]
fn resolves_rendered_atx_and_setext_heading_text() {
    let document = MarkdownDocument::parse(
        "# Hello `code` [linked](https://example.com) *emphasis* &amp; more\n\nSetext heading\n==============\n\n# &#32;spaced&#32;\n",
    );

    for fragment in [
        "hello-code-linked-emphasis--more",
        "setext-heading",
        "-spaced-",
    ] {
        assert!(document.has_anchor(fragment), "missing {fragment:?}");
    }
    assert!(!document.has_anchor("spaced"));
}

#[test]
fn resolves_duplicate_headings_and_suffix_collisions_in_document_order() {
    let document = MarkdownDocument::parse("# Same\n# Same\n# Same-1\n# Same\n");

    for fragment in ["same", "same-1", "same-1-1", "same-2"] {
        assert!(document.has_anchor(fragment), "missing {fragment:?}");
    }
    assert!(!document.has_anchor("same-3"));
}

#[test]
fn ignores_heading_and_anchor_lookalikes_in_non_rendered_content() {
    let document = MarkdownDocument::parse(
        "---\ntitle: fake\n# Frontmatter\n---\n\n<!--\n# Comment heading\n<a id=\"comment-anchor\"></a>\n-->\n\n```html\n# Code heading\n<a id=\"code-anchor\"></a>\n```\n\n    # Indented heading\n\n<div>\n# HTML block heading\n</div>\n\n# Real heading\n",
    );

    assert!(document.has_anchor("real-heading"));
    for fragment in [
        "frontmatter",
        "comment-heading",
        "comment-anchor",
        "code-heading",
        "code-anchor",
        "indented-heading",
        "html-block-heading",
    ] {
        assert!(!document.has_anchor(fragment), "unexpected {fragment:?}");
    }
}

#[test]
fn resolves_explicit_html_ids_and_anchor_names_with_decoded_values() {
    let document = MarkdownDocument::parse(
        "<div ID=\"block&amp;id\"></div>\n\nText <span id='inline&#45;id'></span> <a NAME=\"named&amp;anchor\"></a> <img id=\"self-closing\" />.\n\n<script>\nconst fake = '<a id=\"script-anchor\">';\n</script>\n\n<div name=\"not-an-anchor\"></div>\n",
    );

    for fragment in ["block&id", "inline-id", "named&anchor", "self-closing"] {
        assert!(document.has_anchor(fragment), "missing {fragment:?}");
    }
    for fragment in ["block&amp;id", "script-anchor", "not-an-anchor"] {
        assert!(!document.has_anchor(fragment), "unexpected {fragment:?}");
    }
}

#[test]
fn pinned_minify_badge_heading_resolves_the_rendered_github_anchor() {
    let document = MarkdownDocument::parse(include_str!("heading_anchors_test/minify-heading.md"));
    assert!(document.has_anchor("minify----"));
}

#[test]
fn pinned_minify_badge_heading_refuses_the_phantom_image_alt_anchor() {
    let document = MarkdownDocument::parse(include_str!("heading_anchors_test/minify-heading.md"));
    assert!(!document.has_anchor("minify--api-reference-go-report-card-codecov"));
}

#[test]
fn heading_images_inside_links_leave_spaces_and_ordinary_projection_keeps_alt() {
    use crate::document_evidence::markdown_offset_projection::{
        NormalizedMarkdown, OffsetProjection,
    };
    let heading = "# Before [![Badge **alt** `code`](badge.svg)](https://example.com) After\n";
    let raw = format!("{heading}\n# Plain [linked](https://example.com) `code`\n");
    let document = MarkdownDocument::parse(&raw);
    assert!(document.has_anchor("before--after"));
    assert!(!document.has_anchor("before-badge-alt-code-after"));
    assert!(document.has_anchor("plain-linked-code"));
    let normalized = NormalizedMarkdown::new(&raw);
    let projection = OffsetProjection::new(&normalized);
    assert_eq!(
        projection.prose(0..heading.len(), false).0,
        "Before Badge alt code After"
    );
}

#[test]
fn image_only_headings_keep_empty_and_duplicate_slug_collisions() {
    let document =
        MarkdownDocument::parse("# ![first](a.svg)\n# ![second](b.svg)\n# -1\n# ![third](c.svg)\n");
    for fragment in ["", "-1", "-1-1", "-2"] {
        assert!(document.has_anchor(fragment), "missing {fragment:?}");
    }
    for fragment in ["first", "second", "third", "-3"] {
        assert!(!document.has_anchor(fragment), "unexpected {fragment:?}");
    }
}
