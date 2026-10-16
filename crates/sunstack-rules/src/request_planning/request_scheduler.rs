use super::*;
use crate::*;
use std::{sync::Arc,time::{Duration,Instant}};
use sunstack_decisions::{DecisionBackend,DecisionRunOutcome,AttemptSettings,RequestBudgetSettings,run_admitted_judgments};
use sunstack_evidence::EvidenceSession;
use tokio::{sync::Semaphore,task::JoinSet};
use tokio_util::{sync::CancellationToken,task::TaskTracker};
#[derive(Default)]
pub(crate) struct SchedulerMeasurements {pub local_elapsed:Duration,pub provider_elapsed:Duration,pub model_requests:usize,pub inference_attempts:usize}
pub(crate) struct SchedulerInput<'invocation> {
    pub plan:&'invocation mut ExecutionPlan,pub state:&'invocation mut ExecutionState,pub session:&'invocation mut EvidenceSession,
    pub origin:&'invocation InvocationOrigin,pub backend:Arc<dyn DecisionBackend>,pub attempts:AttemptSettings,pub budget:RequestBudgetSettings,
    pub max_concurrent_requests:usize,pub cancellation:CancellationToken,
}
struct RequestCompletion {id:RequestId,outcome:DecisionRunOutcome,bindings:Vec<QuestionBinding>,units:Vec<(UnitKey,sunstack_decisions::QuestionName)>,elapsed:Duration}
fn attempt_count(outcome:&DecisionRunOutcome)->usize {match outcome {DecisionRunOutcome::Completed{attempts,..}|DecisionRunOutcome::ProviderFailure{attempts,..}|DecisionRunOutcome::InvalidResponse{attempts,..}|DecisionRunOutcome::Cancelled{attempts,..}=>attempts.len(),DecisionRunOutcome::InvalidRequest{..}=>0}}
/// One scheduler owns mutable plan state. Worker futures own only immutable admitted bytes and the backend.
pub(crate) async fn execute_plan(input:SchedulerInput<'_>)->Result<SchedulerMeasurements,(DiagnosticList,SchedulerMeasurements)> {
    let SchedulerInput{plan,state,session,origin,backend,attempts,budget,max_concurrent_requests,cancellation}=input;
    let mut measurements=SchedulerMeasurements::default();let tracker=TaskTracker::new();let semaphore=Arc::new(Semaphore::new(max_concurrent_requests));let mut tasks=JoinSet::new();let mut fatal=None;
    loop {
        let local_started=Instant::now();
        let mut changed=advance_entries(plan,state,origin,session,&cancellation).await;
        for position in 0..plan.requests.len() {
            let request=plan.requests[position].clone();
            if !matches!(state.requests.get(&request.id),Some(RequestSettlement::Waiting)){continue}
            if cancellation.is_cancelled() {mark_not_sent(plan,state,&request,RequestRefusal::Cancelled);changed=true;continue}
            match request_readiness(plan,state,&request) {
                RequestReadiness::Waiting=>continue,
                RequestReadiness::Stopped{entry}=>{mark_not_sent(plan,state,&request,RequestRefusal::Gate{entry});changed=true;continue},RequestReadiness::Ready=>{}
            }
            if measurements.model_requests>=budget.max_requests {mark_not_sent(plan,state,&request,RequestRefusal::Limit);changed=true;continue}
            let Ok(permit)=semaphore.clone().try_acquire_owned() else{break};
            let units=prepare_units(plan,state,&request,session,&cancellation).await;
            if cancellation.is_cancelled(){mark_not_sent(plan,state,&request,RequestRefusal::Cancelled);drop(permit);changed=true;continue}
            if units.is_empty(){let diagnostics=preparation_diagnostics(state,&request.units);mark_not_sent(plan,state,&request,RequestRefusal::Evidence{diagnostics});drop(permit);changed=true;continue}
            let mut admission=encode_units(plan,state,&units,false,backend.as_ref(),&budget);
            if matches!(admission,RequestAdmission::Exceeded(_)) && has_optional(state,&units){admission=encode_units(plan,state,&units,true,backend.as_ref(),&budget);}
            let encoded=match admission {
                RequestAdmission::Within(encoded)=>encoded,
                RequestAdmission::Exceeded(diagnostics)|RequestAdmission::Unverifiable(diagnostics)|RequestAdmission::Invalid(diagnostics)=>{
                    for unit in &units {state.budget_refusals.insert(*unit,diagnostics.clone());}
                    mark_not_sent(plan,state,&request,RequestRefusal::Budget{diagnostics});drop(permit);changed=true;continue
                }
            };
            // The budget splitter replaces this single ready node before dispatch when needed.
            let backend=backend.clone();let attempts=attempts.clone();let cancellation=cancellation.clone();let id=request.id;
            state.requests.insert(id,RequestSettlement::InFlight);
            tasks.spawn(tracker.track_future(async move {
                let started=Instant::now();let outcome=run_admitted_judgments(&encoded.request,backend.as_ref(),&encoded.admitted,&attempts,&cancellation).await;
                drop(permit);RequestCompletion{id,outcome,bindings:encoded.bindings,units:encoded.units,elapsed:started.elapsed()}
            }));
            measurements.model_requests+=1;changed=true;
        }
        measurements.local_elapsed=measurements.local_elapsed.saturating_add(local_started.elapsed());
        let waiting=state.requests.values().any(|request|matches!(request,RequestSettlement::Waiting));
        if tasks.is_empty() {
            if !waiting {break}
            if changed {continue}
            fatal=Some(DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError));break
        }
        let completion=if cancellation.is_cancelled(){tasks.join_next().await}else{
            tokio::select!{biased;_=cancellation.cancelled()=>{continue},completion=tasks.join_next()=>completion}
        };
        match completion {
            Some(Ok(completion))=>{
                measurements.provider_elapsed=measurements.provider_elapsed.saturating_add(completion.elapsed);measurements.inference_attempts+=attempt_count(&completion.outcome);
                settle_request(plan,state,completion.id,completion.outcome,completion.bindings,completion.units);
            }
            Some(Err(_))=>{fatal=Some(DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError));cancellation.cancel();},None=>{}
        }
    }
    tracker.close();
    while let Some(completion)=tasks.join_next().await {match completion{Ok(completion)=>{measurements.provider_elapsed=measurements.provider_elapsed.saturating_add(completion.elapsed);measurements.inference_attempts+=attempt_count(&completion.outcome);settle_request(plan,state,completion.id,completion.outcome,completion.bindings,completion.units);},Err(_)=>{fatal=Some(DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError));}}}
    tracker.wait().await;
    let started=Instant::now();advance_entries(plan,state,origin,session,&cancellation).await;measurements.local_elapsed=measurements.local_elapsed.saturating_add(started.elapsed());
    match fatal{Some(diagnostics)=>Err((diagnostics,measurements)),None=>Ok(measurements)}
}
