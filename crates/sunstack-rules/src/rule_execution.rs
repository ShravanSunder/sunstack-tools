use crate::{invocation_admission::admit_invocation, request_planning::*, *};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use sunstack_evidence::{AdapterRegistry, EvidenceMeasurements, EvidenceSession};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct RuleExecutionContext {
    pub backend: Arc<dyn RuleBackendFactory>,
    pub adapters: AdapterRegistry,
    pub cancellation: CancellationToken,
}
fn cancelled_before(measurements: InvocationMeasurements) -> RuleInvocationOutcome {
    RuleInvocationOutcome::Cancelled {
        progress: InvocationCancellationProgress::BeforeReportAdmission,
        diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
        measurements,
    }
}
fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
pub async fn evaluate_rules_with_policy(
    input: RuleInvocationInput,
    policy: RulePolicy,
    context: RuleExecutionContext,
) -> PolicyEvaluation {
    let mut request_evidence = Default::default();
    let outcome = evaluate_with_evidence(
        input,
        policy,
        context,
        &mut request_evidence,
        &mut Default::default(),
        None,
    )
    .await;
    PolicyEvaluation {
        outcome,
        request_evidence,
    }
}
pub async fn evaluate_rules(
    input: RuleInvocationInput,
    context: RuleExecutionContext,
) -> RuleInvocationOutcome {
    evaluate_rules_with_policy(input, RulePolicy::default(), context)
        .await
        .outcome
}
pub(crate) async fn evaluate_with_evidence(
    input: RuleInvocationInput,
    policy: RulePolicy,
    context: RuleExecutionContext,
    request_evidence: &mut std::collections::BTreeMap<
        usize,
        sunstack_decisions::SharedEvidenceState,
    >,
    candidate_data: &mut crate::candidate_execution_data::CandidateExecutionData,
    candidate_admission: Option<crate::candidate_admission::CandidateAdmission>,
) -> RuleInvocationOutcome {
    let started = Instant::now();
    let mut measurements = InvocationMeasurements::default();
    let cancellation = context.cancellation.child_token();
    // Dropping this invocation signals its owned work without cancelling the caller's token.
    let _cancellation_guard = cancellation.clone().drop_guard();
    if cancellation.is_cancelled() {
        return cancelled_before(measurements);
    }
    if input.backend != *context.backend.selection() {
        return RuleInvocationOutcome::Invalid {
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration),
        };
    }
    let candidate_run = candidate_admission.is_some();
    let (admission, opened_session) = match candidate_admission {
        Some(admitted) => (Ok(admitted.invocation), Some(admitted.session)),
        None => (
            admit_invocation(&input, &context.adapters, &cancellation).await,
            None,
        ),
    };
    let admitted = match admission {
        Ok(admitted) => admitted,
        Err(diagnostics) => {
            measurements.local_elapsed_ms = duration_ms(started.elapsed());
            return if cancellation.is_cancelled() {
                cancelled_before(measurements)
            } else {
                RuleInvocationOutcome::Invalid { diagnostics }
            };
        }
    };
    let policy = match policy.resolve(&input.definition_root, &admitted.groups) {
        Ok(policy) => policy,
        Err(diagnostics) => return RuleInvocationOutcome::Invalid { diagnostics },
    };
    let mut session = if let Some(session) = opened_session {
        session
    } else {
        match EvidenceSession::open(admitted.evidence, context.adapters, cancellation.clone()).await
        {
            Ok(session) => session,
            Err(error) => {
                measurements.local_elapsed_ms = duration_ms(started.elapsed());
                return if cancellation.is_cancelled() {
                    cancelled_before(measurements)
                } else {
                    RuleInvocationOutcome::Invalid {
                        diagnostics: error.diagnostics().into(),
                    }
                };
            }
        }
    };
    let mut plan = definition_topology_with_policy(admitted.groups, admitted.router, &policy);
    plan_change_selection(&mut plan, &mut session).await;
    precompute_conditions(
        &mut plan,
        &input.origin,
        &mut session,
        &cancellation,
        input.execution.speculation,
    )
    .await;
    build_request_graph(&mut plan, &input.execution);
    measurements.local_elapsed_ms = duration_ms(started.elapsed());
    if cancellation.is_cancelled() {
        copy_evidence_measurements(&mut measurements, session.close().measurements);
        return cancelled_before(measurements);
    }
    // Admission of the report fixes occurrence and request identities before dispatch.
    let mut state = ExecutionState::new(&plan);
    let execution = execute_plan(SchedulerInput {
        plan: &mut plan,
        state: &mut state,
        session: &mut session,
        origin: &input.origin,
        backend: context.backend,
        attempts: input.attempts,
        budget: input.request_budget,
        max_concurrent_requests: input.execution.max_concurrent_requests.get(),
        cancellation: cancellation.clone(),
    })
    .await;
    let (scheduler, failure) = match execution {
        Ok(scheduler) => (scheduler, None),
        Err((diagnostics, scheduler)) => (scheduler, Some(diagnostics)),
    };
    if candidate_run {
        candidate_data.elapsed_ms = scheduler.candidate_elapsed_ms;
        crate::candidate_execution_data::project_evidence_data(&plan, &state, candidate_data);
    }
    measurements.local_elapsed_ms = measurements
        .local_elapsed_ms
        .saturating_add(duration_ms(scheduler.local_elapsed));
    measurements.provider_elapsed_ms = duration_ms(scheduler.provider_elapsed);
    measurements.model_requests = scheduler.model_requests;
    measurements.inference_attempts = scheduler.inference_attempts;
    if candidate_run {
        candidate_data.outside_privacy = session.privacy_projection().clone();
    }
    let closed = session.close();
    copy_evidence_measurements(&mut measurements, closed.measurements);
    *request_evidence = project_request_evidence(&plan, &state);
    if let Some(diagnostics) = failure {
        return match diagnostics {
            SchedulerFailure::Backend { diagnostics } => {
                RuleInvocationOutcome::Invalid { diagnostics }
            }
            SchedulerFailure::Execution { diagnostics } => {
                RuleInvocationOutcome::PreparationFailed {
                    diagnostics,
                    measurements,
                }
            }
        };
    }
    let mut report = project_report(&plan, &state, &input.origin, measurements);
    report.captures = closed.captures;
    report.change_selections = closed.change_selections;
    report.cursors = closed.cursors;
    if cancellation.is_cancelled() {
        let measurements = report.measurements.clone();
        RuleInvocationOutcome::Cancelled {
            progress: InvocationCancellationProgress::ReportAdmitted {
                partial_report: Box::new(report),
            },
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
            measurements,
        }
    } else if report.is_complete() {
        RuleInvocationOutcome::CompleteReport { report }
    } else {
        RuleInvocationOutcome::IncompleteReport { report }
    }
}
pub(crate) fn copy_evidence_measurements(
    target: &mut InvocationMeasurements,
    evidence: EvidenceMeasurements,
) {
    target.files_considered = evidence.files_considered;
    target.files_read = evidence.files_read;
    target.files_parsed = evidence.files_parsed;
    target.transcript_bytes_read = evidence.transcript_bytes_read;
    target.transcript_bytes_parsed = evidence.transcript_bytes_parsed;
    target.bytes_scanned = evidence.bytes_scanned;
    target.bytes_included = evidence.bytes_included;
}
