use crate::*;
use std::ops::ControlFlow;
use sunstack_evidence::CheckedFileSource;
use tokio_util::sync::CancellationToken;

pub(super) fn invalid(mut report: LintReport, error: LintInvalid) -> LintReport {
    report.status = LintStatus::Invalid;
    report.diagnostics.push(error.into());
    report
}
pub(super) fn failed(mut report: LintReport, message: &str) -> LintReport {
    report.status = LintStatus::Incomplete;
    report.incomplete.push(IncompletePart {
        rule: None,
        file: None,
        field: None,
        reason: IncompleteReason::PreparationFailed,
    });
    report.diagnostics.push(LintDiagnostic {
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
pub(super) enum ExecutionMode {
    Ordinary,
    Prepare {
        recorded_selections: Vec<sunstack_rules::RecordedSelection>,
    },
}

// Only controls and the accumulating report cross every phase. Configuration,
// scope, and evidence resources stay explicit in the phase calls.
pub(super) struct RunState {
    pub(super) request: LintRequest,
    pub(super) cancellation: CancellationToken,
    pub(super) preparing: bool,
    pub(super) recorded_selections: Vec<sunstack_rules::RecordedSelection>,
    pub(super) report: LintReport,
}

async fn run_internal(
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
    mode: ExecutionMode,
) -> LintReport {
    let (state, cwd, location, config, mut checked, model_work) =
        match super::run_configuration::configure_run(request, cancellation, mode).await {
            ControlFlow::Continue(configured) => configured,
            ControlFlow::Break(report) => return report,
        };
    let (state, mut scope) = match super::run_scope::select_run_scope(
        state,
        cwd,
        &location,
        &config,
        &checked.inputs,
        model_work,
    )
    .await
    {
        ControlFlow::Continue(scoped) => scoped,
        ControlFlow::Break(report) => return report,
    };
    let backend = match super::backend_factory::prepare_run_backend(
        &config,
        &location,
        model_work,
        state.preparing,
        api_key.as_deref(),
    ) {
        Ok(backend) => backend,
        Err(error) => return invalid(state.report, error),
    };
    let (state, evidence, source) = match super::run_evidence::build_run_evidence(
        state,
        &config,
        &mut checked,
        &mut scope,
        source,
        backend,
    )
    .await
    {
        ControlFlow::Continue(built) => built,
        ControlFlow::Break(report) => return report,
    };
    let (state, pieces) = match super::run_coverage::pick_run_pieces(
        state,
        &checked.assembly.rules,
        &scope,
        evidence,
        &source,
    ) {
        ControlFlow::Continue(picked) => picked,
        ControlFlow::Break(report) => return report,
    };
    let report = super::run_evaluation::evaluate_run(state, &config, &checked, pieces).await;
    // An injected checked source stays alive through filters and report assembly.
    drop(source);
    report
}
