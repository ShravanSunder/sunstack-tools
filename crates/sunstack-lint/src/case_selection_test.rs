#![allow(clippy::unwrap_used)]
use super::*;

fn inputs<'a>(
    catalog: &'a EvidenceCatalog,
    filters: &'a FilterRegistry,
) -> CaseSelectionInputs<'a> {
    CaseSelectionInputs {
        rules: &[],
        pieces: &[],
        catalog,
        folders: &[],
        filters,
        needs_answer: &[],
        incomplete: &[],
        cancelled: false,
    }
}

fn filter_binding(id: &str) -> CaseBinding {
    CaseBinding {
        id: id.into(),
        subject: CaseSubjectBinding::Filter {
            filter: FilterName::new("purpose").unwrap(),
            folder: "docs/a".into(),
            reads_sha256: "digest".into(),
        },
    }
}

#[test]
fn duplicate_ids_are_rejected_but_distinct_ids_may_share_an_owner() {
    assert!(validate_case_bindings(&[filter_binding("one"), filter_binding("one")]).is_err());
    assert!(validate_case_bindings(&[filter_binding("one"), filter_binding("two")]).is_ok());
}

#[test]
fn empty_selection_has_no_allowed_owners() {
    let plan = plan_case_selection(
        &[],
        inputs(&EvidenceCatalog::default(), &FilterRegistry::default()),
    );
    assert!(plan.report.bindings.is_empty());
    assert!(plan.allowed_rule_pieces.is_empty());
    assert!(plan.allowed_filters.is_empty());
}

#[test]
fn missing_filter_and_cancelled_binding_remain_in_accounting() {
    let binding = filter_binding("one");
    let catalog = EvidenceCatalog::default();
    let filters = FilterRegistry::default();
    let missing = plan_case_selection(std::slice::from_ref(&binding), inputs(&catalog, &filters));
    assert!(matches!(
        missing.report.bindings[0].outcome,
        CaseBindingStatus::Unmatched
    ));
    let cancelled = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            cancelled: true,
            ..inputs(&catalog, &filters)
        },
    );
    assert!(matches!(
        cancelled.report.bindings[0].outcome,
        CaseBindingStatus::Cancelled
    ));
}

#[test]
fn duplicate_filter_candidates_are_ambiguous_instead_of_first_match() {
    let binding = filter_binding("one");
    let candidate = FilterFolder {
        filter: FilterName::new("purpose").unwrap(),
        folder: "docs/a".into(),
        members: vec![],
        reads: ReadsPiece::None,
    };
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            folders: &[candidate.clone(), candidate],
            ..inputs(&EvidenceCatalog::default(), &FilterRegistry::default())
        },
    );
    assert!(
        matches!(&plan.report.bindings[0].outcome, CaseBindingStatus::Ambiguous { owners } if owners.len() == 2)
    );
    assert!(plan.allowed_filters.is_empty());
    assert_eq!(plan.report.excluded.len(), 2);
}

#[test]
fn missing_rule_with_pending_dependency_is_not_unmatched() {
    let kind = EvidenceKindName::new("section").unwrap();
    let file = PathBuf::from("docs/a.md");
    let pending = NeedsSelectionAnswer {
        kind: kind.clone(),
        file: file.clone(),
        waits_on: crate::SelectionDependency {
            selection_kind: kind.clone(),
            request_id: 7,
        },
    };
    let status = missing_status(
        &file,
        Some(&kind),
        &EvidenceCatalog::default(),
        &[pending],
        &[],
    );
    assert!(
        matches!(status, CaseBindingStatus::SelectionPending { evidence_kind: actual, file: actual_file } if actual == kind && actual_file == file)
    );
}

#[test]
fn acquired_range_digest_distinguishes_unmatched_from_stale() {
    let file = PathBuf::from("docs/a.md");
    let binding = CaseBinding {
        id: "rule-case".into(),
        subject: CaseSubjectBinding::Rule {
            bytes: None,
            rule: RuleId::new("rule").unwrap(),
            file: file.clone(),
            lines: [2, 3],
            text_sha256: "expected".into(),
        },
    };
    let mut catalog = EvidenceCatalog::default();
    catalog.matches.insert(file.clone(), Default::default());
    let run = |catalog: &EvidenceCatalog| {
        plan_case_selection(
            std::slice::from_ref(&binding),
            inputs(catalog, &FilterRegistry::default()),
        )
        .report
        .bindings
        .remove(0)
        .outcome
    };
    assert!(matches!(run(&catalog), CaseBindingStatus::Stale));
    catalog
        .piece_text_hashes
        .insert((file.clone(), 2, 3), "different".into());
    assert!(matches!(run(&catalog), CaseBindingStatus::Stale));
    catalog
        .piece_text_hashes
        .insert((file, 2, 3), "expected".into());
    assert!(matches!(run(&catalog), CaseBindingStatus::Unmatched));
}

#[test]
fn admission_rejects_blank_ids_and_escaping_paths() {
    assert!(validate_case_bindings(&[filter_binding("  ")]).is_err());
    for path in ["../outside", "/absolute/path", "docs/../outside"] {
        let mut binding = filter_binding("one");
        if let CaseSubjectBinding::Filter { folder, .. } = &mut binding.subject {
            *folder = path.into();
        }
        assert!(validate_case_bindings(&[binding]).is_err(), "{path}");
    }
    assert!(validate_case_bindings(&[filter_binding("one")]).is_ok());
}

#[test]
fn wire_binding_rejects_expected_labels_and_unknown_fields() {
    let valid = r#"{"id":"one","kind":"filter","filter":"purpose","folder":"docs/a","reads_sha256":"digest"}"#;
    assert!(serde_json::from_str::<CaseBinding>(valid).is_ok());
    for extra in [r#", "expected_label":"yes"}"#, r#", "unknown":true}"#] {
        let invalid = format!("{}{}", &valid[..valid.len() - 1], extra);
        assert!(serde_json::from_str::<CaseBinding>(&invalid).is_err());
    }
    let rule = r#"{"id":"two","kind":"rule","rule":"example","file":"docs/a.md","lines":[1,1],"text_sha256":"digest","expected":"pass"}"#;
    assert!(serde_json::from_str::<CaseBinding>(rule).is_err());
}

fn rule_piece_fixture() -> (RulebookRule, EvidencePiece, CaseBinding, EvidenceCatalog) {
    use sunstack_evidence::{
        DefinitionOrigin, MarkerId, ModelView, PieceLocation, RepositoryId, SourceRef, SourceSpan,
    };
    use sunstack_rules::{RuleEntryWire, RulebookSentence};
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sunstack/tests/fixtures/rulebook-lint");
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let project = sunstack_evidence::load_evidence_project(
        &root.join("evidence/sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let entries: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("rulebook/steering.rules.json")).unwrap(),
    )
    .unwrap();
    let entry: RuleEntryWire = serde_json::from_value(entries["K001"].clone()).unwrap();
    let graph =
        sunstack_rules::admit_rule_entry(&entry, &kinds, &project, &Default::default()).unwrap();
    let rule = RulebookRule {
        reference: entry.reference,
        gate: None,
        name: entry.name,
        sentence: RulebookSentence {
            marker: MarkerId::new("K001").unwrap(),
            rulebook: DefinitionOrigin::file("rulebook.md"),
            line: 1,
            standard: "A checkable completion covers its obligations.".into(),
        },
        kind: entry.evidence,
        questions: entry.questions,
        decision: graph,
        severity: entry.severity,
        related: entry.related,
    };
    let file = PathBuf::from("SKILL.md");
    let span = SourceSpan {
        source: SourceRef::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: file.clone(),
            content_identity: "fixture".into(),
        },
        byte_start: 0,
        byte_end: 4,
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 5,
    };
    let piece = EvidencePiece {
        kind: rule.kind.clone(),
        location: PieceLocation {
            file: file.clone(),
            first_line: 1,
            last_line: 1,
            span,
        },
        text: "test".into(),
        fields: Default::default(),
        model_view: ModelView::Bounded {
            fields: Default::default(),
            cut: vec![],
        },
    };
    let binding = CaseBinding {
        id: "one".into(),
        subject: CaseSubjectBinding::Rule {
            bytes: None,
            rule: rule.name.clone(),
            file: file.clone(),
            lines: [1, 1],
            text_sha256: "digest".into(),
        },
    };
    let mut catalog = EvidenceCatalog::default();
    catalog.matches.insert(file.clone(), Default::default());
    catalog
        .piece_text_hashes
        .insert((file, 1, 1), "digest".into());
    (rule, piece, binding, catalog)
}

#[test]
fn known_rule_owner_survives_unrelated_global_incomplete_and_deduplicates_ids() {
    let (rule, piece, binding, catalog) = rule_piece_fixture();
    let mut second = binding.clone();
    second.id = "two".into();
    let incomplete = IncompletePart {
        field: None,
        rule: None,
        file: None,
        reason: IncompleteReason::CoveragePartial,
    };
    let plan = plan_case_selection(
        &[binding, second],
        CaseSelectionInputs {
            rules: &[rule],
            pieces: &[piece],
            incomplete: &[incomplete],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(
        plan.report
            .bindings
            .iter()
            .all(|entry| matches!(entry.outcome, CaseBindingStatus::Selected { .. }))
    );
    assert_eq!(plan.allowed_rule_pieces, BTreeSet::from([(0, 0)]));
    assert!(plan.report.excluded.is_empty());
}

#[test]
fn duplicate_exact_rule_pieces_are_ambiguous() {
    let (rule, piece, binding, catalog) = rule_piece_fixture();
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            rules: &[rule],
            pieces: &[piece.clone(), piece],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(
        matches!(&plan.report.bindings[0].outcome, CaseBindingStatus::Ambiguous { owners } if owners.len() == 2)
    );
    assert!(plan.allowed_rule_pieces.is_empty());
}

#[test]
fn pending_existing_piece_is_left_for_the_rule_walk_to_settle() {
    let (rule, mut piece, binding, catalog) = rule_piece_fixture();
    piece.model_view = sunstack_evidence::ModelView::SelectionPending;
    let pending = NeedsSelectionAnswer {
        kind: rule.kind.clone(),
        file: piece.location.file.clone(),
        waits_on: crate::SelectionDependency {
            selection_kind: rule.kind.clone(),
            request_id: 7,
        },
    };
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            rules: &[rule],
            pieces: &[piece],
            needs_answer: &[pending],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(matches!(
        plan.report.bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));
    assert_eq!(plan.allowed_rule_pieces, BTreeSet::from([(0, 0)]));
}

#[test]
fn missing_rule_piece_with_pending_dependency_remains_pending() {
    let (rule, piece, binding, catalog) = rule_piece_fixture();
    let pending = NeedsSelectionAnswer {
        kind: rule.kind.clone(),
        file: piece.location.file.clone(),
        waits_on: crate::SelectionDependency {
            selection_kind: rule.kind.clone(),
            request_id: 7,
        },
    };
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            rules: &[rule],
            needs_answer: &[pending],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(matches!(
        plan.report.bindings[0].outcome,
        CaseBindingStatus::SelectionPending { .. }
    ));
    assert!(plan.allowed_rule_pieces.is_empty());
}

#[path = "case_selection_scope_test.rs"]
mod scope_tests;

#[test]
fn unavailable_selection_keeps_unknown_binding_and_real_staleness_distinct() {
    // Arrange: the historical source is captured, but the selected kind has no final piece.
    let (rule, piece, binding, mut catalog) = rule_piece_fixture();
    let scope = IncompletePart {
        field: None,
        rule: None,
        file: Some(piece.location.file.clone()),
        reason: IncompleteReason::SelectionUnavailable {
            evidence_kind: rule.kind.clone(),
            file: piece.location.file.clone(),
            selection_kind: EvidenceKindName::new("selector").unwrap(),
        },
    };
    let filters = FilterRegistry::default();
    // Act: unavailable coverage takes priority over absence, not over actual digest mismatch.
    let plan = plan_case_selection(
        std::slice::from_ref(&binding),
        CaseSelectionInputs {
            rules: std::slice::from_ref(&rule),
            incomplete: std::slice::from_ref(&scope),
            ..inputs(&catalog, &filters)
        },
    );
    // Assert: retain the selected case ID without inventing a piece owner.
    assert_eq!(plan.report.bindings[0].binding.id, binding.id);
    assert!(
        matches!(&plan.report.bindings[0].outcome, CaseBindingStatus::Unavailable {
        reason: sunstack_rules::WorkUnavailableReason::SelectionUnavailable { evidence_kind, file }
    } if evidence_kind == &rule.kind && file == &piece.location.file)
    );
    assert!(plan.allowed_rule_pieces.is_empty());
    catalog
        .piece_text_hashes
        .insert((piece.location.file.clone(), 1, 1), "changed".into());
    let stale = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            rules: &[rule],
            incomplete: &[scope],
            ..inputs(&catalog, &filters)
        },
    );
    assert!(matches!(
        stale.report.bindings[0].outcome,
        CaseBindingStatus::Stale
    ));
}

#[test]
fn unrelated_unavailable_kind_does_not_change_a_missing_binding_disposition() {
    // Arrange: the same file has unavailable coverage for a different evidence kind.
    let file = PathBuf::from("docs/one.md");
    let kind = EvidenceKindName::new("known").unwrap();
    let scope = IncompletePart {
        field: None,
        rule: None,
        file: Some(file.clone()),
        reason: IncompleteReason::SelectionUnavailable {
            evidence_kind: EvidenceKindName::new("other").unwrap(),
            file: file.clone(),
            selection_kind: EvidenceKindName::new("selector").unwrap(),
        },
    };
    // Act / Assert: availability follows kind/file identity, not only the file name.
    let status = missing_status(
        &file,
        Some(&kind),
        &EvidenceCatalog::default(),
        &[],
        &[scope],
    );
    assert!(matches!(status, CaseBindingStatus::Unmatched));
}
