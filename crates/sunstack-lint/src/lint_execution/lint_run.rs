use crate::{
    configuration_loading::{
        config_decoding::AdmissionPurpose,
        config_validation::{CheckedRulebook, admit_definitions_for},
        definition_bundle::DefinitionBundle,
    },
    lint_execution::stopped_stages::{RunStage, RunStageStopped},
    *,
};
use std::{ops::ControlFlow, path::PathBuf};
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

/// How a phase that runs blocking stages ends early: with a domain report, or with a
/// stage whose task could not join.
pub(super) type RunEnd = Result<LintReport, RunStageStopped>;

async fn run_internal(
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
    mode: ExecutionMode,
) -> LintReport {
    let listing_requested = request.evidence_listing.is_some();
    let report = match super::run_configuration::acquire_file_run(request, cancellation, mode).await
    {
        ControlFlow::Continue((state, cwd, bundle, checked)) => {
            execute_definitions(state, cwd, bundle, checked, api_key, source)
                .await
                .unwrap_or_else(RunStageStopped::into_cli_report)
        }
        ControlFlow::Break(report) => report,
    };
    if listing_requested && report.evidence_listing.is_none() {
        report.with_evidence_listing(vec![])
    } else {
        report
    }
}

/// Run already captured definitions against an explicit absolute checked root.
///
/// The caller owns definition capture; nothing here rereads a definition file,
/// consults the process working directory or reads process credentials. A stage whose
/// blocking task ends without returning its result is returned as [`RunStageStopped`],
/// never as a domain report.
pub async fn run_definitions(
    bundle: DefinitionBundle,
    root: PathBuf,
    paths: Vec<PathBuf>,
    scope: ScopeChoice,
    cancellation: CancellationToken,
    api_key: Option<String>,
) -> Result<LintReport, RunStageStopped> {
    let request = LintRequest {
        config_path: None,
        root: Some(root.clone()),
        paths,
        scope,
        max_warnings: None,
        evidence_listing: None,
    };
    run_definition_request(
        bundle,
        root,
        request,
        cancellation,
        api_key,
        AdmissionPurpose::RuleEvaluation,
    )
    .await
}

/// Inspect captured definitions through the same listing path as CLI evidence work.
/// Only this entry permits zero rulebooks; it never evaluates a rule.
pub async fn inspect_definitions(
    bundle: DefinitionBundle,
    root: PathBuf,
    paths: Vec<PathBuf>,
    scope: ScopeChoice,
    evidence_kind: Option<sunstack_evidence::EvidenceKindName>,
    cancellation: CancellationToken,
    api_key: Option<String>,
) -> Result<LintReport, RunStageStopped> {
    let request = LintRequest {
        config_path: None,
        root: Some(root.clone()),
        paths,
        scope,
        max_warnings: None,
        evidence_listing: Some(evidence_kind),
    };
    let report = run_definition_request(
        bundle,
        root,
        request,
        cancellation,
        api_key,
        AdmissionPurpose::EvidenceInspection,
    )
    .await?;
    Ok(if report.evidence_listing.is_none() {
        report.with_evidence_listing(vec![])
    } else {
        report
    })
}

async fn run_definition_request(
    bundle: DefinitionBundle,
    root: PathBuf,
    request: LintRequest,
    cancellation: CancellationToken,
    api_key: Option<String>,
    purpose: AdmissionPurpose,
) -> Result<LintReport, RunStageStopped> {
    // Named paths check those files in full, exactly as the CLI reports them.
    let mut report = LintReport::empty(
        bundle.root.origin.cli_path(),
        if request.paths.is_empty() {
            request.scope.clone()
        } else {
            ScopeChoice::All
        },
        bundle
            .settings
            .limits
            .as_ref()
            .map(EffectiveLimits::from)
            .unwrap_or_default(),
    );
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return Ok(report);
    }
    let admitted = tokio::task::spawn_blocking(move || {
        let checked = admit_definitions_for(&bundle, purpose);
        (bundle, checked)
    })
    .await;
    let (bundle, checked) = match admitted {
        Ok(value) => value,
        Err(_) => {
            return Err(RunStageStopped::before_dispatch(
                RunStage::Admission,
                report,
            ));
        }
    };
    // Admission runs to completion once started; an abort during it ends the run cancelled
    // before its result is used or any checked input is selected.
    if cancellation.is_cancelled() {
        report.status = LintStatus::Cancelled;
        return Ok(report);
    }
    let state = RunState {
        request,
        cancellation,
        preparing: false,
        recorded_selections: vec![],
        report,
    };
    execute_definitions(state, root, bundle, checked, api_key, None).await
}

/// The one execution path for admitted definitions, shared by the CLI and the SDK.
async fn execute_definitions(
    state: RunState,
    cwd: PathBuf,
    bundle: DefinitionBundle,
    checked: Result<CheckedRulebook, Vec<LintDiagnostic>>,
    api_key: Option<String>,
    source: Option<Box<dyn CheckedFileSource + Send>>,
) -> RunEnd {
    let (state, mut checked, model_work) =
        match super::run_configuration::configure_admitted_run(state, &bundle, checked) {
            ControlFlow::Continue(configured) => configured,
            ControlFlow::Break(report) => return Ok(report),
        };
    let (state, mut scope) =
        match super::run_scope::select_run_scope(state, cwd, &bundle, &checked.inputs, model_work)
            .await
        {
            ControlFlow::Continue(scoped) => scoped,
            ControlFlow::Break(end) => return end,
        };
    let backend = match super::backend_factory::prepare_run_backend(
        &bundle,
        model_work,
        state.preparing,
        api_key.as_deref(),
    ) {
        Ok(backend) => backend,
        Err(error) => return Ok(invalid(state.report, error)),
    };
    let (state, evidence, source) = match super::run_evidence::build_run_evidence(
        state,
        &bundle,
        &mut checked,
        &mut scope,
        source,
        backend,
    )
    .await
    {
        ControlFlow::Continue(built) => built,
        ControlFlow::Break(end) => return end,
    };
    let (state, pieces) = match super::run_coverage::pick_run_pieces(
        state,
        &checked.assembly.rules,
        &scope,
        evidence,
        &source,
    ) {
        ControlFlow::Continue(picked) => picked,
        ControlFlow::Break(report) => return Ok(report),
    };
    let report = super::run_evaluation::evaluate_run(state, &bundle, &checked, pieces).await;
    // An injected checked source stays alive through filters and report assembly.
    drop(source);
    Ok(report)
}
