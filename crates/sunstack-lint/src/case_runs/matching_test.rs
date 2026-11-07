#![allow(clippy::unwrap_used)]
use super::*;
use crate::{EffectiveLimits, LintReport, RulePieceResult, ScopeChoice};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn exact_range_matching_keeps_duplicate_exact_results_ambiguous() {
    let source = "First line.\nSecond line.\n";
    let file = std::path::PathBuf::from("docs/one.md");
    let snapshot = super::super::snapshots::snapshot_for_matching_tests(BTreeMap::from([(
        file,
        Ok(source.into()),
    )]))
    .unwrap();
    let case: LintCase = serde_json::from_value(json!({"id":"one","kind":"rule","rule":"known","file":"docs/one.md","revision":"fixture","lines":[1,1],"text_sha256":sunstack_evidence::content_digest(b"First line.\n"),"expected":"pass","set":"development","label_from":"blind-agents"})).unwrap();
    let exact: RulePieceResult = serde_json::from_value(json!({"text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001","file":"docs/one.md","first_line":1,"last_line":1,"outcome":"pass","step":null,"path":[]})).unwrap();
    let mut parent = exact.clone();
    parent.id = 8;
    parent.last_line = 2;
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.results = vec![exact.clone(), parent];

    let (outcome, id) = score(&case, &snapshot, &report);

    assert!(matches!(outcome, CaseOutcome::Right));
    assert_eq!(id, Some(7));
    let mut duplicate = exact;
    duplicate.id = 9;
    report.results[1] = duplicate;
    let (outcome, id) = score(&case, &snapshot, &report);
    assert!(matches!(outcome, CaseOutcome::MoreThanOneMatchingPiece));
    assert_eq!(id, None);
}
