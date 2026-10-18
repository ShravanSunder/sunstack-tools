use super::*;
use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};
use sunstack_decisions::{
    AttemptSettings, DecisionRunOutcome, RequestBudgetSettings, run_admitted_judgments,
};
use sunstack_evidence::EvidenceSession;
use tokio::{sync::Semaphore, task::JoinSet};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
#[derive(Default)]
pub(crate) struct SchedulerMeasurements {
    pub local_elapsed: Duration,
    pub provider_elapsed: Duration,
    pub model_requests: usize,
    pub inference_attempts: usize,
}
pub(crate) struct SchedulerInput<'invocation> {
    pub plan: &'invocation mut ExecutionPlan,
    pub state: &'invocation mut ExecutionState,
    pub session: &'invocation mut EvidenceSession,
    pub origin: &'invocation InvocationOrigin,
    pub backend: Arc<dyn RuleBackendFactory>,
    pub attempts: AttemptSettings,
    pub budget: RequestBudgetSettings,
    pub max_concurrent_requests: usize,
    pub cancellation: CancellationToken,
}
pub(crate) enum SchedulerFailure {
    Backend { diagnostics: DiagnosticList },
    Execution { diagnostics: DiagnosticList },
}
struct RequestCompletion {
    id: RequestId,
    outcome: DecisionRunOutcome,
    bindings: Vec<QuestionBinding>,
    units: Vec<(UnitKey, sunstack_decisions::QuestionName)>,
    elapsed: Duration,
}
fn attempt_count(outcome: &DecisionRunOutcome) -> usize {
    match outcome {
        DecisionRunOutcome::Completed { attempts, .. }
        | DecisionRunOutcome::ProviderFailure { attempts, .. }
        | DecisionRunOutcome::InvalidResponse { attempts, .. }
        | DecisionRunOutcome::Cancelled { attempts, .. } => attempts.len(),
        DecisionRunOutcome::InvalidRequest { .. } => 0,
    }
}
/// One scheduler owns mutable plan state. Worker futures own only immutable admitted bytes and the backend.
pub(crate) async fn execute_plan(
    input: SchedulerInput<'_>,
) -> Result<SchedulerMeasurements, (SchedulerFailure, SchedulerMeasurements)> {
    let SchedulerInput {
        plan,
        state,
        session,
        origin,
        backend,
        attempts,
        budget,
        max_concurrent_requests,
        cancellation,
    } = input;
    let mut measurements = SchedulerMeasurements::default();
    let tracker = TaskTracker::new();
    let semaphore = Arc::new(Semaphore::new(max_concurrent_requests));
    let mut tasks = JoinSet::new();
    let mut fatal = None;
    let mut split_encodings = BTreeMap::<RequestId, (Vec<UnitKey>, EncodedRequest)>::new();
    let mut omitted_optionals = BTreeSet::new();
    let mut backend_session = crate::rule_backend_factory::RuleBackendSession::new(backend);
    'scheduler: loop {
        let local_started = Instant::now();
        let mut changed = advance_entries(plan, state, origin, session, &cancellation).await;
        changed |= discover_ready_rules(plan, state, session, &cancellation).await;
        finish_empty_carriers(plan, state);
        for position in 0..plan.requests.len() {
            let request = plan.requests[position].clone();
            if !matches!(
                state.requests.get(&request.id),
                Some(RequestSettlement::Waiting)
            ) {
                continue;
            }
            if cancellation.is_cancelled() {
                mark_not_sent(plan, state, &request, RequestRefusal::Cancelled);
                changed = true;
                continue;
            }
            match request_readiness(plan, state, &request) {
                RequestReadiness::Waiting => continue,
                RequestReadiness::Stopped { entry } => {
                    mark_not_sent(plan, state, &request, RequestRefusal::Gate { entry });
                    changed = true;
                    continue;
                }
                RequestReadiness::Ready => {}
            }
            // Local checks consume neither a model slot nor a request budget.
            // Keep legacy judgment acquisition behind those same boundaries.
            let mut local_request = request.clone();
            local_request
                .units
                .retain(|unit| plan.unit_is_mechanical(*unit));
            if !local_request.units.is_empty() {
                prepare_units(plan, state, &local_request, session, &cancellation).await;
            }
            finish_empty_carriers(plan, state);
            if matches!(
                state.requests.get(&request.id),
                Some(RequestSettlement::Settled {
                    status: RequestStatus::NotSent {
                        reason: NotSentReason::NoQuestions
                    },
                    ..
                })
            ) {
                changed = true;
                continue;
            }
            if measurements.model_requests >= budget.max_requests {
                mark_not_sent(plan, state, &request, RequestRefusal::Limit);
                changed = true;
                continue;
            }
            let Ok(permit) = semaphore.clone().try_acquire_owned() else {
                continue;
            };
            let units = prepare_units(plan, state, &request, session, &cancellation).await;
            if cancellation.is_cancelled() {
                mark_not_sent(plan, state, &request, RequestRefusal::Cancelled);
                drop(permit);
                changed = true;
                continue;
            }
            if units.is_empty() {
                let diagnostics = preparation_diagnostics(state, &request.units);
                mark_not_sent(
                    plan,
                    state,
                    &request,
                    if request_outside_scope(state, &request.units) {
                        RequestRefusal::OutsideChangeScope
                    } else {
                        RequestRefusal::Evidence { diagnostics }
                    },
                );
                drop(permit);
                changed = true;
                continue;
            }
            let backend = match backend_session.resolve() {
                Ok(backend) => backend,
                Err(diagnostics) => {
                    fatal = Some(SchedulerFailure::Backend { diagnostics });
                    drop(permit);
                    break 'scheduler;
                }
            };
            let mut omit_optional = omitted_optionals.contains(&request.id);
            let mut admission = match split_encodings.remove(&request.id) {
                Some((prepared_units, encoded)) if prepared_units == units => {
                    RequestAdmission::Within(encoded)
                }
                _ => encode_units(
                    plan,
                    state,
                    &units,
                    omit_optional,
                    backend.as_ref(),
                    &budget,
                ),
            };
            if matches!(admission, RequestAdmission::Exceeded(_))
                && !omit_optional
                && has_optional(state, &units)
            {
                omit_optional = true;
                omitted_optionals.insert(request.id);
                admission = encode_units(plan, state, &units, true, backend.as_ref(), &budget);
            }
            if matches!(admission, RequestAdmission::Exceeded(_))
                && matches!(request.purpose, RequestPurpose::Rules { .. })
                && units.iter().any(|unit| matches!(unit, UnitKey::Rule(_)))
            {
                let parts = split_rules_request(
                    plan,
                    state,
                    &units,
                    omit_optional,
                    backend.as_ref(),
                    &budget,
                );
                for (id, units, encoded) in install_budget_parts(plan, state, &request, parts) {
                    if omit_optional {
                        omitted_optionals.insert(id);
                    }
                    split_encodings.insert(id, (units, encoded));
                }
                drop(permit);
                changed = true;
                continue;
            }
            let encoded = match admission {
                RequestAdmission::Within(encoded) => encoded,
                RequestAdmission::Exceeded(diagnostics)
                | RequestAdmission::Unverifiable(diagnostics)
                | RequestAdmission::Invalid(diagnostics) => {
                    for unit in &units {
                        state.budget_refusals.insert(*unit, diagnostics.clone());
                    }
                    mark_not_sent(
                        plan,
                        state,
                        &request,
                        RequestRefusal::Budget { diagnostics },
                    );
                    drop(permit);
                    changed = true;
                    continue;
                }
            };
            let backend = backend.clone();
            let attempts = attempts.clone();
            let cancellation = cancellation.clone();
            let id = request.id;
            state.requests.insert(id, RequestSettlement::InFlight);
            tasks.spawn(tracker.track_future(async move {
                let started = Instant::now();
                let outcome = run_admitted_judgments(
                    &encoded.request,
                    backend.as_ref(),
                    &encoded.admitted,
                    &attempts,
                    &cancellation,
                )
                .await;
                drop(permit);
                RequestCompletion {
                    id,
                    outcome,
                    bindings: encoded.bindings,
                    units: encoded.units,
                    elapsed: started.elapsed(),
                }
            }));
            measurements.model_requests += 1;
            changed = true;
        }
        measurements.local_elapsed = measurements
            .local_elapsed
            .saturating_add(local_started.elapsed());
        let waiting = state
            .expansions
            .iter()
            .any(|expansion| matches!(expansion, ExpansionState::Waiting))
            || state
                .requests
                .values()
                .any(|request| matches!(request, RequestSettlement::Waiting));
        if tasks.is_empty() {
            if !waiting {
                break;
            }
            if changed {
                continue;
            }
            fatal = Some(SchedulerFailure::Execution {
                diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
            });
            break;
        }
        if changed && waiting && semaphore.available_permits() > 0 {
            continue;
        }
        let completion = if cancellation.is_cancelled() {
            tasks.join_next().await
        } else {
            tokio::select! {biased;_=cancellation.cancelled()=>{continue},completion=tasks.join_next()=>completion}
        };
        match completion {
            Some(Ok(completion)) => {
                measurements.provider_elapsed = measurements
                    .provider_elapsed
                    .saturating_add(completion.elapsed);
                measurements.inference_attempts += attempt_count(&completion.outcome);
                settle_request(
                    plan,
                    state,
                    completion.id,
                    completion.outcome,
                    completion.bindings,
                    completion.units,
                );
            }
            Some(Err(_)) => {
                fatal = Some(SchedulerFailure::Execution {
                    diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                });
                cancellation.cancel();
            }
            None => {}
        }
    }
    tracker.close();
    while let Some(completion) = tasks.join_next().await {
        match completion {
            Ok(completion) => {
                measurements.provider_elapsed = measurements
                    .provider_elapsed
                    .saturating_add(completion.elapsed);
                measurements.inference_attempts += attempt_count(&completion.outcome);
                settle_request(
                    plan,
                    state,
                    completion.id,
                    completion.outcome,
                    completion.bindings,
                    completion.units,
                );
            }
            Err(_) => {
                fatal = Some(SchedulerFailure::Execution {
                    diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                });
            }
        }
    }
    tracker.wait().await;
    let started = Instant::now();
    advance_entries(plan, state, origin, session, &cancellation).await;
    discover_ready_rules(plan, state, session, &cancellation).await;
    measurements.local_elapsed = measurements.local_elapsed.saturating_add(started.elapsed());
    match fatal {
        Some(diagnostics) => Err((diagnostics, measurements)),
        None => Ok(measurements),
    }
}
