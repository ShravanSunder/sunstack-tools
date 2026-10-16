use super::plan_contracts::*;
use crate::{
    applicability_evaluation::{ConditionContext, condition_basis, evaluate_condition},
    *,
};
use sunstack_decisions::JudgmentAnswer;
use sunstack_evidence::EvidenceSession;
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone)]
pub(crate) enum PrefixState {
    Pending,
    Passed,
    Stopped {
        entry: EntryIndex,
    },
    Failed {
        entry: EntryIndex,
        cause: FailureCause,
    },
    Cancelled {
        entry: EntryIndex,
    },
}
#[derive(Debug, Clone)]
pub(crate) enum RouteState {
    Pending,
    Selected,
    NotSelected,
    Failed { cause: FailureCause },
    Cancelled,
}
#[derive(Debug, Clone)]
pub(crate) enum RequestReadiness {
    Waiting,
    Ready,
    Stopped { entry: EntryIndex },
}
pub(crate) fn route_state(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    routing: &RouteDependency,
) -> RouteState {
    match state.routers.get(&routing.entry) {
        Some(RouterSettlement::Answered {
            answer:
                JudgmentAnswer::Choice {
                    selected_label,
                    confidence,
                    ..
                },
            ..
        }) => {
            let EntryDefinition::Router { definition, .. } =
                &plan.entries[routing.entry.0].definition
            else {
                return RouteState::NotSelected;
            };
            if *confidence >= definition.confidence_floor
                && selected_label == &routing.label
                && definition.routes.contains_key(selected_label)
            {
                RouteState::Selected
            } else {
                RouteState::NotSelected
            }
        }
        Some(RouterSettlement::Answered { reference, .. }) => RouteState::Failed {
            cause: FailureCause::Request {
                request: reference.request,
            },
        },
        Some(RouterSettlement::Failed { cause }) => RouteState::Failed {
            cause: cause.clone(),
        },
        Some(RouterSettlement::Cancelled) => RouteState::Cancelled,
        _ => RouteState::Pending,
    }
}
pub(crate) fn entry_state(index: EntryIndex, state: &ExecutionState) -> PrefixState {
    match &state.entries[index.0] {
        EntrySettlement::Router | EntrySettlement::Pending => PrefixState::Pending,
        EntrySettlement::Condition {
            outcome: ConditionOutcome::Passed,
            ..
        }
        | EntrySettlement::Decision {
            result: DecisionSettlement::Passed { .. },
            ..
        } => PrefixState::Passed,
        EntrySettlement::Condition {
            outcome: ConditionOutcome::DidNotPass,
            ..
        }
        | EntrySettlement::Decision {
            result: DecisionSettlement::DidNotPass { .. },
            ..
        } => PrefixState::Stopped { entry: index },
        EntrySettlement::Condition {
            outcome: ConditionOutcome::Unresolved { diagnostic },
            ..
        } => PrefixState::Failed {
            entry: index,
            cause: FailureCause::Condition {
                entry: index,
                diagnostic: diagnostic.clone(),
            },
        },
        EntrySettlement::Decision {
            result: DecisionSettlement::Failed { cause },
            ..
        } => PrefixState::Failed {
            entry: index,
            cause: cause.clone(),
        },
        EntrySettlement::Decision {
            result: DecisionSettlement::Cancelled,
            ..
        }
        | EntrySettlement::NotEvaluated {
            reason: NotEvaluatedReason::Cancelled,
        } => PrefixState::Cancelled { entry: index },
        EntrySettlement::NotEvaluated { .. } => PrefixState::Stopped { entry: index },
    }
}
pub(crate) fn prefix_state(entries: &[EntryIndex], state: &ExecutionState) -> PrefixState {
    for entry in entries {
        let result = entry_state(*entry, state);
        if !matches!(result, PrefixState::Passed) {
            return result;
        }
    }
    PrefixState::Passed
}
/// Only this scheduler-owned pass evaluates conditions; response completion merely fills gate slots.
pub(crate) async fn advance_entries(
    plan: &ExecutionPlan,
    state: &mut ExecutionState,
    origin: &InvocationOrigin,
    session: &mut EvidenceSession,
    cancellation: &CancellationToken,
) -> bool {
    let mut changed = false;
    for position in 0..plan.entries.len() {
        if !matches!(state.entries[position], EntrySettlement::Pending) {
            continue;
        }
        let index = EntryIndex(position);
        let entry = &plan.entries[position];
        if let Some(routing) = &entry.routing {
            match route_state(plan, state, routing) {
                RouteState::Pending => continue,
                RouteState::NotSelected | RouteState::Failed { .. } => {
                    state.entries[position] = EntrySettlement::NotEvaluated {
                        reason: NotEvaluatedReason::EarlierEntryStopped,
                    };
                    changed = true;
                    continue;
                }
                RouteState::Cancelled => {
                    state.entries[position] = EntrySettlement::NotEvaluated {
                        reason: NotEvaluatedReason::Cancelled,
                    };
                    changed = true;
                    continue;
                }
                RouteState::Selected => {}
            }
        }
        match prefix_state(&entry.parents, state) {
            PrefixState::Pending => continue,
            PrefixState::Stopped { .. } | PrefixState::Failed { .. } => {
                state.entries[position] = EntrySettlement::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped,
                };
                changed = true;
                continue;
            }
            PrefixState::Cancelled { .. } => {
                state.entries[position] = EntrySettlement::NotEvaluated {
                    reason: NotEvaluatedReason::Cancelled,
                };
                changed = true;
                continue;
            }
            PrefixState::Passed => {}
        }
        let settled = match &entry.definition {
            EntryDefinition::Condition(test) => {
                let outcome = if let Some(outcome) = plan.precomputed.get(&index) {
                    outcome.clone()
                } else {
                    if cancellation.is_cancelled() {
                        state.entries[position] = EntrySettlement::NotEvaluated {
                            reason: NotEvaluatedReason::Cancelled,
                        };
                        changed = true;
                        continue;
                    }
                    evaluate_condition(
                        test,
                        ConditionContext {
                            sources: &entry.sources,
                            origin,
                            session,
                        },
                    )
                    .await
                };
                Some(EntrySettlement::Condition {
                    outcome,
                    basis: condition_basis(test),
                })
            }
            EntryDefinition::Decision { threshold, .. } => match state.gates.get(&index) {
                Some(GateSettlement::Answered {
                    answer: JudgmentAnswer::Noul { yes_probability },
                    reference,
                }) => Some(EntrySettlement::Decision {
                    threshold: *threshold,
                    result: if *yes_probability >= *threshold {
                        DecisionSettlement::Passed {
                            reference: reference.clone(),
                        }
                    } else {
                        DecisionSettlement::DidNotPass {
                            reference: reference.clone(),
                        }
                    },
                }),
                Some(GateSettlement::Answered { reference, .. }) => {
                    Some(EntrySettlement::Decision {
                        threshold: *threshold,
                        result: DecisionSettlement::Failed {
                            cause: FailureCause::Request {
                                request: reference.request,
                            },
                        },
                    })
                }
                Some(GateSettlement::Failed { cause }) => Some(EntrySettlement::Decision {
                    threshold: *threshold,
                    result: DecisionSettlement::Failed {
                        cause: cause.clone(),
                    },
                }),
                Some(GateSettlement::Cancelled) => Some(EntrySettlement::Decision {
                    threshold: *threshold,
                    result: DecisionSettlement::Cancelled,
                }),
                _ => None,
            },
            EntryDefinition::Router { .. } => None,
        };
        if let Some(settled) = settled {
            state.entries[position] = settled;
            changed = true
        }
    }
    changed
}
fn known_stop(entries: &[EntryIndex], state: &ExecutionState) -> Option<EntryIndex> {
    entries.iter().copied().find(|entry| {
        matches!(
            entry_state(*entry, state),
            PrefixState::Stopped { .. }
                | PrefixState::Failed { .. }
                | PrefixState::Cancelled { .. }
        )
    })
}
pub(crate) fn unit_stopped(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    unit: UnitKey,
) -> Option<EntryIndex> {
    let (parents, routing) = match unit {
        UnitKey::Rule(index) => (
            plan.occurrences[index.0].entries.as_slice(),
            plan.groups[plan.occurrences[index.0].group.0]
                .routing
                .as_ref(),
        ),
        UnitKey::Gate(index) => (
            plan.entries[index.0].parents.as_slice(),
            plan.entries[index.0].routing.as_ref(),
        ),
    };
    if let Some(routing) = routing
        && matches!(
            route_state(plan, state, routing),
            RouteState::NotSelected | RouteState::Failed { .. } | RouteState::Cancelled
        )
    {
        return Some(routing.entry);
    }
    known_stop(parents,state).or_else(||parents.iter().copied().find(|index| {
        if plan.precomputed.get(index).is_some_and(|outcome|!matches!(outcome,ConditionOutcome::Passed)){return true}
        match state.gates.get(index){
            Some(GateSettlement::Failed{..}|GateSettlement::Cancelled)=>true,
            Some(GateSettlement::Answered{answer:JudgmentAnswer::Noul{yes_probability},..})=>matches!(plan.entries[index.0].definition,EntryDefinition::Decision{threshold,..} if *yes_probability<threshold),
            _=>false,
        }
    }))
}
pub(crate) fn request_readiness(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    request: &PlannedRequest,
) -> RequestReadiness {
    if let Some(routing) = &request.routing {
        match route_state(plan, state, routing) {
            RouteState::Pending if request.waits_for_route => return RequestReadiness::Waiting,
            RouteState::NotSelected | RouteState::Failed { .. } | RouteState::Cancelled => {
                return RequestReadiness::Stopped {
                    entry: routing.entry,
                };
            }
            _ => {}
        }
    }
    match request.purpose {
        RequestPurpose::Gate { .. } => match prefix_state(&request.prerequisites, state) {
            PrefixState::Pending => return RequestReadiness::Waiting,
            PrefixState::Stopped { entry }
            | PrefixState::Failed { entry, .. }
            | PrefixState::Cancelled { entry } => return RequestReadiness::Stopped { entry },
            PrefixState::Passed => {}
        },
        RequestPurpose::Rules { .. } => {
            if request
                .prerequisites
                .iter()
                .any(|entry| matches!(entry_state(*entry, state), PrefixState::Pending))
            {
                return RequestReadiness::Waiting;
            }
        }
    }
    if request
        .units
        .iter()
        .all(|unit| unit_stopped(plan, state, *unit).is_some())
        && let Some(entry) = request
            .units
            .iter()
            .find_map(|unit| unit_stopped(plan, state, *unit))
    {
        return RequestReadiness::Stopped { entry };
    }
    RequestReadiness::Ready
}
