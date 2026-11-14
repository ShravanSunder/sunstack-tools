use super::{
    lint_run::{ExecutionMode, RunState, failed, invalid},
    stopped_stages::{RunStage, RunStageStopped},
};
use crate::{
    configuration_loading::{
        config_validation::{CheckedRulebook, admit_definitions, native_diagnostics},
        definition_bundle::{DefinitionBundle, acquire_file_definitions},
    },
    *,
};
use std::{ops::ControlFlow, path::PathBuf};
use tokio_util::sync::CancellationToken;

/// The CLI's first step: discover and read its configuration file, then acquire and
/// admit the definitions it names.
pub(super) async fn acquire_file_run(
    request: LintRequest,
    cancellation: CancellationToken,
    mode: ExecutionMode,
) -> ControlFlow<
    LintReport,
    (
        RunState,
        PathBuf,
        DefinitionBundle,
        Result<CheckedRulebook, Vec<LintDiagnostic>>,
    ),
> {
    let preparing = matches!(mode, ExecutionMode::Prepare { .. });
    let recorded_selections = match mode {
        ExecutionMode::Ordinary => vec![],
        ExecutionMode::Prepare {
            recorded_selections,
        } => recorded_selections,
    };
    let mut report = LintReport::empty(
        request
            .config_path
            .clone()
            .unwrap_or_else(|| "sunstack.lint.json".into()),
        if request.paths.is_empty() {
            request.scope.clone()
        } else {
            ScopeChoice::All
        },
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
        Err(_) => return ControlFlow::Break(failed(report, "current directory unavailable")),
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
        Ok(Err(error)) => return ControlFlow::Break(invalid(report, error)),
        Err(_) => return ControlFlow::Break(failed(report, "config loading failed")),
    };
    report.config = location.file.clone();
    report.limits = config
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return ControlFlow::Break(report);
    }
    let acquired = tokio::task::spawn_blocking(move || {
        let bundle = acquire_file_definitions(&config, &location);
        let checked = admit_definitions(&bundle);
        (bundle, checked)
    })
    .await;
    let (bundle, checked) = match acquired {
        Ok(value) => value,
        Err(_) => {
            return ControlFlow::Break(
                RunStageStopped::before_dispatch(RunStage::Admission, report).into_cli_report(),
            );
        }
    };
    ControlFlow::Continue((
        RunState {
            request,
            cancellation,
            preparing,
            recorded_selections,
            report,
        },
        cwd,
        bundle,
        checked,
    ))
}

/// Settle an admission result into the report and decide whether the run needs model work.
pub(super) fn configure_admitted_run(
    mut state: RunState,
    bundle: &DefinitionBundle,
    checked: Result<CheckedRulebook, Vec<LintDiagnostic>>,
) -> ControlFlow<LintReport, (RunState, CheckedRulebook, bool)> {
    let request = &state.request;
    let mut report = state.report;
    let checked = match checked {
        Ok(checked) => checked,
        Err(diagnostics) => {
            report.status = LintStatus::Invalid;
            report.diagnostics = diagnostics;
            return ControlFlow::Break(report);
        }
    };
    if let Some(Some(kind)) = &request.evidence_listing
        && !bundle.settings.kinds.contains_key(kind)
    {
        return ControlFlow::Break(invalid(
            report,
            bundle
                .root
                .invalid(LintInvalidCode::Syntax, "unknown evidence kind"),
        ));
    }
    report.rulebook_files = match bundle.file_snapshots(&checked.inputs) {
        Ok(files) => files,
        Err(_) => return ControlFlow::Break(failed(report, "rulebook snapshot unavailable")),
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
    state.report = report;
    ControlFlow::Continue((state, checked, model_work))
}
