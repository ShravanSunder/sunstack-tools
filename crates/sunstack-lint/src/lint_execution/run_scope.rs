use super::{
    lint_run::{RunEnd, RunState, failed, invalid},
    stopped_stages::{RunStage, RunStageStopped},
    targets::TargetSelection,
};
use crate::{configuration_loading::definition_bundle::DefinitionBundle, *};
use std::{collections::BTreeMap, ops::ControlFlow, path::PathBuf};
use sunstack_evidence::*;

pub(super) type ScopeInputs = (
    TargetSelection,
    Vec<CapturedChange>,
    BTreeMap<PathBuf, String>,
    bool,
);

pub(super) async fn select_run_scope(
    mut state: RunState,
    cwd: PathBuf,
    bundle: &DefinitionBundle,
    checked_inputs: &[PathBuf],
    model_work: bool,
) -> ControlFlow<RunEnd, (RunState, ScopeInputs)> {
    let request = &state.request;
    let cancellation = &state.cancellation;
    let mut report = state.report;
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
        Ok(Err(error)) => return ControlFlow::Break(Ok(invalid(report, error))),
        Err(_) => {
            return ControlFlow::Break(Err(RunStageStopped::before_dispatch(
                RunStage::TargetSelection,
                report,
            )));
        }
    };
    report.exclusions.ignored = selection.ignored;
    if selection.files.is_empty() && !request.paths.is_empty() {
        return ControlFlow::Break(Ok(failed(
            report,
            &format!(
                "nothing selected for named paths within root {}",
                selection.root.display()
            ),
        )));
    }
    let keyless_empty_scope = model_work && !state.preparing && request.evidence_listing.is_none();
    if keyless_empty_scope && selection.files.is_empty() {
        report.status = if report.incomplete.is_empty() {
            LintStatus::NothingSelected
        } else {
            LintStatus::Incomplete
        };
        return ControlFlow::Break(Ok(report));
    }
    let full = matches!(report.scope.kind, ScopeChoice::All);
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
            return ControlFlow::Break(Ok(invalid(
                report,
                LintInvalid::at(
                    &selection.root,
                    LintInvalidCode::UnsupportedInputs,
                    "change scope with --root requires the repository root; use --all for a checked subdirectory",
                ),
            )));
        }
        if git2::Repository::open(&selection.root).is_err() {
            return ControlFlow::Break(Ok(invalid(
                report,
                bundle.root.invalid(
                    LintInvalidCode::NoGitForDefaultScope,
                    "change scope requires Git; use --all outside a repository",
                ),
            )));
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
            _ => return ControlFlow::Break(Ok(failed(report, "change discovery limits invalid"))),
        };
        let repository = match RepositoryId::new("lint") {
            Ok(repository) => repository,
            Err(_) => return ControlFlow::Break(Ok(failed(report, "repository identity invalid"))),
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
                return ControlFlow::Break(Ok(super::scope_diagnostics::capture_refusal(
                    report,
                    &selection.root,
                    error,
                )));
            }
        };
        if !matches!(
            capture.coverage,
            DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
        ) {
            return ControlFlow::Break(Ok(super::scope_diagnostics::incomplete_capture(
                report,
                &selection.root,
                &capture.coverage,
                &discovery,
            )));
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
            checked_inputs.iter().any(|input| {
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
    if keyless_empty_scope && !full && changes.is_empty() {
        report.status = if report.incomplete.is_empty() {
            LintStatus::NothingSelected
        } else {
            LintStatus::Incomplete
        };
        return ControlFlow::Break(Ok(report));
    }
    state.report = report;
    ControlFlow::Continue((state, (selection, changes, frozen, full)))
}
