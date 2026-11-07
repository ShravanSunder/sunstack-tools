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

#[test]
fn prepared_matching_keeps_duplicate_exact_requests_ambiguous_and_known_exclusion_explicit() {
    // Arrange: the matcher receives saved requests; no provider or verdict is invented here.
    let file = std::path::PathBuf::from("docs/one.md");
    let snapshot = super::super::snapshots::snapshot_for_matching_tests(BTreeMap::from([(
        file.clone(),
        Ok("First line.\nSecond line.\n".into()),
    )]))
    .unwrap();
    let case: LintCase = serde_json::from_value(json!({"id":"one","kind":"rule","rule":"known","file":file,"revision":"fixture","lines":[1,1],"text_sha256":sunstack_evidence::content_digest(b"First line.\n"),"expected":"pass","set":"development","label_from":"blind-agents"})).unwrap();
    let exact: crate::SavedRequest = serde_json::from_value(json!({
        "id":1,"owner":{"kind":"rule","rule":"known","piece":{"file":file,"first_line":1,"last_line":1,
        "span":{"source":{"kind":"document","repository":"fixture","path":file,"content_identity":"fixture"},
            "byte_start":0,"byte_end":12,"start_line":1,"start_column":1,"end_line":1,"end_column":12}}},
        "requested_model":"controlled","resolved_model":null,"request":{"state":"First line.","questions":{}},
        "state":"prepared","status":"prepared"})).unwrap();
    let mut parent = exact.clone();
    parent.id = 2;
    if let crate::RequestOwner::Rule { piece, .. } = &mut parent.owner {
        piece.last_line = 2;
    }
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.requests = vec![exact.clone(), parent];

    // Act / Assert: the parent is distinct, but a second exact request cannot be chosen arbitrarily.
    assert!(matches!(
        prepare(&case, &snapshot, &report, None),
        (CaseOutcome::Prepared, Some(1))
    ));
    let mut duplicate = exact;
    duplicate.id = 3;
    report.requests[1] = duplicate;
    assert!(matches!(
        prepare(&case, &snapshot, &report, None),
        (CaseOutcome::MoreThanOneMatchingPiece, None)
    ));

    // A known exclusion is preserved by matching; this is not offline filter execution.
    report.requests.clear();
    report.results = vec![serde_json::from_value(json!({"text_sha256":null,"id":7,"request_ids":[],
        "name":"known","marker":"K001","file":file,"first_line":1,"last_line":1,
        "outcome":"not_applicable","step":null,"path":[],"reason":{"kind":"filter","filter":"purpose","label":"excluded"}})).unwrap()];
    assert!(matches!(
        prepare(&case, &snapshot, &report, None),
        (CaseOutcome::FilterExcluded, Some(7))
    ));
}
