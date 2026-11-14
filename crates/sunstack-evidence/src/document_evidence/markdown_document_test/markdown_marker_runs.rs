#![allow(clippy::unwrap_used, clippy::panic)]
use crate::{MarkdownBindingRefusal, MarkdownDocument, MarkdownMarkerBinding, MarkerPrefix};

fn bindings(text: &str) -> Vec<MarkdownMarkerBinding> {
    MarkdownDocument::parse(text).bind_markers(&MarkerPrefix::new("K").unwrap(), 400)
}

#[test]
fn trailing_marker_runs_share_host_text_and_keep_each_marker_span() {
    for (text, ids, expected) in [
        (
            "Élan *one point*.<!-- K001 --><!-- K002 -->\r\n",
            vec!["K001", "K002"],
            "Élan one point.",
        ),
        (
            "One point.<!-- K001 --> \t<!-- K002 --><!-- K003 -->  \n",
            vec!["K001", "K002", "K003"],
            "One point.",
        ),
        (
            "## Heading<!-- K001 --> \t<!-- K002 -->\n\nBody.\n",
            vec!["K001", "K002"],
            "Heading\n\nBody.",
        ),
        (
            "- One point.<!-- K001 --><!-- K002 -->\n  Continued.\n",
            vec!["K001", "K002"],
            "- One point.\n  Continued.",
        ),
    ] {
        let found = bindings(text);
        assert_eq!(found.len(), ids.len(), "{text:?}: {found:?}");
        let mut host_span = None;
        for (binding, id) in found.iter().zip(ids) {
            let MarkdownMarkerBinding::Bound { binding } = binding else {
                panic!("{binding:?}")
            };
            assert_eq!(binding.marker.id.as_str(), id);
            assert_eq!(binding.question_text, expected);
            assert_eq!(binding.filter_text, expected);
            assert_eq!(
                &text[binding.marker.span.byte_start..binding.marker.span.byte_end],
                format!("<!-- {id} -->")
            );
            if let Some(span) = &host_span {
                assert_eq!(&binding.span, span);
            } else {
                host_span = Some(binding.span.clone());
            }
        }
    }
}

#[test]
fn marker_runs_keep_escape_code_prefix_and_heading_restrictions() {
    for text in [
        "```md\nExample<!-- K001 --><!-- K002 -->\n```\n",
        "    Example<!-- K001 --><!-- K002 -->\n",
        "`Example<!-- K001 --><!-- K002 -->\ncontinued`\n",
        "Escaped \\<!-- K001 -->\\<!-- K002 -->\n",
        "Text<!-- K001 --><!-- K002 --> trailing\n",
    ] {
        assert!(bindings(text).is_empty(), "{text:?}");
    }
    let mixed = bindings("Text<!-- Z001 --><!-- K001 --><!-- Z002 -->\n");
    assert_eq!(mixed.len(), 1);
    assert!(
        matches!(&mixed[0],MarkdownMarkerBinding::Bound {binding} if binding.marker.id.as_str()=="K001" && binding.question_text=="Text")
    );
    let heading = bindings("## Heading <!-- K001 --><!-- K002 -->\n");
    assert_eq!(heading.len(), 2);
    assert!(heading.iter().all(|binding| matches!(
        binding,
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::SpacedHeading,
            ..
        }
    )));
    let duplicate = bindings("Text<!-- K001 --><!-- K001 -->\n");
    assert_eq!(duplicate.len(), 2);
    assert!(duplicate.iter().all(|binding| matches!(
        binding,
        MarkdownMarkerBinding::Refused {
            reason: MarkdownBindingRefusal::DuplicateId,
            ..
        }
    )));
}
