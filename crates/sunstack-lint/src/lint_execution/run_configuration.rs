use super::lint_run::{ExecutionMode, RunState, failed, invalid};
use crate::{
    configuration_loading::config_validation::{
        CheckedRulebook, load_checked_rulebook, native_diagnostics,
    },
    *,
};
use std::{ops::ControlFlow, path::PathBuf};
use tokio_util::sync::CancellationToken;

pub(super) async fn configure_run(
    request: LintRequest,
    cancellation: CancellationToken,
    mode: ExecutionMode,
) -> ControlFlow<
    LintReport,
    (
        RunState,
        PathBuf,
        ConfigLocation,
        RulesConfig,
        CheckedRulebook,
        bool,
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
    let config_to_check = config.clone();
    let checked =
        match tokio::task::spawn_blocking(move || load_checked_rulebook(&config_to_check)).await {
            Ok(Ok(checked)) => checked,
            Ok(Err(diagnostics)) => {
                report.status = LintStatus::Invalid;
                report.diagnostics = diagnostics;
                return ControlFlow::Break(report);
            }
            Err(_) => return ControlFlow::Break(failed(report, "rulebook loading failed")),
        };
    if let Some(Some(kind)) = &request.evidence_listing
        && !config.evidence.kinds.contains_key(kind)
    {
        return ControlFlow::Break(invalid(
            report,
            LintInvalid::at(
                &location.file,
                LintInvalidCode::Syntax,
                "unknown evidence kind",
            ),
        ));
    }
    report.rulebook_files =
        match crate::lint_reporting::saved_reports::capture_files(&checked.inputs) {
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
    let selected = config
        .evidence
        .kinds
        .values()
        .any(|kind| kind.select.is_some());
    let gated = checked
        .assembly
        .rules
        .iter()
        .any(|rule| rule.gate.is_some());
    let model_work = selected || (request.evidence_listing.is_none() && (asking || gated));
    ControlFlow::Continue((
        RunState {
            request,
            cancellation,
            preparing,
            recorded_selections,
            report,
        },
        cwd,
        location,
        config,
        checked,
        model_work,
    ))
}
