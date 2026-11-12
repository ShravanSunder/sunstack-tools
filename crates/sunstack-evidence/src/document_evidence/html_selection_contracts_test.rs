#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn html_unit_filters_accept_matching_text_and_reject_nonmatching_text() {
    for (wire, selected, rejected) in [
        (
            json!({"mode":"unit","units":["section"],"starts_with":"Parent"}),
            "Parent intro",
            "Child Parent",
        ),
        (
            json!({"mode":"unit","units":["section"],"pattern":"(?i)diagram"}),
            "A DIAGRAM below",
            "Only prose",
        ),
    ] {
        let selection: HtmlUnitSelection = serde_json::from_value(wire).unwrap();
        assert!(selection.matches_text(selected).unwrap());
        assert!(!selection.matches_text(rejected).unwrap());
    }
    let unfiltered: HtmlUnitSelection =
        serde_json::from_value(json!({"mode":"unit","units":["section"]})).unwrap();
    assert!(unfiltered.matches_text("A section").unwrap());
}

#[test]
fn html_unit_validation_refuses_invalid_programmatic_selections() {
    for selection in [
        HtmlUnitSelection::Unit {
            units: vec![],
            filter: None,
        },
        HtmlUnitSelection::Unit {
            units: vec![HtmlUnitKind::Section],
            filter: Some(HtmlUnitFilter::StartsWith {
                starts_with: String::new(),
            }),
        },
        HtmlUnitSelection::Unit {
            units: vec![HtmlUnitKind::Section],
            filter: Some(HtmlUnitFilter::Regex {
                pattern: "a*".into(),
            }),
        },
    ] {
        assert_eq!(
            selection.validate().unwrap_err().code,
            EvidenceCode::InvalidSelector
        );
        assert_eq!(
            selection.matches_text("A").unwrap_err().code,
            EvidenceCode::InvalidSelector
        );
    }
}
