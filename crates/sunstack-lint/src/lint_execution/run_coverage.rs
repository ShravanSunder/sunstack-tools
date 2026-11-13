use super::{
    filter_folders::FilterFolderPlan, lint_run::RunState, run_evidence::BuiltEvidence,
    run_scope::ScopeInputs,
};
use crate::*;
use std::ops::ControlFlow;
use sunstack_evidence::*;
use sunstack_rules::{ModelRequestDispatch, RulebookRule, SelectionOutcome};

pub(super) type PickedEvidence = (
    EvidenceCatalog,
    Vec<EvidencePiece>,
    FilterFolderPlan,
    ModelRequestDispatch,
    usize,
);

pub(super) fn pick_run_pieces(
    mut state: RunState,
    rules: &[RulebookRule],
    scope: &ScopeInputs,
    evidence: BuiltEvidence,
    source: &FrozenCheckedSource,
) -> ControlFlow<LintReport, (RunState, PickedEvidence)> {
    let (mut catalog, checked_files, folder_plan, dispatcher) = evidence;
    let mut report = state.report;
    let cancellation = &state.cancellation;
    crate::lint_reporting::outcome::project_selections(&mut catalog, &mut report);
    let acquired = source.checked_files_acquired();
    report.usage.checked_files_acquired = acquired;
    crate::lint_reporting::outcome::project_evidence_failures(&catalog, &mut report);
    let (picked, _) = pick_pieces(
        &catalog.pieces,
        &scope.1,
        scope.3,
        &scope.0.root,
        &folder_plan.dependencies,
    );
    let picked = picked
        .into_iter()
        .filter(|index| checked_files.contains(&catalog.pieces[*index].location.file))
        .collect::<Vec<_>>();
    let picked_set = picked
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for count in &mut catalog.file_counts {
        for (kind, kind_count) in &mut count.kinds {
            kind_count.picked = catalog
                .pieces
                .iter()
                .enumerate()
                .filter(|(index, piece)| {
                    picked_set.contains(index)
                        && piece.location.file == count.file
                        && &piece.kind == kind
                })
                .count();
        }
    }
    report.file_counts = catalog
        .file_counts
        .iter()
        .filter(|count| checked_files.contains(&count.file))
        .cloned()
        .collect();
    let coverage_kinds = match &state.request.evidence_listing {
        Some(selected_kind) => catalog
            .file_counts
            .iter()
            .flat_map(|count| count.kinds.keys())
            .filter(|kind| {
                selected_kind
                    .as_ref()
                    .is_none_or(|selected| selected == *kind)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>(),
        None => rules.iter().map(|rule| &rule.kind).collect(),
    };
    let outside = coverage_kinds
        .iter()
        .map(|kind| {
            catalog
                .pieces
                .iter()
                .enumerate()
                .filter(|(index, piece)| {
                    checked_files.contains(&piece.location.file)
                        && &piece.kind == *kind
                        && !picked_set.contains(index)
                })
                .count()
        })
        .sum::<usize>();
    report.exclusions.outside_change_scope = u32::try_from(outside).unwrap_or(u32::MAX);
    let zero_matches = coverage_kinds
        .iter()
        .map(|kind| {
            catalog
                .matches
                .keys()
                .filter(|file| checked_files.contains(*file))
                .filter(|file| {
                    !catalog
                        .pieces
                        .iter()
                        .any(|piece| &piece.kind == *kind && &piece.location.file == *file)
                })
                .count()
        })
        .sum::<usize>();
    report.exclusions.zero_matches = u32::try_from(zero_matches).unwrap_or(u32::MAX);
    let pieces = picked
        .into_iter()
        .filter_map(|index| catalog.pieces.get(index).cloned())
        .collect::<Vec<_>>();
    if let Some(kind) = state.request.evidence_listing.take() {
        let pieces = pieces
            .into_iter()
            .filter(|piece| kind.as_ref().is_none_or(|kind| kind == &piece.kind))
            .collect();
        report.usage = crate::lint_reporting::outcome::selection_usage(&dispatcher, acquired);
        report.status = if cancellation.is_cancelled() {
            LintStatus::Cancelled
        } else if !report.incomplete.is_empty() {
            LintStatus::Incomplete
        } else if report
            .selections
            .iter()
            .any(|entry| matches!(entry.outcome, SelectionOutcome::Uncertain { .. }))
        {
            LintStatus::Uncertain
        } else {
            LintStatus::Clean
        };
        return ControlFlow::Break(report.with_evidence_listing(pieces));
    }
    state.report = report;
    ControlFlow::Continue((state, (catalog, pieces, folder_plan, dispatcher, acquired)))
}
