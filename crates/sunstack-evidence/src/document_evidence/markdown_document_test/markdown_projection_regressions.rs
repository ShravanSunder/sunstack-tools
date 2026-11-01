#![allow(clippy::unwrap_used, clippy::panic)]
use crate::{MarkdownDocument, MarkdownMarkerBinding, MarkerPrefix};

fn assert_projection(text: &str, cap: usize, question: &str, filter: &str) {
    let bindings =
        MarkdownDocument::parse(text).bind_markers(&MarkerPrefix::new("K").unwrap(), cap);
    let [MarkdownMarkerBinding::Bound { binding }] = bindings.as_slice() else {
        panic!("expected one bound marker: {bindings:?}");
    };
    assert_eq!(binding.question_text, question);
    assert_eq!(binding.filter_text, filter);
}

macro_rules! projection_case {
    ($name:ident, $text:expr, $question:expr, $filter:expr) => {
        #[test]
        fn $name() {
            assert_projection($text, 400, $question, $filter);
        }
    };
    ($name:ident, $text:expr, $visible:expr) => {
        projection_case!($name, $text, $visible, $visible);
    };
}

projection_case!(
    table_overflow_comment_is_not_rendered,
    "## Heading<!-- K014 -->\n\n| h |\n| --- |\n| visible | <!-- hidden words --> |\n",
    "Heading\n\n| h |\n| --- |\n| visible |"
);
projection_case!(
    table_overflow_html_and_attributes_are_not_rendered,
    "## Heading<!-- K014 -->\n\n| h |\n| --- |\n| visible | <span title=\"hidden attribute\">text</span> |\n",
    "Heading\n\n| h |\n| --- |\n| visible |"
);
projection_case!(
    table_normal_width_hidden_content_is_stripped,
    "## Heading<!-- K014 -->\n\n| h |\n| --- |\n| visible <!-- hidden words --> <span title=\"hidden attribute\">text</span> |\n",
    "Heading\n\n| h |\n| --- |\n| visible  text |"
);
projection_case!(
    table_alignment_comes_from_rendered_columns,
    "## Heading<!-- K014 -->\n\n| left | right | center | plain |\n| :--- | ---: | :---: | --- |\n| a | b | c | d |\n",
    "Heading\n\n| left | right | center | plain |\n| :-- | --: | :-: | --- |\n| a | b | c | d |"
);
projection_case!(
    table_escaped_pipe_stays_inside_its_rendered_cell,
    "## Heading<!-- K014 -->\n\n| h |\n| --- |\n| left\\|right |\n",
    "Heading\n\n| h |\n| --- |\n| left|right |"
);

projection_case!(
    unresolved_references_keep_literal_syntax_and_render_emphasis,
    "See [*emphasized*][missing] and ![alt][missing].<!-- K014 -->\n",
    "See [emphasized][missing] and ![alt][missing]."
);
projection_case!(
    unresolved_reference_drops_hidden_comment,
    "See [visible <!-- hidden words -->][missing].<!-- K014 -->\n",
    "See [visible ][missing]."
);
projection_case!(
    unresolved_reference_drops_hidden_html,
    "See [visible <span title=\"hidden attribute\">text</span>][missing].<!-- K014 -->\n",
    "See [visible text][missing]."
);
projection_case!(
    unresolved_reference_filters_inline_code,
    "See [`hidden code`][missing].<!-- K014 -->\n",
    "See [hidden code][missing].",
    "See [][missing]."
);
projection_case!(
    resolved_reference_image_and_link_keep_labels,
    "See ![alt][ref] and [label][ref].<!-- K014 -->\n\n[ref]: /image.png\n",
    "See alt and label."
);
projection_case!(
    escaped_reference_label_stays_unresolved,
    "See [label][a\\*b].<!-- K014 -->\n\n[a*b]: /destination\n",
    "See [label][a*b]."
);
projection_case!(
    reference_labels_use_unicode_case_folding,
    "See [label][STRASSE].<!-- K014 -->\n\n[straße]: /destination\n",
    "See label."
);
projection_case!(
    empty_fence_has_no_visible_words,
    "## Heading<!-- K014 -->\n\n```rust\n```\n",
    "Heading"
);

#[test]
fn empty_fence_does_not_exceed_word_cap() {
    assert_projection(
        "## Heading<!-- K014 -->\n\n```rust\n```\n",
        1,
        "Heading",
        "Heading",
    );
}

projection_case!(
    indented_code_strips_structural_indentation,
    "## Heading<!-- K014 -->\n\n    first();\n    second();\n",
    "Heading\n\nfirst();\nsecond();"
);
projection_case!(
    indented_code_preserves_content_indentation,
    "## Heading<!-- K014 -->\n\n    first();\n      second();\n",
    "Heading\n\nfirst();\n  second();"
);
projection_case!(
    quoted_paragraph_strips_quote_prefixes,
    "## Heading<!-- K014 -->\n\n> first\n> second\n",
    "Heading\n\nfirst\nsecond"
);
projection_case!(
    quoted_code_strips_quote_prefixes,
    "## Heading<!-- K014 -->\n\n> ```text\n> first\n> second\n> ```\n",
    "Heading\n\nfirst\nsecond"
);
projection_case!(
    list_code_preserves_bullet_and_strips_structural_indentation,
    "## Heading<!-- K014 -->\n\n- ```text\n  first\n  second\n  ```\n",
    "Heading\n\n- first\nsecond"
);
projection_case!(
    heading_closing_hashes_are_not_visible,
    "## Heading<!-- K014 -->\n\n### Child ###\n\nBody.\n",
    "Heading\n\nChild\n\nBody."
);
projection_case!(
    escaped_closing_hashes_stay_visible,
    "## Heading<!-- K014 -->\n\n### Child \\###\n",
    "Heading\n\nChild ###"
);
projection_case!(
    unspaced_heading_hashes_stay_visible,
    "## Heading<!-- K014 -->\n\n### Child###\n",
    "Heading\n\nChild###"
);
projection_case!(
    hard_break_hides_backslash,
    "first\\\nsecond<!-- K014 -->\n",
    "first\nsecond"
);
projection_case!(
    escaped_literal_backslash_stays_visible,
    "first\\\\\nsecond<!-- K014 -->\n",
    "first\\\nsecond"
);
projection_case!(
    crlf_code_span_is_one_space,
    "A `first\r\nsecond` here.<!-- K014 -->\r\n",
    "A first second here.",
    "A  here."
);
projection_case!(
    lf_code_span_is_one_space,
    "A `first\nsecond` here.<!-- K014 -->\n",
    "A first second here.",
    "A  here."
);
projection_case!(
    code_span_preserves_literal_double_space,
    "A `first  second` here.<!-- K014 -->\n",
    "A first  second here.",
    "A  here."
);
projection_case!(
    code_span_keeps_literal_spaces_beside_crlf,
    "A `first  \r\nsecond` here.<!-- K014 -->\n",
    "A first   second here.",
    "A  here."
);
projection_case!(
    code_span_trims_one_surrounding_space_after_crlf,
    "A `` first\r\nsecond `` here.<!-- K014 -->\n",
    "A first second here.",
    "A  here."
);
projection_case!(
    all_space_code_span_keeps_spaces,
    "A ` \r\n ` here.<!-- K014 -->\n",
    "A     here.",
    "A  here."
);
