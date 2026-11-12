use crate::{
    configuration_loading::{
        config_validation::{admit_definitions, native_diagnostics},
        definition_bundle::{DefinitionBundle, acquire_file_definitions},
    },
    lint_execution::backend_factory::{backend_selection, prepare_backend},
    *,
};
use std::collections::BTreeMap;
use sunstack_evidence::*;
use sunstack_rules::{
    ModelRequestDispatch, RulebookRunSettings, SelectionInput, SelectionOutcome,
    run_rulebook_with_dispatcher, select_candidates,
};
use tokio_util::sync::CancellationToken;

fn invalid(mut report: LintReport, error: LintInvalid) -> LintReport {
    report.status = LintStatus::Invalid;
    report.diagnostics.push(error.into());
    report
}
fn failed(mut report: LintReport, message: &str) -> LintReport {
    report.status = LintStatus::Incomplete;
    report.incomplete.push(IncompletePart {
        rule: None,
        file: None,
        field: None,
        reason: IncompleteReason::PreparationFailed,
    });
    report.diagnostics.push(LintDiagnostic {
        definition: None,
        json_path: vec![],
        file: None,
        path: None,
        line: None,
        column: None,
        code: LintInvalidCode::Syntax,
        message: message.into(),
    });
    report
}

pub async fn run(
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
) -> LintReport {
    run_with_source(request, cancellation, api_key, None).await
}
/// Inject the acquisition boundary for callers that need to observe or provide file contents.
/// A captured Git after view is served before the fallback source; verification rereads belong to Git capture.
pub async fn run_with_source(
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
) -> LintReport {
    run_internal(
        request,
        cancellation,
        api_key,
        source,
        ExecutionMode::Ordinary,
    )
    .await
}

pub async fn prepare(
    request: LintRequest,
    cancellation: CancellationToken,
    recorded_selections: Vec<sunstack_rules::RecordedSelection>,
) -> LintReport {
    run_internal(
        request,
        cancellation,
        None,
        None,
        ExecutionMode::Prepare {
            recorded_selections,
        },
    )
    .await
}
enum ExecutionMode {
    Ordinary,
    Prepare {
        recorded_selections: Vec<sunstack_rules::RecordedSelection>,
    },
}
async fn run_internal(
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
    mode: ExecutionMode,
) -> LintReport {
    let preparing = matches!(mode, ExecutionMode::Prepare { .. });
    let mut report = LintReport::empty(
        request
            .config_path
            .clone()
            .unwrap_or_else(|| "sunstack.lint.json".into()),
        request.scope.clone(),
        EffectiveLimits::default(),
    );
    if preparing {
        report.preparation = Some(PreparedReport {
            mode: PrepareMode::Prepare,
            needs_answer: vec![],
        });
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return failed(report, "current directory unavailable"),
    };
    let start = cwd.clone();
    let explicit = request.config_path.clone();
    let loaded = tokio::task::spawn_blocking(move || {
        let location = discover_config(&start, explicit.as_deref())?;
        let config = load_config(&location)?;
        Ok::<_, LintInvalid>((location, config))
    })
    .await;
    let (location, config) = match loaded {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return invalid(report, error),
        Err(_) => return failed(report, "config loading failed"),
    };
    report.config = location.file.clone();
    report.limits = config
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return report;
    }
    let acquired = tokio::task::spawn_blocking(move || {
        let bundle = acquire_file_definitions(&config, &location);
        let checked = admit_definitions(&bundle);
        (bundle, checked)
    })
    .await;
    let (bundle, checked) = match acquired {
        Ok(value) => value,
        Err(_) => return failed(report, "rulebook loading failed"),
    };
    execute_definitions(
        report,
        request,
        cwd,
        bundle,
        checked,
        cancellation,
        api_key,
        source,
        mode,
    )
    .await
}

/// Run already captured definitions against an explicit absolute checked root.
///
/// The caller owns definition capture; nothing here rereads a definition file,
/// consults the process working directory or reads process credentials.
pub async fn run_definitions(
    bundle: DefinitionBundle,
    root: std::path::PathBuf,
    paths: Vec<std::path::PathBuf>,
    scope: ScopeChoice,
    cancellation: CancellationToken,
    api_key: Option<String>,
) -> LintReport {
    let mut report = LintReport::empty(
        bundle.root.origin.cli_path(),
        scope.clone(),
        bundle
            .settings
            .limits
            .as_ref()
            .map(EffectiveLimits::from)
            .unwrap_or_default(),
    );
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return report;
    }
    let request = LintRequest {
        config_path: None,
        root: Some(root.clone()),
        paths,
        scope,
        max_warnings: None,
        evidence_listing: None,
    };
    let admitted = tokio::task::spawn_blocking(move || {
        let checked = admit_definitions(&bundle);
        (bundle, checked)
    })
    .await;
    let (bundle, checked) = match admitted {
        Ok(value) => value,
        Err(_) => return failed(report, "rulebook loading failed"),
    };
    execute_definitions(
        report,
        request,
        root,
        bundle,
        checked,
        cancellation,
        api_key,
        None,
        ExecutionMode::Ordinary,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn execute_definitions(
    mut report: LintReport,
    request: LintRequest,
    cwd: std::path::PathBuf,
    bundle: DefinitionBundle,
    checked: Result<
        crate::configuration_loading::config_validation::CheckedRulebook,
        Vec<LintDiagnostic>,
    >,
    cancellation: CancellationToken,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
    mode: ExecutionMode,
) -> LintReport {
    let preparing = matches!(mode, ExecutionMode::Prepare { .. });
    let recorded_selections = match mode {
        ExecutionMode::Ordinary => vec![],
        ExecutionMode::Prepare {
            recorded_selections,
        } => recorded_selections,
    };
    let checked = match checked {
        Ok(checked) => checked,
        Err(diagnostics) => {
            report.status = LintStatus::Invalid;
            report.diagnostics = diagnostics;
            return report;
        }
    };
    if let Some(Some(kind)) = &request.evidence_listing
        && !bundle.settings.kinds.contains_key(kind)
    {
        return invalid(
            report,
            bundle
                .root
                .invalid(LintInvalidCode::Syntax, "unknown evidence kind"),
        );
    }
    report.rulebook_files = match bundle.file_snapshots(&checked.inputs) {
        Ok(files) => files,
        Err(_) => return failed(report, "rulebook snapshot unavailable"),
    };
    report.skipped_rules = checked.assembly.skipped.clone();
    if !report.skipped_rules.is_empty() {
        report.incomplete.push(IncompletePart {
            rule: None,
            file: None,
            field: None,
            reason: if checked.assembly.rules.is_empty() {
                IncompleteReason::CoverageNone
            } else {
                IncompleteReason::CoveragePartial
            },
        });
        for skipped in &checked.assembly.skipped {
            report.diagnostics.extend(native_diagnostics(
                &bundle.root,
                skipped.diagnostics.clone(),
            ));
        }
    }
    let asking = checked
        .assembly
        .rules
        .iter()
        .any(|rule| !rule.questions.is_empty());
    let selected = bundle
        .settings
        .kinds
        .values()
        .any(|kind| kind.select.is_some());
    let gated = checked
        .assembly
        .rules
        .iter()
        .any(|rule| rule.gate.is_some());
    let model_work = selected || (request.evidence_listing.is_none() && (asking || gated));
    let key = api_key.as_deref().filter(|key| !key.trim().is_empty());
    if model_work && !preparing && key.is_none() {
        return invalid(
            report,
            bundle.root.invalid(
                LintInvalidCode::CredentialMissing,
                "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first.",
            ),
        );
    }
    let backend = if model_work && preparing {
        match super::backend_factory::prepare_encoding_backend(&bundle) {
            Ok(backend) => Some(backend),
            Err(error) => return invalid(report, error),
        }
    } else if model_work {
        match key.map(|key| prepare_backend(&bundle, key)) {
            Some(Ok(backend)) => Some(backend),
            Some(Err(error)) => return invalid(report, error),
            None => None,
        }
    } else {
        None
    };
    let selected_check = bundle.settings.check.clone();
    let config_directory = bundle.root.base_directory.clone();
    let paths = request.paths.clone();
    let selected_cwd = cwd.clone();
    let checked_root = request.root.clone();
    let selection = match tokio::task::spawn_blocking(move || {
        crate::lint_execution::targets::select_targets(
            &selected_check,
            &config_directory,
            &paths,
            &selected_cwd,
            checked_root.as_deref(),
        )
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return invalid(report, error),
        Err(_) => return failed(report, "target selection failed"),
    };
    report.exclusions.ignored = selection.ignored;
    if selection.files.is_empty() && !request.paths.is_empty() {
        return failed(
            report,
            &format!(
                "nothing selected for named paths within root {}",
                selection.root.display()
            ),
        );
    }
    let full = matches!(request.scope, ScopeChoice::All) || !request.paths.is_empty();
    let mut changes = Vec::new();
    let mut frozen = BTreeMap::new();
    let mut full = full;
    if !full {
        if request.root.is_some()
            && git2::Repository::discover(&selection.root)
                .ok()
                .and_then(|repository| {
                    repository
                        .workdir()
                        .and_then(|path| path.canonicalize().ok())
                })
                .as_ref()
                != Some(&selection.root)
        {
            return invalid(
                report,
                LintInvalid::at(
                    &selection.root,
                    LintInvalidCode::UnsupportedInputs,
                    "change scope with --root requires the repository root; use --all for a checked subdirectory",
                ),
            );
        }
        if git2::Repository::open(&selection.root).is_err() {
            return invalid(
                report,
                bundle.root.invalid(
                    LintInvalidCode::NoGitForDefaultScope,
                    "change scope requires Git; use --all outside a repository",
                ),
            );
        }
        let change_scope = match &request.scope {
            ScopeChoice::Since { rev } => ChangeScope::RevisionDiff {
                base_revision: rev.clone(),
                head: RevisionHead::Worktree {},
            },
            ScopeChoice::Default => ChangeScope::Uncommitted {
                layers: vec![
                    DirtyLayer::Staged,
                    DirtyLayer::Unstaged,
                    DirtyLayer::Untracked,
                ],
            },
            ScopeChoice::All => ChangeScope::Full {},
        };
        let discovery = match report.limits.discovery() {
            Ok(EvidenceDiscovery::Bounded { limits }) => limits,
            _ => return failed(report, "change discovery limits invalid"),
        };
        let repository = match RepositoryId::new("lint") {
            Ok(repository) => repository,
            Err(_) => return failed(report, "repository identity invalid"),
        };
        let capture = capture_change_selection(
            ChangeCaptureRequest {
                context: ContextCapture::ChangedOnly,
                binding: RepositoryBinding {
                    key: repository,
                    root: selection.root.clone(),
                    input_paths: vec![QueryPath::Glob {
                        pattern: "**".into(),
                    }],
                    change_scope,
                },
                limits: discovery.clone(),
                anchor_utc: chrono::Utc::now(),
            },
            cancellation.clone(),
        )
        .await;
        let capture = match capture {
            Ok(capture) => capture,
            Err(error) => {
                return super::scope_diagnostics::capture_refusal(report, &selection.root, error);
            }
        };
        if !matches!(
            capture.coverage,
            DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
        ) {
            return super::scope_diagnostics::incomplete_capture(
                report,
                &selection.root,
                &capture.coverage,
                &discovery,
            );
        }
        changes = capture
            .members
            .into_iter()
            .flat_map(|member| member.changes)
            .collect();
        for file in capture.sources {
            if let SourceRef::RepositoryFile {
                path,
                revision: SourceRevision::Worktree { .. },
                ..
            } = &file.source
                && file.side == SourceSide::After
            {
                frozen.insert(path.clone(), file.text);
            }
        }
        if changes.iter().any(|change| {
            checked.inputs.iter().any(|input| {
                crate::lint_execution::targets::relative_path(&selection.root, input) == change.path
            })
        }) {
            full = true;
            report.scope.widened = Some(WidenedReason::ConfigChanged);
        }
    }
    if full {
        report.scope.kind = ScopeChoice::All;
    }
    let mut files = selection.files;
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
            Err(_) => return failed(report, "checked source unavailable"),
        },
    };
    let mut source = FrozenCheckedSource::new(frozen, fallback);
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
            Err(_) => return failed(report, "file byte limit exceeds platform"),
        },
        max_total_bytes: match usize::try_from(report.limits.max_total_bytes) {
            Ok(value) => value,
            Err(_) => return failed(report, "total byte limit exceeds platform"),
        },
        max_matches: report.limits.max_matches as usize,
        evidence_max_chars: report.limits.evidence_max_chars as usize,
    };
    let attempts = match report.limits.attempts() {
        Ok(attempts) => attempts,
        Err(error) => return invalid(report, error),
    };
    let request_budget = match report.limits.request_budget() {
        Ok(budget) => budget,
        Err(error) => return invalid(report, error),
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

    let definitions = checked.selections;
    let project = checked.project;
    let scope_files = checked_files.iter().cloned().collect::<Vec<_>>();
    let started = tokio::task::spawn_blocking(move || {
        let build = EvidenceBuild::start(&project, &kinds, &files, &mut source, limits);
        (build, source)
    })
    .await;
    let (mut build, mut source) = match started {
        Ok((Ok(build), source)) => (build, source),
        Ok((Err(_), _)) => return failed(report, "evidence kind plan failed"),
        Err(_) => return failed(report, "evidence building failed"),
    };
    let scope_changes = changes.clone();
    let repository_root = selection.root.clone();
    let dependencies = folder_plan.dependencies.clone();
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
        Err(_) => return failed(report, "selection scope failed"),
    };
    let mut chosen = ChosenCandidates::default();
    let mut catalog = loop {
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
            Err(_) => return failed(report, "evidence stage failed"),
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
                    return failed(report, "selection admission missing");
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
        &changes,
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
            Err(_) => return failed(report, "context evidence building failed"),
        }
    }
    crate::lint_reporting::outcome::project_selections(&mut catalog, &mut report);
    let acquired = source.checked_files_acquired();
    report.usage.checked_files_acquired = acquired;
    crate::lint_reporting::outcome::project_evidence_failures(&catalog, &mut report);
    let (picked, _) = pick_pieces(
        &catalog.pieces,
        &changes,
        full,
        &selection.root,
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
    let outside = checked
        .assembly
        .rules
        .iter()
        .map(|rule| {
            catalog
                .pieces
                .iter()
                .enumerate()
                .filter(|(index, piece)| {
                    checked_files.contains(&piece.location.file)
                        && piece.kind == rule.kind
                        && !picked_set.contains(index)
                })
                .count()
        })
        .sum::<usize>();
    report.exclusions.outside_change_scope = u32::try_from(outside).unwrap_or(u32::MAX);
    let zero_matches = checked
        .assembly
        .rules
        .iter()
        .map(|rule| {
            catalog
                .matches
                .keys()
                .filter(|file| checked_files.contains(*file))
                .filter(|file| {
                    !catalog
                        .pieces
                        .iter()
                        .any(|piece| piece.kind == rule.kind && &piece.location.file == *file)
                })
                .count()
        })
        .sum::<usize>();
    report.exclusions.zero_matches = u32::try_from(zero_matches).unwrap_or(u32::MAX);
    let pieces = picked
        .into_iter()
        .filter_map(|index| catalog.pieces.get(index).cloned())
        .collect::<Vec<_>>();
    if let Some(kind) = request.evidence_listing {
        let pieces = pieces
            .into_iter()
            .filter(|piece| kind.as_ref().is_none_or(|kind| kind == &piece.kind))
            .collect();
        report.evidence_listing = Some(EvidenceListing {
            version: report.version.clone(),
            config: report.config.clone(),
            scope: report.scope.clone(),
            pieces,
            usage: crate::lint_reporting::outcome::selection_usage(&dispatcher, acquired),
            selections: report.selections.clone(),
            file_counts: report.file_counts.clone(),
        });
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
        return report;
    }
    let folders = super::filter_folders::bind_reads_pieces(
        &folder_plan,
        &checked.filters,
        &checked.assembly.rules,
        &pieces,
        &catalog,
    );
    let filter_answers = sunstack_rules::ask_filters(
        &checked.filters,
        &folders,
        &catalog.pieces,
        &bundle.settings.kinds,
        &catalog.matches,
        report.limits.evidence_max_chars as usize,
        &mut dispatcher,
    )
    .await;
    report.filter_answers = filter_answers
        .entries
        .iter()
        .cloned()
        .map(|value| IdentifiedResult {
            id: 0,
            request_ids: vec![],
            value,
        })
        .collect();
    let run = run_rulebook_with_dispatcher(
        &checked.assembly.rules,
        &pieces,
        &catalog.matches,
        &mut dispatcher,
        &filter_answers,
    )
    .await;
    crate::lint_reporting::saved_reports::capture_requests(
        &mut report,
        &run,
        &filter_answers,
        &checked.assembly.rules,
        &pieces,
        &dispatcher.settings.backend,
    );
    let prepared_requests = run.prepared.clone();
    let mut report = crate::lint_reporting::outcome::project_outcome(
        run,
        &checked.assembly.rules,
        &pieces,
        report,
        request.max_warnings,
        cancellation.is_cancelled(),
    );
    for answer in &report.filter_answers {
        if let Some(piece) = filter_answers
            .pieces
            .get(&(answer.filter.clone(), answer.folder.clone()))
        {
            report.piece_evidence.insert(
                answer.id,
                SavedPieceEvidence {
                    piece: piece.clone(),
                    matches: Default::default(),
                },
            );
        }
    }
    if !preparing
        && let Err(error) = crate::lint_reporting::saved_reports::capture_selection_requests(
            &mut report,
            &dispatcher,
        )
    {
        return failed(report, &error);
    }
    if preparing {
        if let Err(error) = crate::lint_reporting::saved_reports::capture_prepared_requests(
            &mut report,
            &dispatcher,
            &prepared_requests,
            &filter_answers,
            &checked.assembly.rules,
            &pieces,
        ) {
            return failed(report, &error);
        }
        if matches!(
            report.status,
            LintStatus::Clean | LintStatus::NothingSelected | LintStatus::Uncertain
        ) {
            report.status = LintStatus::Prepared;
        }
    }
    for saved in report.piece_evidence.values_mut() {
        if let Some(index) = catalog.matches.get(&saved.piece.location.file) {
            saved.matches = index.by_rule.clone();
        }
    }
    for result in &mut report.results {
        result.text_sha256 = catalog
            .piece_text_hashes
            .get(&(result.file.clone(), result.first_line, result.last_line))
            .cloned();
    }
    report.usage = crate::lint_reporting::outcome::selection_usage(&dispatcher, acquired);
    report
}
