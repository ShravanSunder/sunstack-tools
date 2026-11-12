#![allow(clippy::unwrap_used)]
use crate::rulebook_planning::rulebook_test_support::{entry_json, kinds, piece, project, rule};
use crate::{RuleEntryWire, admit_rule_entry, piece_state};
use serde_json::json;

#[test]
fn reference_is_preserved_in_state_without_implicit_location() {
    let mut encoded = entry_json("K001");
    encoded["reference"] = json!({"examples":{"quiet":{"text":"Portable practice","answer":"pass","why":"No product procedure"}}});
    encoded["questions"]["kind"]["instructions"]["context"] = json!(["reference.examples.quiet"]);
    let entry: RuleEntryWire = serde_json::from_value(encoded.clone()).unwrap();
    let graph = admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap();
    let mut current = rule("K001");
    // Assembly, rather than this test, must transfer the reference into a rule.
    let roundtrip = serde_json::to_value(&entry).unwrap();
    assert_eq!(roundtrip["reference"], encoded["reference"]);
    current.reference = entry.reference;
    current.questions = entry.questions;
    current.decision = graph;
    let state = serde_json::to_value(piece_state(&current, &piece()).unwrap()).unwrap();
    assert_eq!(state["reference"], encoded["reference"]);
    assert!(state.get("where").is_none());
    assert!(state.get("heading").is_none());
}

#[test]
fn reference_paths_and_reserved_names_are_checked_at_load() {
    for key in [
        "standard",
        "where",
        "cut-fields",
        "unavailable-fields",
        "reference",
        "body",
    ] {
        let mut encoded = entry_json("K001");
        encoded["reference"] = json!({key: "reference text"});
        let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
        let errors =
            admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.field_path.to_string().contains("reference"))
        );
    }
    let mut encoded = entry_json("K001");
    encoded["reference"] = json!({"examples":{"quiet":"Portable practice"}});
    encoded["questions"]["kind"]["instructions"]["context"] = json!(["reference.examples.missing"]);
    let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
    assert!(admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).is_err());
}

#[test]
fn question_without_declared_fields_is_refused_and_named_location_has_no_span() {
    let mut encoded = entry_json("K001");
    encoded["questions"]["kind"]["instructions"] =
        json!("Classify this without naming what it reads");
    let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
    let errors = admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("check/context"))
    );
    let mut encoded = entry_json("K001");
    encoded["questions"]["kind"]["instructions"]["context"] = json!(["where"]);
    let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
    let mut current = rule("K001");
    current.decision = admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap();
    current.questions = entry.questions;
    let state = serde_json::to_value(piece_state(&current, &piece()).unwrap()).unwrap();
    assert_eq!(
        state["where"],
        json!({"file":"SKILL.md","first_line":49,"last_line":60})
    );
}

#[test]
fn completeness_check_never_passes_cut_or_unavailable_body() {
    use crate::rulebook_planning::rulebook_test_support::index;
    use crate::{RulebookResult, WalkOutcome, walk_decision};
    use sunstack_evidence::{FieldName, FieldValue, ModelView};
    let mut encoded = entry_json("K001");
    encoded["questions"] = json!({});
    encoded["decision"] = json!({"start":"complete","steps":{"complete":{"mechanical_check":{"tool":"complete","field":"body"},"yes":"pass","no":"uncertain"}}});
    let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
    let mut current = rule("K001");
    current.decision = admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap();
    current.questions = entry.questions;
    let mut evidence = piece();
    assert!(matches!(
        walk_decision(&current, &evidence, &index(), None),
        WalkOutcome::Settled(RulebookResult::Pass { .. })
    ));
    if let ModelView::Bounded { cut, .. } = &mut evidence.model_view {
        cut.push(FieldName::new("body").unwrap());
    }
    assert!(matches!(
        walk_decision(&current, &evidence, &index(), None),
        WalkOutcome::Settled(RulebookResult::Uncertain { .. })
    ));
    evidence = piece();
    evidence.fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Unavailable {
            reason: sunstack_evidence::FieldUnavailable::Unreadable,
        },
    );
    assert!(matches!(
        walk_decision(&current, &evidence, &index(), None),
        WalkOutcome::Settled(RulebookResult::Incomplete { .. })
    ));
}

#[test]
fn every_question_type_requires_the_in_between_route() {
    for (question, step) in [
        (
            json!({"type":"noul","instructions":{"check":"body"}}),
            json!({"ask":"q","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable"}),
        ),
        (
            json!({"type":"choice","instructions":{"check":"body"},"criteria":{"a":"A","b":"B"}}),
            json!({"ask":"q","min_probability":0.6,"routes":{"a":"pass","b":"not_applicable"}}),
        ),
        (
            json!({"type":"score","instructions":{"check":"body"},"criteria":["Low","High"]}),
            json!({"ask":"q","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable"}),
        ),
    ] {
        let mut encoded = entry_json("K001");
        encoded["questions"] = json!({"q":question});
        encoded["decision"] = json!({"start":"q","steps":{"q":step}});
        let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
        let errors =
            admit_rule_entry(&entry, &kinds(), &project(), &Default::default()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("explicit in-between"))
        );
    }
}

#[test]
fn completeness_checks_loaded_openings_and_bounded_projection() {
    use crate::DecisionMechanicalTool;
    use crate::rulebook_planning::rulebook_test_support::index;
    use sunstack_evidence::{FieldName, FieldValue, LoadTarget, project_model_view};
    let field = FieldName::new("targets").unwrap();
    let tool = DecisionMechanicalTool::Complete {
        field: field.clone(),
    };
    let complete = LoadTarget {
        name: "reader".into(),
        path: "skills/reader/SKILL.md".into(),
        exists: true,
        anchor: None,
        candidates: vec![],
        opening: Some("All of the source".into()),
        fields: None,
        opening_cut: false,
        opening_last_line: Some(1),
    };
    for (case, target, expected) in [
        ("whole opening", complete.clone(), true),
        (
            "cut opening",
            LoadTarget {
                opening_cut: true,
                ..complete.clone()
            },
            false,
        ),
        (
            "missing target",
            LoadTarget {
                exists: false,
                anchor: None,
                opening: None,
                ..complete.clone()
            },
            false,
        ),
        (
            "unloaded opening",
            LoadTarget {
                opening: None,
                ..complete.clone()
            },
            false,
        ),
        (
            "ambiguous target",
            LoadTarget {
                candidates: vec!["other/SKILL.md".into()],
                ..complete.clone()
            },
            false,
        ),
    ] {
        let mut evidence = piece();
        evidence.fields.insert(
            field.clone(),
            FieldValue::Targets {
                values: vec![target],
            },
        );
        evidence.model_view = project_model_view(&evidence.fields, 6000);
        assert_eq!(
            super::super::mechanical_check(&tool, &evidence, &index()).unwrap(),
            expected,
            "{case}"
        );
    }
    let mut evidence = piece();
    evidence.fields.insert(
        field.clone(),
        FieldValue::Targets {
            values: vec![complete],
        },
    );
    evidence.model_view = project_model_view(&evidence.fields, 1);
    assert!(!super::super::mechanical_check(&tool, &evidence, &index()).unwrap());
}
