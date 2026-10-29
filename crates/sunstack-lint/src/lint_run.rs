use crate::{
    backend_factory::{backend_selection, prepare_backend},
    config_validation::{load_checked_rulebook, native_diagnostics},
    *,
};
use std::collections::BTreeMap;
use sunstack_evidence::*;
use sunstack_rules::{RulebookRunSettings, run_rulebook};
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
        file: None,
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
    let mut report = LintReport::empty(
        request
            .config_path
            .clone()
            .unwrap_or_else(|| "sunstack.lint.json".into()),
        request.scope.clone(),
        EffectiveLimits::default(),
    );
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
    let config_to_check = config.clone();
    let checked =
        match tokio::task::spawn_blocking(move || load_checked_rulebook(&config_to_check)).await {
            Ok(Ok(checked)) => checked,
            Ok(Err(diagnostics)) => {
                report.status = LintStatus::Invalid;
                report.diagnostics = diagnostics;
                return report;
            }
            Err(_) => return failed(report, "rulebook loading failed"),
        };
    if let Some(Some(kind)) = &request.evidence_listing
        && !config.evidence.kinds.contains_key(kind)
    {
        return invalid(
            report,
            LintInvalid::at(
                &location.file,
                LintInvalidCode::Syntax,
                "unknown evidence kind",
            ),
        );
    }
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
            report
                .diagnostics
                .extend(native_diagnostics(&config, skipped.diagnostics.clone()));
        }
    }
    let asking = checked
        .assembly
        .rules
        .iter()
        .any(|rule| !rule.questions.is_empty());
    let key = api_key.as_deref().filter(|key| !key.trim().is_empty());
    if request.evidence_listing.is_none() && asking && key.is_none() {
        return invalid(
            report,
            LintInvalid::at(
                &location.file,
                LintInvalidCode::CredentialMissing,
                "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first.",
            ),
        );
    }
    let backend = if request.evidence_listing.is_none() && asking {
        match key.map(|key| prepare_backend(&config, key)) {
            Some(Ok(backend)) => Some(backend),
            Some(Err(error)) => return invalid(report, error),
            None => None,
        }
    } else {
        None
    };
    let selected_config = config.clone();
    let paths = request.paths.clone();
    let selected_cwd = cwd.clone();
    let selection = match tokio::task::spawn_blocking(move || {
        targets::select_targets(&selected_config, &paths, &selected_cwd)
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return invalid(report, error),
        Err(_) => return failed(report, "target selection failed"),
    };
    report.exclusions.ignored = selection.ignored;
    let full = matches!(request.scope, ScopeChoice::All) || !request.paths.is_empty();
    let mut changes = Vec::new();
    let mut frozen = BTreeMap::new();
    let mut full = full;
    if !full {
        if git2::Repository::open(&selection.root).is_err() {
            return invalid(
                report,
                LintInvalid::at(
                    &location.file,
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
                binding: RepositoryBinding {
                    key: repository,
                    root: selection.root.clone(),
                    input_paths: vec![QueryPath::Glob {
                        pattern: "**".into(),
                    }],
                    change_scope,
                },
                limits: discovery,
                anchor_utc: chrono::Utc::now(),
            },
            cancellation.clone(),
        )
        .await;
        let capture = match capture {
            Ok(capture) => capture,
            Err(error) => {
                return invalid(
                    report,
                    LintInvalid::at(
                        &location.file,
                        LintInvalidCode::Syntax,
                        format!("cannot resolve change scope: {:?}", error.code),
                    ),
                );
            }
        };
        if !matches!(
            capture.coverage,
            DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
        ) {
            return failed(report, "change discovery reached a limit");
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
            checked
                .inputs
                .iter()
                .any(|input| targets::relative_path(&selection.root, input) == change.path)
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
    let kinds = config.evidence.kinds.clone();
    let limits = PieceBuildLimits {
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
    let project = checked.project;
    let built = tokio::task::spawn_blocking(move || {
        let catalog = build_pieces(&project, &kinds, &files, &mut source, limits);
        (catalog, source.checked_files_acquired())
    })
    .await;
    let (catalog, acquired) = match built {
        Ok(value) => value,
        Err(_) => return failed(report, "evidence building failed"),
    };
    report.usage.checked_files_acquired = acquired;
    for failure in &catalog.failures {
        report.incomplete.push(IncompletePart {
            rule: None,
            file: Some(failure.file.clone()),
            field: None,
            reason: IncompleteReason::SelectionFailed,
        });
    }
    let (picked, _) = pick_pieces(&catalog.pieces, &changes, full);
    let picked_set = picked
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let outside = checked
        .assembly
        .rules
        .iter()
        .map(|rule| {
            catalog
                .pieces
                .iter()
                .enumerate()
                .filter(|(index, piece)| piece.kind == rule.kind && !picked_set.contains(index))
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
            usage: report.usage.clone(),
        });
        report.status = if cancellation.is_cancelled() {
            LintStatus::Cancelled
        } else if !report.incomplete.is_empty() {
            LintStatus::Incomplete
        } else {
            LintStatus::Clean
        };
        return report;
    }
    let attempts = match report.limits.attempts() {
        Ok(attempts) => attempts,
        Err(error) => return invalid(report, error),
    };
    let request_budget = match report.limits.request_budget() {
        Ok(budget) => budget,
        Err(error) => return invalid(report, error),
    };
    let settings = RulebookRunSettings {
        backend: backend_selection(&config),
        attempts,
        request_budget,
        request_max_chars: report.limits.request_max_chars as usize,
        max_concurrent_requests: report.limits.max_concurrent_requests as usize,
        max_model_requests: report.limits.max_model_requests as usize,
    };
    let run = run_rulebook(
        &checked.assembly.rules,
        &pieces,
        &catalog.matches,
        settings,
        backend,
        cancellation.clone(),
    )
    .await;
    outcome::project_outcome(
        run,
        &checked.assembly.rules,
        &pieces,
        report,
        request.max_warnings,
        cancellation.is_cancelled(),
    )
}
