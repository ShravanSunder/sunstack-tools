#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

fn document(text: &str) -> CapturedDocument {
    let now = chrono::Utc::now();
    CapturedDocument {
        interval: CaptureInterval {
            started_at: now,
            completed_at: now,
        },
        source: SourceRef::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "design.html".into(),
            content_identity: content_digest(text.as_bytes()),
        },
        text: text.into(),
    }
}

#[test]
fn html_unit_public_selector_preserves_full_lexical_utf8_ranges() {
    let raw = "猫<h2>Parent</h2>Intro<div><h4>Child</h4>Body</div><h2>Next</h2>End";
    let doc = document(raw);
    let selection: EvidenceSelection =
        serde_json::from_value(json!({"kind":"html_units","mode":"unit","units":["section"]}))
            .unwrap();

    let result = select_document(&doc, &selection, 10).unwrap();

    let SelectionContent::Matches { matches } = result else {
        panic!("expected raw matches");
    };
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].span.byte_start, 3);
    assert_eq!(
        matches[0].text,
        "<h2>Parent</h2>Intro<div><h4>Child</h4>Body</div>"
    );
    assert_eq!(matches[1].text, "<h4>Child</h4>Body</div>");
    assert_eq!(matches[2].text, "<h2>Next</h2>End");
    for found in matches {
        assert_eq!(found.text, raw[found.span.byte_start..found.span.byte_end]);
    }
}

#[test]
fn html_unit_filters_use_projected_visible_text_and_reject_incomplete_match_lists() {
    let doc = document("<h2>Alpha</h2><p>One</p><h2>Beta</h2><img alt='Diagram'><h3>Gamma</h3>End");
    for (filter, expected) in [
        (
            json!({"starts_with":"Beta"}),
            vec!["<h2>Beta</h2><img alt='Diagram'><h3>Gamma</h3>End"],
        ),
        (
            json!({"pattern":"Diagram"}),
            vec!["<h2>Beta</h2><img alt='Diagram'><h3>Gamma</h3>End"],
        ),
    ] {
        let mut wire = json!({"kind":"html_units","mode":"unit","units":["section"]});
        wire.as_object_mut()
            .unwrap()
            .extend(filter.as_object().unwrap().clone());
        let selection = serde_json::from_value(wire).unwrap();

        let SelectionContent::Matches { matches } = select_document(&doc, &selection, 10).unwrap()
        else {
            panic!("expected matches");
        };

        assert_eq!(
            matches
                .iter()
                .map(|found| found.text.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let selection =
        serde_json::from_value(json!({"kind":"html_units","mode":"unit","units":["section"]}))
            .unwrap();
    assert_eq!(
        select_document(&doc, &selection, 2).unwrap_err().code,
        EvidenceCode::SelectorLimit
    );
}
