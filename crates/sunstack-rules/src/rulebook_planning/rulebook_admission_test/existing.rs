#![allow(clippy::unwrap_used)]
use super::super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use serde_json::json;

#[test]
fn worked_entries_are_admitted_and_every_graph_refusal_is_detected_before_walk() {
    let kinds = fixture::kinds();
    let project = fixture::project();
    for marker in ["K001", "K002", "K003"] {
        assert!(admit_rule_entry(&fixture::entry(marker), &kinds, &project).is_ok());
    }
    let cases = [
        ("/evidence", json!("undefined")),
        ("/decision/start", json!("missing")),
        ("/decision/steps/has_completion/yes", json!("missing")),
        (
            "/decision/steps/has_completion/yes",
            json!("has_completion"),
        ),
        ("/decision/steps/has_completion/no/at", json!("undefined")),
        (
            "/decision/steps/has_completion/mechanical_check/rule",
            json!("undefined"),
        ),
        ("/decision/steps/kind/ask", json!("undefined")),
        ("/decision/steps/kind/min_probability", json!(0)),
        ("/decision/steps/kind/min_probability", json!(1.1)),
        ("/decision/steps/kind/routes", json!({"outcome":"pass"})),
        ("/decision/steps/quality/combine", json!({"kind":1.0})),
        ("/decision/steps/quality/combine", json!({"undefined":1.0})),
        (
            "/decision/steps/quality/combine",
            json!({"covers":-0.1,"checkable":1.1}),
        ),
        (
            "/decision/steps/quality/combine",
            json!({"covers":0.5,"checkable":0.4}),
        ),
        ("/decision/steps/quality/yes_at_or_above", json!(0.6)),
        ("/decision/steps/quality/yes_at_or_above", json!(1.01)),
        ("/decision/steps/quality/no_at_or_below", json!(-0.01)),
        (
            "/questions/kind/criteria",
            json!({"outcome":"Only one option."}),
        ),
        ("/questions/checkable/criteria", json!(["Only one level."])),
        ("/questions/covers/instructions/check", json!("undefined")),
    ];
    for (path, value) in cases {
        let mut entry = fixture::entry_json("K001");
        if path.ends_with("/at") {
            entry["decision"]["steps"]["has_completion"]["no"]["at"] = value;
        } else {
            *entry.pointer_mut(path).unwrap() = value;
        }
        let wire: crate::RuleEntryWire = serde_json::from_value(entry).unwrap();
        assert!(admit_rule_entry(&wire, &kinds, &project).is_err(), "{path}");
    }
}

#[test]
fn mechanical_tools_refuse_field_shape_mismatches_and_invalid_regex() {
    let kinds = fixture::kinds();
    let project = fixture::project();
    for tool in [
        json!({"tool":"regex","field":"targets","pattern":"."}),
        json!({"tool":"file_exists","field":"body"}),
        json!({"tool":"regex","field":"body","pattern":"["}),
        json!({"tool":"max_chars","field":"completion","max":300}),
        json!({"tool":"contains","field":"undefined","text":"x"}),
    ] {
        let mut entry = fixture::entry_json("K001");
        entry["decision"]["steps"]["has_completion"]["mechanical_check"] = tool;
        let entry = serde_json::from_value(entry).unwrap();
        assert!(admit_rule_entry(&entry, &kinds, &project).is_err());
    }
}

#[test]
fn kind_refusals_cover_rules_regex_sizes_and_one_level_composition() {
    let project = fixture::project();
    assert!(admit_evidence_kinds(&fixture::kinds(), &project, 6000).is_empty());
    let mut kinds = fixture::kinds();
    kinds
        .get_mut(&EvidenceKindName::new("load").unwrap())
        .unwrap()
        .fields
        .insert(
            FieldName::new("bad").unwrap(),
            FieldSource::Target {
                regex: "[".into(),
                resolve: sunstack_evidence::EvidenceTargetResolution::Path,
                opening_chars: 3000,
                as_kind: Some(EvidenceKindName::new("load").unwrap()),
                exclude: None,
            },
        );
    kinds
        .get_mut(&EvidenceKindName::new("step").unwrap())
        .unwrap()
        .rule = sunstack_evidence::EvidenceRuleId::new("missing").unwrap();
    kinds
        .get_mut(&EvidenceKindName::new("skill").unwrap())
        .unwrap()
        .max_chars = std::num::NonZeroU32::new(999);
    assert!(admit_evidence_kinds(&kinds, &project, 999).len() >= 5);
}
