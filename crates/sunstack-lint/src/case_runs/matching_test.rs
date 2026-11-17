#![allow(clippy::unwrap_used)]
use super::*;
use crate::{EffectiveLimits, LintReport, RulePieceResult, ScopeChoice};
use serde_json::json;
use std::collections::BTreeMap;

fn attach_result_evidence(report: &mut LintReport) {
    for result in &report.results {
        let piece = json!({
            "kind":"document", "where": {
                "file":result.file, "first_line":result.first_line, "last_line":result.last_line,
                "span":{"source":{"kind":"document","repository":"fixture","path":result.file,"content_identity":"fixture"},
                    "byte_start":0,"byte_end":12,"start_line":1,"start_column":1,"end_line":1,"end_column":12}
            }, "text":"First line.", "fields":{}, "model_view":{"state":"bounded","fields":{},"cut":[]}
        });
        report.piece_evidence.insert(
            result.id,
            serde_json::from_value(json!({"piece":piece,"matches":{}})).unwrap(),
        );
    }
}

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
    attach_result_evidence(&mut report);

    let (outcome, id) = score(&case, &snapshot, &report);

    assert!(matches!(outcome, CaseOutcome::Right));
    assert_eq!(id, Some(7));
    let mut duplicate = exact;
    duplicate.id = 9;
    report.results[1] = duplicate;
    attach_result_evidence(&mut report);
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
    attach_result_evidence(&mut report);
    assert!(matches!(
        prepare(&case, &snapshot, &report, None),
        (CaseOutcome::FilterExcluded, Some(7))
    ));
}

#[test]
fn selection_dependency_does_not_hide_a_known_result_on_an_independent_field() {
    let file = std::path::PathBuf::from("docs/one.md");
    let snapshot = super::super::snapshots::snapshot_for_matching_tests(BTreeMap::from([(
        file.clone(),
        Ok("First line.\n".into()),
    )]))
    .unwrap();
    let case:LintCase=serde_json::from_value(json!({"id":"known","kind":"rule","rule":"known","file":file,"revision":"fixture","lines":[1,1],"text_sha256":sunstack_evidence::content_digest(b"First line.\n"),"expected":"pass","set":"development","label_from":"blind-agents"})).unwrap();
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.results.push(serde_json::from_value(json!({"text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001","file":file,"first_line":1,"last_line":1,"outcome":"pass","step":null,"path":[]})).unwrap());
    attach_result_evidence(&mut report);
    let kind = sunstack_evidence::EvidenceKindName::new("document").unwrap();
    report.preparation = Some(crate::PreparedReport {
        mode: crate::PrepareMode::Prepare,
        needs_answer: vec![crate::NeedsSelectionAnswer {
            kind: kind.clone(),
            file: file.clone(),
            waits_on: crate::SelectionDependency {
                selection_kind: sunstack_evidence::EvidenceKindName::new("selected_sections")
                    .unwrap(),
                request_id: 1,
            },
        }],
    });
    let dependency = (kind, file);
    let (outcome, id) = prepare(&case, &snapshot, &report, Some(&dependency));
    assert!(
        matches!(outcome, CaseOutcome::Prepared),
        "a different field's pending selection hid the known mechanical result"
    );
    assert_eq!(id, Some(7));
    let mut duplicate = report.results[0].clone();
    duplicate.id = 8;
    report.results.push(duplicate);
    attach_result_evidence(&mut report);
    assert!(matches!(
        prepare(&case, &snapshot, &report, Some(&dependency)),
        (CaseOutcome::MoreThanOneMatchingPiece, None)
    ));
}

#[test]
fn cancelled_selected_case_keeps_completed_judgements_and_identifies_unfinished_execution() {
    // Arrange: the identity is selected, but execution can stop before or after its result.
    let file = std::path::PathBuf::from("docs/one.md");
    let snapshot = super::super::snapshots::snapshot_for_matching_tests(BTreeMap::from([(
        file.clone(),
        Ok("First line.\n".into()),
    )]))
    .unwrap();
    let case: LintCase = serde_json::from_value(json!({
        "id":"selected","kind":"rule","rule":"known","file":file,
        "revision":"fixture","lines":[1,1],
        "text_sha256":sunstack_evidence::content_digest(b"First line.\n"),
        "expected":"pass","set":"development","label_from":"blind-agents"
    }))
    .unwrap();
    let completed: RulePieceResult = serde_json::from_value(json!({
        "text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001",
        "file":file,"first_line":1,"last_line":1,"outcome":"pass","step":null,"path":[]
    }))
    .unwrap();
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.status = crate::LintStatus::Cancelled;

    // Act / Assert: a never-started owner is cancelled, not an identity mismatch, in either mode.
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, false),
        (CaseOutcome::Cancelled, None)
    ));
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, true),
        (CaseOutcome::Cancelled, None)
    ));
    report.results.push(completed);
    attach_result_evidence(&mut report);
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, false),
        (CaseOutcome::Right, Some(7))
    ));
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, true),
        (CaseOutcome::Prepared, Some(7))
    ));

    // Incomplete result evidence stays available for readback, without claiming a finished judgement.
    report.results[0].ending = crate::RulePieceEnding::Incomplete { gap: None };
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, false),
        (CaseOutcome::Cancelled, Some(7))
    ));
    report.status = crate::LintStatus::Incomplete;
    assert!(matches!(
        selected_result(&case, &snapshot, &report, None, false),
        (CaseOutcome::Incomplete { .. }, Some(7))
    ));
}

#[test]
fn settled_case_status_keeps_only_unique_exact_result_links() {
    // Arrange: an incomplete rule result and filter result share neither identity nor range.
    let rule_case: LintCase = serde_json::from_value(json!({
        "id":"rule-case","kind":"rule","rule":"known","file":"docs/one.md",
        "revision":"fixture","lines":[1,1],"text_sha256":"recorded",
        "expected":"pass","set":"development","label_from":"blind-agents"
    }))
    .unwrap();
    let filter_case: LintCase = serde_json::from_value(json!({
        "id":"filter-case","kind":"filter","filter":"purpose","folder":"docs",
        "revision":"fixture","reads_sha256":"recorded","expected_label":"eligible",
        "set":"development","label_from":"blind-agents"
    }))
    .unwrap();
    let rule_result: RulePieceResult = serde_json::from_value(json!({
        "text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001",
        "file":"docs/one.md","first_line":1,"last_line":1,"outcome":"incomplete","step":null,"path":[]
    })).unwrap();
    let filter_result: crate::IdentifiedResult<sunstack_rules::FilterAnswer> =
        serde_json::from_value(json!({
            "id":8,"request_ids":[],"filter":"purpose","folder":"docs","input_cut":false,
            "requested_model":"controlled","resolved_model":null,"state":"incomplete",
            "reason":{"kind":"reads_absent"},"path":[],"answers":{}
        }))
        .unwrap();
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.results.push(rule_result.clone());
    report.filter_answers.push(filter_result.clone());
    attach_result_evidence(&mut report);

    // Act / Assert: each settled typed status preserves the actual unique result ID.
    for case in [&rule_case, &filter_case] {
        let expected_id = if case.id == "rule-case" { 7 } else { 8 };
        for status in [
            crate::CaseBindingStatus::Incomplete {
                reason: crate::IncompleteReason::EvidenceUnavailable,
            },
            crate::CaseBindingStatus::Cancelled,
            crate::CaseBindingStatus::Unavailable {
                reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {},
            },
            crate::CaseBindingStatus::SelectionPending {
                evidence_kind: sunstack_evidence::EvidenceKindName::new("document").unwrap(),
                file: "docs/one.md".into(),
            },
        ] {
            report.case_selection = Some(crate::CaseSelectionReport {
                bindings: vec![crate::CaseBindingResult {
                    binding: case.binding(),
                    outcome: status.clone(),
                }],
                excluded: vec![],
            });
            let (outcome, result_id) = selection_outcome(case, &report).unwrap();
            assert_eq!(result_id, Some(expected_id));
            match status {
                crate::CaseBindingStatus::Incomplete { .. } => {
                    assert!(matches!(outcome, CaseOutcome::Incomplete { .. }))
                }
                crate::CaseBindingStatus::Cancelled => {
                    assert!(matches!(outcome, CaseOutcome::Cancelled))
                }
                crate::CaseBindingStatus::Unavailable { reason } => {
                    assert!(
                        matches!(outcome, CaseOutcome::Unavailable { reason: actual } if actual == reason)
                    )
                }
                _ => assert!(matches!(outcome, CaseOutcome::SelectionPending { .. })),
            }
        }
    }

    // Duplicate exact owners remain ambiguous to the result-link selector.
    report.results.push(rule_result);
    report.filter_answers.push(filter_result);
    attach_result_evidence(&mut report);
    assert_eq!(unique_result_id(&rule_case, &report), None);
    assert_eq!(unique_result_id(&filter_case, &report), None);
    report.results.clear();
    report.filter_answers.clear();
    assert_eq!(unique_result_id(&rule_case, &report), None);
    assert_eq!(unique_result_id(&filter_case, &report), None);
}

#[test]
fn refused_or_pending_identity_never_links_an_unrelated_prerequisite_request() {
    // Arrange: a prepared evidence selection exists, but no rule result exists.
    let case: LintCase = serde_json::from_value(json!({
        "id":"one","kind":"rule","rule":"known","file":"docs/one.md","revision":"fixture",
        "lines":[1,1],"text_sha256":"recorded","expected":"pass","set":"development","label_from":"blind-agents"
    })).unwrap();
    let request: crate::SavedRequest = serde_json::from_value(json!({
        "id":42,"owner":{"kind":"selection","selection_kind":"selected_context","file":"docs/one.md"},
        "requested_model":"controlled","resolved_model":null,"request":{"state":{},"questions":{}},
        "state":"prepared","status":"prepared"
    })).unwrap();
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.requests.push(request);
    for status in [
        crate::CaseBindingStatus::Unavailable {
            reason: sunstack_rules::WorkUnavailableReason::SelectionUnavailable {
                evidence_kind: sunstack_evidence::EvidenceKindName::new("document").unwrap(),
                file: "docs/one.md".into(),
            },
        },
        crate::CaseBindingStatus::SelectionPending {
            evidence_kind: sunstack_evidence::EvidenceKindName::new("document").unwrap(),
            file: "docs/one.md".into(),
        },
        crate::CaseBindingStatus::Unmatched,
        crate::CaseBindingStatus::Stale,
        crate::CaseBindingStatus::Ambiguous { owners: vec![] },
    ] {
        report.case_selection = Some(crate::CaseSelectionReport {
            bindings: vec![crate::CaseBindingResult {
                binding: case.binding(),
                outcome: status,
            }],
            excluded: vec![],
        });
        // Act / Assert: a request ID cannot stand in for missing result evidence.
        assert_eq!(selection_outcome(&case, &report).unwrap().1, None);
    }

    // Even an exact result cannot override refused identity accounting.
    report.results.push(serde_json::from_value(json!({
        "text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001",
        "file":"docs/one.md","first_line":1,"last_line":1,"outcome":"incomplete","step":null,"path":[]
    })).unwrap());
    for status in [
        crate::CaseBindingStatus::Unmatched,
        crate::CaseBindingStatus::Stale,
        crate::CaseBindingStatus::Ambiguous { owners: vec![] },
    ] {
        report.case_selection.as_mut().unwrap().bindings[0].outcome = status;
        assert_eq!(selection_outcome(&case, &report).unwrap().1, None);
    }
}

#[test]
fn missing_same_line_piece_evidence_never_borrows_a_unique_result() {
    let file = std::path::PathBuf::from("docs/one.md");
    let snapshot = super::super::snapshots::snapshot_for_matching_tests(BTreeMap::from([(
        file,
        Ok("First line.\n".into()),
    )]))
    .unwrap();
    let case: LintCase = serde_json::from_value(json!({"id":"one","kind":"rule","rule":"known",
        "file":"docs/one.md","revision":"fixture","lines":[1,1],"bytes":[0,5],
        "text_sha256":sunstack_evidence::content_digest(b"First"),"expected":"pass",
        "set":"development","label_from":"blind-agents"}))
    .unwrap();
    let mut report = LintReport::empty(
        Default::default(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    let result: RulePieceResult = serde_json::from_value(json!({"text_sha256":null,"id":7,
        "request_ids":[],"name":"known","marker":"K001","file":"docs/one.md",
        "first_line":1,"last_line":1,"outcome":"pass","step":null,"path":[]}))
    .unwrap();
    report.results.push(result.clone());
    attach_result_evidence(&mut report);
    report
        .piece_evidence
        .get_mut(&7)
        .unwrap()
        .piece
        .location
        .span
        .byte_end = 5;
    let mut unknown = result;
    unknown.id = 8;
    report.results.push(unknown);
    assert!(matches!(
        score(&case, &snapshot, &report),
        (CaseOutcome::Incomplete { .. }, None)
    ));
    assert_eq!(unique_result_id(&case, &report), None);
}
