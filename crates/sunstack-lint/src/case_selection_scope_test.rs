use super::*;

fn record_complete_rule_file(catalog: &mut EvidenceCatalog, rule: &RulebookRule, file: &Path) {
    catalog.file_counts.push(sunstack_evidence::FileCount {
        file: file.into(),
        kinds: std::collections::BTreeMap::from([(
            rule.kind.clone(),
            sunstack_evidence::KindCount::default(),
        )]),
    });
}

#[test]
fn captured_missing_owner_is_not_hidden_by_unrelated_global_incomplete() {
    // Arrange: this admitted rule's file and matching source digest are complete.
    let (rule, piece, mut binding, mut catalog) = rule_piece_fixture();
    if let CaseSubjectBinding::Rule { lines, .. } = &mut binding.subject {
        *lines = [2, 2];
    }
    catalog
        .piece_text_hashes
        .insert((piece.location.file.clone(), 2, 2), "digest".into());
    record_complete_rule_file(&mut catalog, &rule, &piece.location.file);
    for reason in [
        IncompleteReason::CoveragePartial,
        IncompleteReason::SelectionFailed,
        IncompleteReason::SelectionIncomplete,
    ] {
        // Act: another definition/file is incomplete; it cannot supply this owner.
        let incomplete = IncompletePart {
            field: None,
            rule: None,
            file: None,
            reason,
        };
        let plan = plan_case_selection(
            std::slice::from_ref(&binding),
            CaseSelectionInputs {
                rules: std::slice::from_ref(&rule),
                pieces: std::slice::from_ref(&piece),
                incomplete: &[incomplete],
                ..inputs(&catalog, &FilterRegistry::default())
            },
        );
        // Assert: retain the requested ID and a conclusive unmatched identity.
        assert_eq!(plan.report.bindings.len(), 1);
        assert_eq!(plan.report.bindings[0].binding.id, binding.id);
        assert!(matches!(
            plan.report.bindings[0].outcome,
            CaseBindingStatus::Unmatched
        ));
        assert!(plan.allowed_rule_pieces.is_empty());
        assert!(plan.allowed_filters.is_empty());
    }
}

#[test]
fn captured_stale_range_is_not_hidden_by_unrelated_global_incomplete() {
    // Arrange: no piece owns the recorded lines, whose captured source changed.
    let (rule, piece, mut binding, mut catalog) = rule_piece_fixture();
    if let CaseSubjectBinding::Rule { lines, .. } = &mut binding.subject {
        *lines = [2, 2];
    }
    record_complete_rule_file(&mut catalog, &rule, &piece.location.file);
    let incomplete = IncompletePart {
        field: None,
        rule: None,
        file: None,
        reason: IncompleteReason::CoveragePartial,
    };
    for captured_hash in [Some("changed"), None] {
        if let Some(hash) = captured_hash {
            catalog
                .piece_text_hashes
                .insert((piece.location.file.clone(), 2, 2), hash.into());
        } else {
            catalog
                .piece_text_hashes
                .remove(&(piece.location.file.clone(), 2, 2));
        }
        // Act / Assert: both changed text and an invalid captured range stay stale.
        let plan = plan_case_selection(
            std::slice::from_ref(&binding),
            CaseSelectionInputs {
                rules: std::slice::from_ref(&rule),
                pieces: std::slice::from_ref(&piece),
                incomplete: std::slice::from_ref(&incomplete),
                ..inputs(&catalog, &FilterRegistry::default())
            },
        );
        assert!(matches!(
            plan.report.bindings[0].outcome,
            CaseBindingStatus::Stale
        ));
        assert!(plan.allowed_rule_pieces.is_empty());
    }
}

#[test]
fn unresolved_or_failed_capture_keeps_applicable_incompleteness() {
    for scenario in [
        "unacquired",
        "unknown-rule",
        "missing-counts",
        "limited",
        "failed-file",
        "failed-parent",
        "uncertain-parent",
        "unsettled-parent",
        "unknown-global",
        "pending",
    ] {
        let (rule, piece, mut binding, mut catalog) = rule_piece_fixture();
        if let CaseSubjectBinding::Rule { lines, .. } = &mut binding.subject {
            *lines = [2, 2];
        }
        let file = piece.location.file.clone();
        catalog
            .piece_text_hashes
            .insert((file.clone(), 2, 2), "digest".into());
        record_complete_rule_file(&mut catalog, &rule, &file);
        let mut incomplete = vec![IncompletePart {
            field: None,
            rule: None,
            file: None,
            reason: IncompleteReason::CoveragePartial,
        }];
        let mut pending = vec![];
        let mut rules = vec![rule.clone()];
        match scenario {
            "unacquired" => {
                catalog.matches.clear();
                catalog.file_counts.clear();
                catalog.piece_text_hashes.clear();
                incomplete[0].reason = IncompleteReason::SelectionFailed;
            }
            "unknown-rule" => rules.clear(),
            "missing-counts" => catalog.file_counts.clear(),
            "limited" => {
                catalog.file_counts[0]
                    .kinds
                    .get_mut(&rule.kind)
                    .unwrap()
                    .limited = true
            }
            "failed-file" => {
                catalog
                    .failures
                    .push(sunstack_evidence::EvidenceFileFailure {
                        file: file.clone(),
                        reason: "max_matches".into(),
                    });
                incomplete.push(IncompletePart {
                    field: None,
                    rule: None,
                    file: Some(file.clone()),
                    reason: IncompleteReason::SelectionFailed,
                });
            }
            "failed-parent" | "uncertain-parent" | "unsettled-parent" => {
                catalog.file_counts[0].kinds.insert(
                    EvidenceKindName::new("selected_parent").unwrap(),
                    sunstack_evidence::KindCount {
                        selection: Some(sunstack_evidence::SelectionCount {
                            candidates: 1,
                            asked: usize::from(scenario != "unsettled-parent"),
                            incomplete: usize::from(scenario == "failed-parent"),
                            uncertain: usize::from(scenario == "uncertain-parent"),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                );
                incomplete[0].reason = IncompleteReason::SelectionIncomplete;
            }
            "unknown-global" => incomplete[0].reason = IncompleteReason::EvaluationFailed,
            "pending" => pending.push(NeedsSelectionAnswer {
                kind: rule.kind.clone(),
                file: file.clone(),
                waits_on: crate::SelectionDependency {
                    selection_kind: rule.kind.clone(),
                    request_id: 7,
                },
            }),
            _ => unreachable!(),
        }
        let plan = plan_case_selection(
            std::slice::from_ref(&binding),
            CaseSelectionInputs {
                rules: &rules,
                pieces: std::slice::from_ref(&piece),
                incomplete: &incomplete,
                needs_answer: &pending,
                ..inputs(&catalog, &FilterRegistry::default())
            },
        );
        if scenario == "pending" {
            assert!(matches!(
                plan.report.bindings[0].outcome,
                CaseBindingStatus::SelectionPending { .. }
            ));
        } else {
            assert!(
                matches!(
                    plan.report.bindings[0].outcome,
                    CaseBindingStatus::Incomplete { .. }
                ),
                "{scenario}"
            );
        }
        assert_eq!(plan.report.bindings[0].binding.id, binding.id);
        assert!(plan.allowed_rule_pieces.is_empty());
    }
}

#[test]
fn captured_digest_mismatch_proves_stale_even_when_matching_failed() {
    let (rule, piece, binding, mut catalog) = rule_piece_fixture();
    catalog
        .piece_text_hashes
        .insert((piece.location.file.clone(), 1, 1), "changed".into());
    catalog
        .failures
        .push(sunstack_evidence::EvidenceFileFailure {
            file: piece.location.file.clone(),
            reason: "max_matches".into(),
        });
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            rules: &[rule],
            incomplete: &[IncompletePart {
                field: None,
                rule: None,
                file: None,
                reason: IncompleteReason::CoveragePartial,
            }],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(matches!(
        plan.report.bindings[0].outcome,
        CaseBindingStatus::Stale
    ));
    assert!(plan.allowed_rule_pieces.is_empty());
}

#[test]
fn missing_filter_folder_does_not_invent_read_file_completeness() {
    let binding = filter_binding("filter");
    let mut catalog = EvidenceCatalog::default();
    catalog.matches.insert("docs/a".into(), Default::default());
    let plan = plan_case_selection(
        &[binding],
        CaseSelectionInputs {
            incomplete: &[IncompletePart {
                field: None,
                rule: None,
                file: None,
                reason: IncompleteReason::CoveragePartial,
            }],
            ..inputs(&catalog, &FilterRegistry::default())
        },
    );
    assert!(matches!(
        plan.report.bindings[0].outcome,
        CaseBindingStatus::Incomplete { .. }
    ));
    assert!(plan.allowed_filters.is_empty());
}
