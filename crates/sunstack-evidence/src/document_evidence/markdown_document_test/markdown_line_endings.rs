#![allow(clippy::unwrap_used, clippy::panic)]
use crate::{BoundMarkdownText, MarkdownDocument, MarkdownMarkerBinding, MarkerPrefix};

fn bindings(text: &str) -> Vec<BoundMarkdownText> {
    MarkdownDocument::parse(text)
        .bind_markers(&MarkerPrefix::new("K").unwrap(), 1000)
        .into_iter()
        .map(|result| match result {
            MarkdownMarkerBinding::Bound { binding } => binding,
            refused => panic!("expected bound marker: {refused:?}"),
        })
        .collect()
}

#[test]
fn quoted_multiline_code_span_has_identical_lf_and_crlf_text() {
    let text = "## Heading<!-- K014 -->\n\n> A `first\n> second` here.\n";
    for source in [text.to_owned(), text.replace('\n', "\r\n")] {
        let found = bindings(&source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].question_text, "Heading\n\nA first second here.");
        assert_eq!(found[0].filter_text, "Heading\n\nA  here.");
    }
}

#[test]
fn list_multiline_code_span_has_identical_lf_and_crlf_text() {
    let text = "## Heading<!-- K014 -->\n\n- A `first\n  second` here.\n";
    for source in [text.to_owned(), text.replace('\n', "\r\n")] {
        let found = bindings(&source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].question_text, "Heading\n\n- A first second here.");
        assert_eq!(found[0].filter_text, "Heading\n\n- A  here.");
    }
}

#[test]
fn lone_cr_in_code_span_is_one_line_ending() {
    let found = bindings("A `first\rsecond` here.<!-- K014 -->\n");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].question_text, "A first second here.");
    assert_eq!(found[0].filter_text, "A  here.");
}

#[test]
fn crlf_and_lf_twins_keep_projection_bytes_equal_and_spans_in_original_coordinates() {
    let lf = "Élan\n\n## Heading<!-- K014 -->\n\nA `first\nsecond` here.<!-- K015 -->\n\n- parent\n  - child<!-- K016 -->\n    continuation\n\n> quoted\n> continuation\n\n```text\nfirst();\n  second();\n```\n\n| left | right |\n| :--- | ---: |\n| a | `b` |\n\nhard\\\nbreak\n\n## Peer\n\nOutside.\n";
    let crlf = lf.replace('\n', "\r\n");
    let left = bindings(lf);
    let right = bindings(&crlf);
    assert_eq!(left.len(), 3);
    assert_eq!(right.len(), 3);
    assert_eq!((left[0].span.byte_start, right[0].span.byte_start), (7, 9));
    assert_eq!(
        (
            left[0].marker.span.byte_start,
            right[0].marker.span.byte_start
        ),
        (17, 19)
    );
    assert_eq!(left[0].span.byte_end, lf.find("## Peer").unwrap());
    assert_eq!(right[0].span.byte_end, crlf.find("## Peer").unwrap());
    for (left, right) in left.iter().zip(&right) {
        assert_eq!(left.marker.id, right.marker.id);
        assert_eq!(left.host, right.host);
        assert_eq!(left.question_text, right.question_text);
        assert_eq!(left.filter_text, right.filter_text);
        assert!(!right.question_text.contains('\r'));
        assert!(!right.filter_text.contains('\r'));
        assert_eq!(
            &lf[left.span.byte_start..left.span.byte_end],
            crlf[right.span.byte_start..right.span.byte_end].replace("\r\n", "\n")
        );
        for (source, binding) in [(lf, left), (crlf.as_str(), right)] {
            let marker = &binding.marker;
            assert_eq!(
                &source[marker.span.byte_start..marker.span.byte_end],
                format!("<!-- {} -->", marker.id.as_str())
            );
            for span in [&marker.span, &binding.span] {
                for (offset, line, column) in [
                    (span.byte_start, span.start_line, span.start_column),
                    (span.byte_end, span.end_line, span.end_column),
                ] {
                    let prefix = &source[..offset];
                    assert_eq!(
                        line,
                        prefix
                            .chars()
                            .filter(|character| *character == '\n')
                            .count()
                            + 1
                    );
                    assert_eq!(
                        column,
                        prefix.rsplit('\n').next().unwrap().chars().count() + 1
                    );
                }
            }
        }
    }
}
