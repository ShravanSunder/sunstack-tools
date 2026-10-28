#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use support::*;
#[test]
fn absolute_rule_ref_from_frozen_definition_home_joins_the_case_set() {
    let root = tempfile::tempdir().unwrap();
    let frozen = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let home = root
        .path()
        .join("cases/definitions")
        .canonicalize()
        .unwrap();
    let mut candidate = entry(
        1,
        json!({"kind":"finding","path":["answer_1"]}),
        Some("passed"),
        Some(1),
    );
    candidate.rule.rule_set = home.join("set.json");
    candidate.candidate_id.rule = candidate.rule.clone();
    let observed = bundle(&frozen, vec![candidate], Some(0.0), 1);
    let grade = score(&[observed], &frozen).unwrap();
    assert_eq!(grade.filter.kept, 1);
    assert_eq!(
        grade.end_result.true_findings,
        vec![CaseId::new("case_1").unwrap()]
    );
    assert_eq!(grade.speed.measured_candidates, 1);
}
#[test]
fn bundle_from_live_definitions_refuses_naming_its_frozen_namespace_mismatch() {
    let root = tempfile::tempdir().unwrap();
    let frozen = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let mut candidate = entry(
        1,
        json!({"kind":"finding","path":["answer_1"]}),
        Some("passed"),
        Some(1),
    );
    candidate.rule.rule_set = root.path().join("set.json").canonicalize().unwrap();
    candidate.candidate_id.rule = candidate.rule.clone();
    let observed = bundle(&frozen, vec![candidate], Some(0.0), 1);
    let error = score(&[observed], &frozen).unwrap_err().to_string();
    assert!(error.contains("frozen definitions"), "{error}");
    assert!(error.contains("set.json"), "{error}");
}
