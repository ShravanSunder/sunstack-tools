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
fn html_heading_levels_intersect_text_filters_before_the_match_budget() {
    let raw =
        "猫<h3>Keep wrong-level</h3><h2>Keep correct-level</h2><h3>Keep child</h3><h2>Other</h2>";
    let doc = document(raw);
    let selection = serde_json::from_value(json!({"kind":"html_units","mode":"unit","units":["section"],"levels":[2],"starts_with":"Keep"})).unwrap();

    let result = select_document(&doc, &selection, &Default::default(), 1).unwrap();

    let SelectionContent::Matches { matches } = result else {
        panic!("expected matches")
    };
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].text,
        "<h2>Keep correct-level</h2><h3>Keep child</h3>"
    );
    assert_eq!(matches[0].span.byte_start, raw.find("<h2>").unwrap());
    assert_eq!(matches[0].span.byte_end, raw.find("<h2>Other").unwrap());
}

#[test]
fn html_unit_public_selector_preserves_full_lexical_utf8_ranges() {
    let raw = "猫<h2>Parent</h2>Intro<div><h4>Child</h4>Body</div><h2>Next</h2>End";
    let doc = document(raw);
    let selection: EvidenceSelection =
        serde_json::from_value(json!({"kind":"html_units","mode":"unit","units":["section"]}))
            .unwrap();

    let result = select_document(&doc, &selection, &Default::default(), 10).unwrap();

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

        let SelectionContent::Matches { matches } =
            select_document(&doc, &selection, &Default::default(), 10).unwrap()
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
        select_document(&doc, &selection, &Default::default(), 2)
            .unwrap_err()
            .code,
        EvidenceCode::SelectorLimit
    );
}

#[test]
fn public_html_selector_uses_only_its_explicit_document_settings() {
    let raw = "<custom-source><h2>Nested</h2></custom-source><h2>Main</h2><custom-diagram caption='Flow'>Body</custom-diagram>";
    let doc = document(raw);
    let configured: DocumentEvidenceSettings = serde_json::from_value(json!({"html":{
        "skip_tags":["custom-source"],"text_attributes":{"custom-diagram":["caption"]}
    }}))
    .unwrap();
    let selection = serde_json::from_value(json!({
        "kind":"html_units","mode":"unit","units":["section"],"pattern":"\\[caption: Flow\\]"
    }))
    .unwrap();
    for (settings, expected) in [
        (DocumentEvidenceSettings::default(), 0),
        (configured.clone(), 1),
        (DocumentEvidenceSettings::default(), 0),
    ] {
        let SelectionContent::Matches { matches } =
            select_document(&doc, &selection, &settings, 10).unwrap()
        else {
            panic!("HTML selector must produce matches");
        };
        assert_eq!(matches.len(), expected);
        for found in matches {
            assert_eq!(found.text, raw[found.span.byte_start..found.span.byte_end]);
            assert!(found.text.starts_with("<h2>Main</h2>"));
        }
    }
}
