use super::backend_factory::backend_selection;
use super::{
    filter_folders::FilterFolderPlan,
    lint_run::{RunEnd, RunState, failed, invalid},
    run_scope::ScopeInputs,
    stopped_stages::{RunStage, RunStageStopped},
};
use crate::{
    configuration_loading::{
        config_validation::CheckedRulebook, definition_bundle::DefinitionBundle,
    },
    *,
};
use std::{collections::BTreeSet, ops::ControlFlow, path::PathBuf, sync::Arc};
use sunstack_decisions::DecisionBackend;
use sunstack_evidence::*;
use sunstack_rules::{
    ModelRequestDispatch, RulebookRunSettings, SelectionInput, SelectionOutcome, select_candidates,
};

pub(super) type BuiltEvidence = (
    EvidenceCatalog,
    BTreeSet<PathBuf>,
    FilterFolderPlan,
    ModelRequestDispatch,
);

pub(super) async fn build_run_evidence(
    mut state: RunState,
    bundle: &DefinitionBundle,
    checked: &mut CheckedRulebook,
    scope: &mut ScopeInputs,
    source: Option<Box<dyn CheckedFileSource + Send>>,
    backend: Option<Arc<dyn DecisionBackend>>,
) -> ControlFlow<RunEnd, (RunState, BuiltEvidence, FrozenCheckedSource)> {
    let request = &state.request;
    let cancellation = &state.cancellation;
    let preparing = state.preparing;
    let recorded_selections = std::mem::take(&mut state.recorded_selections);
    let mut report = state.report;
    let (selection, changes, frozen, full) = scope;
    let full = *full;
    let mut files = std::mem::take(&mut selection.files);
    if files.len() > report.limits.max_files as usize {
        files.truncate(report.limits.max_files as usize);
        report.diagnostics.push(LintDiagnostic {
            definition: None,
            json_path: vec![],
            file: None,
            path: None,
            line: None,
            column: None,
            code: LintInvalidCode::Syntax,
            message: "max_files".into(),
        });
        report.incomplete.push(IncompletePart {
            rule: None,
            file: None,
            field: None,
            reason: IncompleteReason::SelectionFailed,
        });
    }
    let fallback = match source {
        Some(source) => source,
        None => match FilesystemCheckedSource::new(&selection.root) {
            Ok(source) => Box::new(source),
            Err(_) => return ControlFlow::Break(Ok(failed(report, "checked source unavailable"))),
        },
    };
    let mut source = FrozenCheckedSource::new(std::mem::take(frozen), fallback);
    let checked_files = files
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let folder_plan = if request.evidence_listing.is_none() {
        super::filter_folders::resolve_filter_folders(
            &checked.filters,
            &checked.assembly.rules,
            &files,
        )
    } else {
        super::filter_folders::FilterFolderPlan::default()
    };
    let kinds = bundle.settings.kinds.clone();
    let limits = PieceBuildLimits {
        max_files: report.limits.max_files as usize,
        max_file_bytes: match usize::try_from(report.limits.max_file_bytes) {
            Ok(value) => value,
            Err(_) => {
                return ControlFlow::Break(Ok(failed(report, "file byte limit exceeds platform")));
            }
        },
        max_total_bytes: match usize::try_from(report.limits.max_total_bytes) {
            Ok(value) => value,
            Err(_) => {
                return ControlFlow::Break(Ok(failed(report, "total byte limit exceeds platform")));
            }
        },
        max_matches: report.limits.max_matches as usize,
        evidence_max_chars: report.limits.evidence_max_chars as usize,
    };
    let attempts = match report.limits.attempts() {
        Ok(attempts) => attempts,
        Err(error) => return ControlFlow::Break(Ok(invalid(report, error))),
    };
    let request_budget = match report.limits.request_budget() {
        Ok(budget) => budget,
        Err(error) => return ControlFlow::Break(Ok(invalid(report, error))),
    };
    let settings = RulebookRunSettings {
        backend: backend_selection(&bundle.settings.decision_model),
        attempts,
        request_budget,
        request_max_chars: report.limits.request_max_chars as usize,
        max_concurrent_requests: report.limits.max_concurrent_requests as usize,
        max_model_requests: report.limits.max_model_requests as usize,
        max_locate_lines: report.limits.max_locate_lines as usize,
    };

    let mut dispatcher = ModelRequestDispatch::new(settings, backend, cancellation.clone());
    if preparing {
        dispatcher = dispatcher
            .prepare_only()
            .with_recorded_selections(recorded_selections);
    }

    let definitions = std::mem::take(&mut checked.selections);
    let project = checked.project.clone();
    let scope_files = checked_files.iter().cloned().collect::<Vec<_>>();
    let acquired_before_stage = source.checked_files_acquired();
    let started = tokio::task::spawn_blocking(move || {
        let build = EvidenceBuild::start(&project, &kinds, &files, &mut source, limits);
        (build, source)
    })
    .await;
    let (mut build, mut source) = match started {
        Ok((Ok(build), source)) => (build, source),
        Ok((Err(_), _)) => {
            return ControlFlow::Break(Ok(failed(report, "evidence kind plan failed")));
        }
        Err(_) => {
            return ControlFlow::Break(Err(RunStageStopped::with_settled_requests(
                RunStage::EvidenceBuilding,
                report,
                &dispatcher,
                acquired_before_stage,
            )));
        }
    };
    let scope_changes = changes.clone();
    let repository_root = selection.root.clone();
    let dependencies = folder_plan.dependencies.clone();
    let acquired_before_stage = source.checked_files_acquired();
    let prepared = tokio::task::spawn_blocking(move || {
        let mut targets = build.target_paths(&source, &repository_root);
        for (file, inputs) in dependencies {
            targets.entry(file).or_default().extend(inputs);
        }
        let files = selection_files_for_changes(&scope_files, &targets, &scope_changes, full);
        (build, source, files)
    })
    .await;
    let selection_files = match prepared {
        Ok((next_build, next_source, files)) => {
            build = next_build;
            source = next_source;
            files
        }
        Err(_) => {
            return ControlFlow::Break(Err(RunStageStopped::with_settled_requests(
                RunStage::SelectionScope,
                report,
                &dispatcher,
                acquired_before_stage,
            )));
        }
    };
    let mut chosen = ChosenCandidates::default();
    let mut catalog = loop {
        let acquired_before_stage = source.checked_files_acquired();
        let advanced = tokio::task::spawn_blocking(move || {
            let step = build.advance(chosen, &mut source);
            (build, source, step)
        })
        .await;
        let step = match advanced {
            Ok((next_build, next_source, step)) => {
                build = next_build;
                source = next_source;
                step
            }
            Err(_) => {
                return ControlFlow::Break(Err(RunStageStopped::with_settled_requests(
                    RunStage::EvidenceStage,
                    report,
                    &dispatcher,
                    acquired_before_stage,
                )));
            }
        };
        match step {
            BuildStep::Done(catalog) => break catalog,
            BuildStep::NeedsSelection {
                kind,
                candidates,
                shared,
            } => {
                let Some(selection) = definitions
                    .iter()
                    .find(|selection| selection.kind == kind)
                    .cloned()
                else {
                    return ControlFlow::Break(Ok(failed(report, "selection admission missing")));
                };
                let input = SelectionInput {
                    selection,
                    candidates,
                    shared,
                    files: selection_files.clone(),
                };
                let outcomes = select_candidates(&input, &mut dispatcher).await;
                if preparing {
                    let unresolved = outcomes
                        .iter()
                        .filter_map(|(candidate, outcome)| {
                            matches!(
                                outcome,
                                SelectionOutcome::Prepared { .. } | SelectionOutcome::Deferred
                            )
                            .then_some(candidate.clone())
                        })
                        .collect();
                    let deferred = build.defer_selection(&unresolved);
                    if let Some(preparation) = &mut report.preparation {
                        for item in deferred {
                            for request_id in outcomes
                                .iter()
                                .filter_map(|(candidate, outcome)| match outcome {
                                    SelectionOutcome::Prepared { request_id }
                                        if candidate.file == item.file =>
                                    {
                                        Some(*request_id)
                                    }
                                    _ => None,
                                })
                                .collect::<std::collections::BTreeSet<_>>()
                            {
                                preparation.needs_answer.push(NeedsSelectionAnswer {
                                    kind: item.kind.clone(),
                                    file: item.file.clone(),
                                    waits_on: SelectionDependency {
                                        selection_kind: kind.clone(),
                                        request_id,
                                    },
                                });
                            }
                        }
                    }
                }
                chosen = ChosenCandidates(
                    outcomes
                        .iter()
                        .filter_map(|(id, outcome)| {
                            matches!(outcome, SelectionOutcome::Selected { .. })
                                .then_some(id.clone())
                        })
                        .collect(),
                );
                for candidate in input.candidates {
                    if let Some(outcome) = outcomes.get(&candidate.id) {
                        report.selections.push(SelectionEntry {
                            kind: kind.clone(),
                            location: candidate.location,
                            first_line: candidate.first_line,
                            outcome: outcome.clone(),
                        });
                    }
                }
            }
        }
    };
    let (demanded, _) = pick_pieces(
        &catalog.pieces,
        changes,
        full,
        &selection.root,
        &folder_plan.dependencies,
    );
    let demanded = demanded
        .into_iter()
        .filter_map(|index| catalog.pieces.get(index).cloned())
        .collect::<Vec<_>>();
    let context_files = super::filter_folders::demanded_context_files(
        &folder_plan,
        &checked.filters,
        &checked.assembly.rules,
        &demanded,
    );
    if !context_files.is_empty() {
        let acquired_before_stage = source.checked_files_acquired();
        let extended = tokio::task::spawn_blocking(move || {
            let catalog = build.add_context_files(&context_files, &mut source);
            (source, catalog)
        })
        .await;
        match extended {
            Ok((next_source, next_catalog)) => {
                source = next_source;
                catalog = next_catalog;
            }
            Err(_) => {
                return ControlFlow::Break(Err(RunStageStopped::with_settled_requests(
                    RunStage::ContextEvidence,
                    report,
                    &dispatcher,
                    acquired_before_stage,
                )));
            }
        }
    }
    state.report = report;
    ControlFlow::Continue((
        state,
        (catalog, checked_files, folder_plan, dispatcher),
        source,
    ))
}
