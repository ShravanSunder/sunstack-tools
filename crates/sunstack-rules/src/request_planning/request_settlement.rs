use super::*;
use crate::*;
use sunstack_decisions::{DecisionRunOutcome, QuestionName};
#[derive(Clone)]
pub(crate) enum RequestRefusal {
    OutsideChangeScope,
    Gate { entry: EntryIndex },
    Cancelled,
    Budget { diagnostics: DiagnosticList },
    Evidence { diagnostics: DiagnosticList },
    Limit,
}
pub(crate) fn mark_not_sent(
    plan: &ExecutionPlan,
    state: &mut ExecutionState,
    request: &PlannedRequest,
    refusal: RequestRefusal,
) {
    let failure = match &refusal {
        RequestRefusal::Gate { entry } => Some(FailureCause::Gate { entry: *entry }),
        RequestRefusal::Cancelled | RequestRefusal::OutsideChangeScope => None,
        RequestRefusal::Budget { diagnostics } => Some(FailureCause::Budget {
            diagnostics: diagnostics.clone(),
        }),
        RequestRefusal::Evidence { diagnostics } => Some(FailureCause::Evidence {
            diagnostics: diagnostics.clone(),
        }),
        RequestRefusal::Limit => Some(FailureCause::Budget {
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::RequestLimit),
        }),
    };
    for unit in &request.units {
        if let UnitKey::Gate(index) = unit {
            if plan.controls.get(index) != Some(&request.id) {
                continue;
            }
            if matches!(state.gates.get(index), Some(GateSettlement::Pending)) {
                state.gates.insert(
                    *index,
                    match &failure {
                        Some(cause) => GateSettlement::Failed {
                            cause: cause.clone(),
                        },
                        None => GateSettlement::Cancelled,
                    },
                );
            }
            if matches!(state.routers.get(index), Some(RouterSettlement::Pending)) {
                state.routers.insert(
                    *index,
                    match &failure {
                        Some(cause) => RouterSettlement::Failed {
                            cause: cause.clone(),
                        },
                        None => RouterSettlement::Cancelled,
                    },
                );
            }
        }
    }
    let reason = match refusal {
        RequestRefusal::Gate { entry } => NotSentReason::GateStopped {
            entry: plan.entries[entry.0].key.clone(),
        },
        RequestRefusal::Cancelled => NotSentReason::Cancelled,
        RequestRefusal::OutsideChangeScope => NotSentReason::OutsideChangeScope,
        RequestRefusal::Budget { diagnostics } => NotSentReason::BudgetRefused { diagnostics },
        RequestRefusal::Evidence { diagnostics } => NotSentReason::EvidenceRefused { diagnostics },
        RequestRefusal::Limit => NotSentReason::RequestLimit,
    };
    state.requests.insert(
        request.id,
        RequestSettlement::Settled {
            status: RequestStatus::NotSent { reason },
            bindings: vec![],
            unit_bindings: vec![],
        },
    );
}
pub(crate) fn settle_request(
    plan: &ExecutionPlan,
    state: &mut ExecutionState,
    id: RequestId,
    outcome: DecisionRunOutcome,
    bindings: Vec<QuestionBinding>,
    units: Vec<(UnitKey, QuestionName)>,
) {
    for (unit, name) in &units {
        let UnitKey::Gate(index) = unit else { continue };
        if plan.controls.get(index) != Some(&id) {
            continue;
        }
        let reference = AnswerReference {
            request: id,
            question: name.clone(),
        };
        if matches!(state.gates.get(index), Some(GateSettlement::Pending)) {
            state.gates.insert(
                *index,
                match &outcome {
                    DecisionRunOutcome::Completed { answers, .. } => match answers.get(name) {
                        Some(answer) => GateSettlement::Answered {
                            reference: reference.clone(),
                            answer: answer.clone(),
                        },
                        None => GateSettlement::Failed {
                            cause: FailureCause::Request { request: id },
                        },
                    },
                    DecisionRunOutcome::Cancelled { .. } => GateSettlement::Cancelled,
                    _ => GateSettlement::Failed {
                        cause: FailureCause::Request { request: id },
                    },
                },
            );
        }
        if matches!(state.routers.get(index), Some(RouterSettlement::Pending)) {
            state.routers.insert(
                *index,
                match &outcome {
                    DecisionRunOutcome::Completed { answers, .. } => match answers.get(name) {
                        Some(answer) => RouterSettlement::Answered {
                            reference: reference.clone(),
                            answer: answer.clone(),
                        },
                        None => RouterSettlement::Failed {
                            cause: FailureCause::Request { request: id },
                        },
                    },
                    DecisionRunOutcome::Cancelled { .. } => RouterSettlement::Cancelled,
                    _ => RouterSettlement::Failed {
                        cause: FailureCause::Request { request: id },
                    },
                },
            );
        }
    }
    state.requests.insert(
        id,
        RequestSettlement::Settled {
            status: RequestStatus::Sent { outcome },
            bindings,
            unit_bindings: units,
        },
    );
}
