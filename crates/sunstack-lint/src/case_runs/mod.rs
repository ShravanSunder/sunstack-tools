//! Score revision-bound rule and filter cases through the ordinary lint command.
mod bars;
mod contracts;
mod matching;
mod saved_readback;
mod snapshots;
use crate::{configuration_loading::config_validation::load_checked_rulebook, *};
pub use contracts::*;
pub use saved_readback::{show_case, show_prepared_case};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use tokio_util::sync::CancellationToken;

fn validate(file: &CaseFile) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for case in &file.cases {
        if case.id.trim().is_empty() || !ids.insert(case.id.clone()) {
            return Err("case ids must be nonempty and unique".into());
        }
        let path = match &case.subject {
            CaseSubject::Rule { file, .. } => file,
            CaseSubject::Filter { folder, .. } => folder,
        };
        if path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(format!("case {} needs a repository-relative path", case.id));
        }
    }
    Ok(())
}
pub async fn run_cases(
    file_path: &Path,
    config_path: Option<&Path>,
    set: Option<&str>,
    executable: &Path,
    cancellation: CancellationToken,
) -> Result<CasesReport, String> {
    run_cases_internal(
        file_path,
        config_path,
        set,
        executable,
        cancellation,
        CaseExecution::Ordinary,
    )
    .await
}
pub async fn prepare_cases(
    file_path: &Path,
    config_path: Option<&Path>,
    set: Option<&str>,
    executable: &Path,
    cancellation: CancellationToken,
    selections_from: Option<&Path>,
) -> Result<CasesReport, String> {
    let selections_from = selections_from
        .map(|path| path.canonicalize().map_err(|error| error.to_string()))
        .transpose()?;
    if let Some(path) = &selections_from {
        crate::lint_reporting::selection_sources::read_selection_records(path)?;
    }
    run_cases_internal(
        file_path,
        config_path,
        set,
        executable,
        cancellation,
        CaseExecution::Prepare { selections_from },
    )
    .await
}
pub(super) enum CaseExecution {
    Ordinary,
    Prepare {
        selections_from: Option<std::path::PathBuf>,
    },
}
impl CaseExecution {
    fn is_preparing(&self) -> bool {
        matches!(self, Self::Prepare { .. })
    }
}
async fn run_cases_internal(
    file_path: &Path,
    config_path: Option<&Path>,
    set: Option<&str>,
    executable: &Path,
    cancellation: CancellationToken,
    mode: CaseExecution,
) -> Result<CasesReport, String> {
    let file_path = file_path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let file: CaseFile =
        serde_json::from_slice(&std::fs::read(&file_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    validate(&file)?;
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let location = discover_config(&cwd, config_path).map_err(|error| error.to_string())?;
    let config = load_config(&location).map_err(|error| error.to_string())?;
    let checked = load_checked_rulebook(&config).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    if !checked.assembly.skipped.is_empty() {
        return Err("today's rulebook has skipped rules; fix them before running cases".into());
    }
    let current_repository =
        git2::Repository::discover(&location.directory).map_err(|error| error.to_string())?;
    let current_root = current_repository
        .workdir()
        .ok_or_else(|| "current rulebook needs a working repository".to_string())?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let repository = file.repository.as_ref().map_or_else(
        || Ok(current_root.clone()),
        |path| {
            let base = file_path.parent().unwrap_or(&cwd);
            base.join(path)
                .canonicalize()
                .map_err(|error| error.to_string())
        },
    )?;
    let files = crate::lint_reporting::saved_reports::capture_files(&checked.inputs)
        .map_err(|error| error.to_string())?;
    let selected = file
        .cases
        .iter()
        .filter(|case| set.is_none_or(|set| case.set == set))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("no cases selected".into());
    }
    let revisions = selected
        .iter()
        .map(|case| case.revision.clone())
        .collect::<BTreeSet<_>>();
    let mut snapshots = BTreeMap::new();
    let mut errors = BTreeMap::new();
    let mut runs = Vec::new();
    for revision in revisions {
        if cancellation.is_cancelled() {
            if mode.is_preparing() {
                break;
            }
            return Err("case run cancelled".into());
        }
        let case_files = selected
            .iter()
            .filter(|case| case.revision == revision)
            .filter_map(|case| match &case.subject {
                CaseSubject::Rule { file, .. } => Some(file.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let selected_paths = selected
            .iter()
            .filter(|case| case.revision == revision)
            .map(|case| match &case.subject {
                CaseSubject::Rule { file, .. } => file.clone(),
                CaseSubject::Filter { folder, .. } => folder.clone(),
            })
            .collect::<BTreeSet<_>>();
        match snapshots::prepare(
            &repository,
            &revision,
            &snapshots::RulebookOverlay {
                root: &current_root,
                config: &location.file,
                files: &files,
                rule_directories: checked.project.rule_directories(),
                check: &config.check,
            },
            &case_files,
            &selected_paths,
        ) {
            Ok(snapshot) => {
                let input_paths = selected
                    .iter()
                    .filter(|case| case.revision == revision)
                    .filter_map(|case| {
                        let selected_path = match &case.subject {
                            CaseSubject::Rule { file, .. } => file,
                            CaseSubject::Filter { folder, .. } => folder,
                        };
                        if snapshot.symlink_paths.contains(selected_path) {
                            return None;
                        }
                        let path = match &case.subject {
                            CaseSubject::Rule { file, .. }
                                if snapshot.tree.join(file).is_file() =>
                            {
                                file
                            }
                            CaseSubject::Filter { folder, .. }
                                if snapshot.tree.join(folder).is_dir() =>
                            {
                                folder
                            }
                            _ => return None,
                        };
                        Some(path.clone())
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                match snapshots::lint(executable, &snapshot, &input_paths, &cancellation, &mode)
                    .await
                {
                    Ok(report) => {
                        runs.push(RevisionRun {
                            revision: revision.clone(),
                            resolved_revision: snapshot.resolved_revision.clone(),
                            report,
                        });
                    }
                    Err(error) => {
                        errors.insert(revision.clone(), error);
                    }
                }
                snapshots.insert(revision, snapshot);
            }
            Err(error) => {
                errors.insert(revision, error);
            }
        }
    }
    let mut report = CasesReport {
        preparation: mode.is_preparing().then_some(PreparedCasesReport {
            mode: PrepareMode::Prepare,
            status: LintStatus::Prepared,
        }),
        not_selected: file
            .cases
            .iter()
            .filter(|case| set.is_some_and(|set| case.set != set))
            .map(|case| case.id.clone())
            .collect(),
        by_rule: BTreeMap::new(),
        by_filter: BTreeMap::new(),
        cases: vec![],
        counts: BTreeMap::new(),
        bars: bars::assess(&file, &checked),
        runs,
        errors,
    };
    for case in selected {
        let snapshot = snapshots.get(&case.revision);
        let selected_path = match &case.subject {
            CaseSubject::Rule { file, .. } => file,
            CaseSubject::Filter { folder, .. } => folder,
        };
        let run = report.runs.iter().find(|run| run.revision == case.revision);
        let selected_symlink =
            snapshot.is_some_and(|snapshot| snapshot.symlink_paths.contains(selected_path));
        let (outcome, result_id, crossings) = if selected_symlink {
            (
                CaseOutcome::NotJudged {
                    reason: "selected path is a symlink".into(),
                },
                None,
                vec![],
            )
        } else if let (Some(snapshot), Some(run)) = (snapshot, run) {
            let dependency = match &case.subject {
                CaseSubject::Rule { rule, file, .. } => checked
                    .assembly
                    .rules
                    .iter()
                    .find(|entry| &entry.name == rule)
                    .map(|entry| (entry.kind.clone(), file.clone())),
                CaseSubject::Filter { filter, folder, .. } => checked
                    .filters
                    .get(filter)
                    .map(|entry| (entry.reads.kind.clone(), folder.join(&entry.reads.file))),
            };
            let (outcome, id) = if mode.is_preparing() {
                matching::prepare(case, snapshot, &run.report, dependency.as_ref())
            } else {
                matching::score(case, snapshot, &run.report)
            };
            let crossings = (!mode.is_preparing())
                .then_some(id)
                .flatten()
                .and_then(|id| {
                    run.report
                        .results
                        .iter()
                        .find(|result| result.id == id)
                        .map(|result| {
                            bars::crossings(
                                bars::BarOwner::Rule(&result.name),
                                &result.path,
                                &result.answers,
                                &checked,
                            )
                        })
                        .or_else(|| {
                            run.report
                                .filter_answers
                                .iter()
                                .find(|result| result.id == id)
                                .map(|result| {
                                    let (path, answers) = match &result.state {
                                        sunstack_rules::FilterAnswerState::Judged {
                                            path,
                                            answers,
                                            ..
                                        }
                                        | sunstack_rules::FilterAnswerState::Unsure {
                                            path,
                                            answers,
                                            ..
                                        }
                                        | sunstack_rules::FilterAnswerState::Incomplete {
                                            path,
                                            answers,
                                            ..
                                        } => (path, answers),
                                    };
                                    bars::crossings(
                                        bars::BarOwner::Filter(&result.filter),
                                        path,
                                        answers,
                                        &checked,
                                    )
                                })
                        })
                        .or_else(|| {
                            run.report
                                .filter_skips
                                .iter()
                                .find(|skip| skip.id == id)
                                .map(|skip| {
                                    bars::crossings(
                                        bars::BarOwner::Filter(&skip.filter),
                                        &skip.path,
                                        &skip.answers,
                                        &checked,
                                    )
                                })
                        })
                })
                .unwrap_or_default();
            (outcome, id, crossings)
        } else {
            (
                CaseOutcome::Incomplete {
                    reason: report
                        .errors
                        .get(&case.revision)
                        .cloned()
                        .unwrap_or_else(|| "revision unavailable".into()),
                },
                None,
                vec![],
            )
        };
        *report.counts.entry(outcome.key().into()).or_default() += 1;
        let counts = match &case.subject {
            CaseSubject::Rule { rule, .. } => {
                report.by_rule.entry(rule.clone()).or_insert_with(|| {
                    ["right", "false_warning", "missed", "unsure", "incomplete"]
                        .into_iter()
                        .map(|name| (name.into(), 0))
                        .collect()
                })
            }
            CaseSubject::Filter { filter, .. } => {
                report.by_filter.entry(filter.clone()).or_insert_with(|| {
                    ["right", "wrong_label", "unsure", "incomplete"]
                        .into_iter()
                        .map(|name| (name.into(), 0))
                        .collect()
                })
            }
        };
        *counts.entry(outcome.key().into()).or_default() += 1;
        report.cases.push(CaseResult {
            id: case.id.clone(),
            definition: case.clone(),
            outcome,
            result_id,
            bars: crossings,
        });
    }
    if let Some(preparation) = &mut report.preparation {
        preparation.status = if report
            .runs
            .iter()
            .any(|run| run.report.status == LintStatus::Invalid)
        {
            LintStatus::Invalid
        } else if cancellation.is_cancelled()
            || report
                .runs
                .iter()
                .any(|run| run.report.status == LintStatus::Cancelled)
        {
            LintStatus::Cancelled
        } else if !report.errors.is_empty()
            || report
                .cases
                .iter()
                .any(|case| matches!(case.outcome, CaseOutcome::Incomplete { .. }))
            || report
                .runs
                .iter()
                .any(|run| run.report.status == LintStatus::Incomplete)
        {
            LintStatus::Incomplete
        } else if report
            .runs
            .iter()
            .any(|run| run.report.status == LintStatus::Findings)
            || report
                .cases
                .iter()
                .any(|case| !matches!(case.outcome, CaseOutcome::Prepared))
        {
            LintStatus::Findings
        } else {
            LintStatus::Prepared
        };
    }
    Ok(report)
}
