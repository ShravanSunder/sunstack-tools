use crate::{
    backend_factory::LintBackendFactory,
    config_validation::checked_entries,
    invocation::{InvocationPreparation, InvocationSetupError, prepare_invocation},
    outcome::project_outcome,
    *,
};
use std::{path::PathBuf, sync::Arc};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::RuleExecutionContext;
use tokio_util::sync::CancellationToken;

fn invalid(mut report: LintReport, error: LintInvalid) -> LintReport {
    report.status = LintStatus::Invalid;
    report.diagnostics.push(error.into());
    report
}
fn failed(mut report: LintReport) -> LintReport {
    report.status = LintStatus::Incomplete;
    report.incomplete.push(IncompletePart {
        rule: None,
        file: None,
        reason: IncompleteReason::PreparationFailed,
    });
    report
}
pub async fn run(request: LintRequest, cancellation: CancellationToken) -> LintReport {
    let mut report = LintReport::empty(
        request
            .config_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("sunstack.lint.json")),
        request.scope.clone(),
        EffectiveLimits::default(),
    );
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => {
            return invalid(
                report,
                LintInvalid::at(
                    "sunstack.lint.json",
                    LintInvalidCode::NoConfigFound,
                    "Cannot resolve the current directory.",
                ),
            );
        }
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
        Ok(Ok(loaded)) => loaded,
        Ok(Err(error)) => return invalid(report, error),
        Err(_) => return failed(report),
    };
    report.config = location.file;
    report.limits = config
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return report;
    }
    let entries = match checked_entries(&config, cancellation.clone()).await {
        Ok(entries) => entries,
        Err(diagnostics) => {
            report.status = if cancellation.is_cancelled() {
                LintStatus::Cancelled
            } else {
                LintStatus::Invalid
            };
            report.diagnostics = diagnostics;
            return report;
        }
    };
    let selection_config = config.clone();
    let paths = request.paths.clone();
    let selected = tokio::task::spawn_blocking(move || {
        targets::select_targets(&selection_config, &paths, &cwd)
    })
    .await;
    let selection = match selected {
        Ok(Ok(selection)) => selection,
        Ok(Err(error)) => return invalid(report, error),
        Err(_) => return failed(report),
    };
    let preparation = prepare_invocation(
        &config,
        &entries,
        selection,
        &request,
        &report.limits,
        cancellation.clone(),
    )
    .await;
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return report;
    }
    match preparation {
        Ok(InvocationPreparation::NothingSelected { exclusions, scope }) => {
            report.exclusions = exclusions;
            report.scope = scope;
            report
        }
        Ok(InvocationPreparation::Ready {
            input,
            policy,
            targets,
            exclusions,
            scope,
        }) => {
            report.exclusions = exclusions;
            report.scope = scope;
            let adapters = match AdapterRegistry::builtins() {
                Ok(adapters) => adapters,
                Err(_) => return failed(report),
            };
            let context = RuleExecutionContext {
                backend: Arc::new(LintBackendFactory {
                    selection: input.backend.clone(),
                    configured: config.backend.clone(),
                }),
                adapters,
                cancellation,
            };
            let evaluation =
                sunstack_rules::evaluate_rules_with_policy(*input, policy, context).await;
            project_outcome(&config, evaluation, report, &targets, request.max_warnings)
        }
        Err(InvocationSetupError::Invalid(error)) => invalid(report, error),
        Err(InvocationSetupError::Incomplete(message)) => {
            let mut report = failed(report);
            report.diagnostics.push(LintDiagnostic {
                file: Some(report.config.clone()),
                line: None,
                column: None,
                code: LintInvalidCode::Syntax,
                message,
            });
            report
        }
    }
}
