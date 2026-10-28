use super::*;
use crate::*;
use std::collections::BTreeMap;
use sunstack_decisions::JudgmentAnswer;
pub(crate) fn reference(
    reference: &AnswerReference,
    indices: &BTreeMap<RequestId, usize>,
) -> JudgmentReference {
    JudgmentReference {
        request_index: indices
            .get(&reference.request)
            .copied()
            .unwrap_or(reference.request.0),
        question_name: reference.question.clone(),
    }
}
pub(crate) fn cause(
    cause: &FailureCause,
    plan: &ExecutionPlan,
    indices: &BTreeMap<RequestId, usize>,
) -> Cause {
    match cause {
        FailureCause::Request { request } => Cause::Request {
            request_index: indices.get(request).copied().unwrap_or(request.0),
        },
        FailureCause::Evidence { diagnostics } => Cause::Evidence {
            diagnostics: diagnostics.clone(),
        },
        FailureCause::Budget { diagnostics } => Cause::Budget {
            diagnostics: diagnostics.clone(),
        },
        FailureCause::Gate { entry } => Cause::Gate {
            entry: plan.entries[entry.0].key.clone(),
        },
        FailureCause::Condition { entry, diagnostic } => Cause::Condition {
            entry: plan.entries[entry.0].key.clone(),
            diagnostic: diagnostic.clone(),
        },
    }
}
pub(crate) fn evaluated_entry(
    index: EntryIndex,
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> EvaluatedEntry {
    let result = match &state.entries[index.0] {
        EntrySettlement::Condition { outcome, basis } => EntryResult::Condition {
            outcome: outcome.clone(),
            basis: basis.clone(),
        },
        EntrySettlement::Decision { threshold, result } => EntryResult::Decision {
            threshold: *threshold,
            outcome: match result {
                DecisionSettlement::Passed { reference: answer } => DecisionEntryOutcome::Passed {
                    answer: reference(answer, indices),
                },
                DecisionSettlement::DidNotPass { reference: answer } => {
                    DecisionEntryOutcome::DidNotPass {
                        answer: reference(answer, indices),
                    }
                }
                DecisionSettlement::Uncertain { reference: answer } => {
                    DecisionEntryOutcome::Uncertain {
                        answer: reference(answer, indices),
                    }
                }
                DecisionSettlement::Failed { cause: failure } => DecisionEntryOutcome::Failed {
                    cause: cause(failure, plan, indices),
                },
                DecisionSettlement::Cancelled => DecisionEntryOutcome::Cancelled,
            },
        },
        EntrySettlement::NotEvaluated { reason } => EntryResult::NotEvaluated {
            reason: reason.clone(),
        },
        EntrySettlement::Router | EntrySettlement::Pending => EntryResult::NotEvaluated {
            reason: NotEvaluatedReason::Cancelled,
        },
    };
    EvaluatedEntry {
        key: plan.entries[index.0].key.clone(),
        result,
    }
}
pub(crate) fn applicability(
    entries: &[EntryIndex],
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> ApplicabilityOutcome {
    match prefix_state(entries, state) {
        PrefixState::Passed => ApplicabilityOutcome::Applicable,
        PrefixState::Pending => ApplicabilityOutcome::NotEvaluated {
            reason: NotEvaluatedReason::Cancelled,
        },
        PrefixState::Cancelled { .. } => ApplicabilityOutcome::NotEvaluated {
            reason: NotEvaluatedReason::Cancelled,
        },
        PrefixState::Failed {
            entry,
            cause: failure,
        } => ApplicabilityOutcome::ApplicabilityFailed {
            entry: plan.entries[entry.0].key.clone(),
            cause: cause(&failure, plan, indices),
        },
        PrefixState::Uncertain { entry } => {
            let EntrySettlement::Decision {
                result: DecisionSettlement::Uncertain { reference: answer },
                ..
            } = &state.entries[entry.0]
            else {
                return ApplicabilityOutcome::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped,
                };
            };
            ApplicabilityOutcome::Uncertain {
                entry: plan.entries[entry.0].key.clone(),
                answer: reference(answer, indices),
            }
        }
        PrefixState::Stopped { entry } => {
            let basis = match &state.entries[entry.0] {
                EntrySettlement::Condition { basis, .. } => EntryBasis::Condition {
                    test: basis.clone(),
                },
                EntrySettlement::Decision {
                    threshold,
                    result: DecisionSettlement::DidNotPass { reference: answer },
                } => EntryBasis::Decision {
                    threshold: *threshold,
                    answer: reference(answer, indices),
                },
                _ => {
                    return ApplicabilityOutcome::NotEvaluated {
                        reason: NotEvaluatedReason::EarlierEntryStopped,
                    };
                }
            };
            ApplicabilityOutcome::NotApplicable {
                entry: plan.entries[entry.0].key.clone(),
                basis,
            }
        }
    }
}
pub(crate) fn router_outcome(
    index: EntryIndex,
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> RouterOutcome {
    let EntryDefinition::Router { definition, .. } = &plan.entries[index.0].definition else {
        return RouterOutcome::NotEvaluated {
            reason: RouterNotEvaluated::Cancelled,
        };
    };
    match state.routers.get(&index) {
        Some(RouterSettlement::Answered {
            answer:
                JudgmentAnswer::Choice {
                    selected_label,
                    confidence,
                    probabilities,
                },
            reference: answer,
        }) => {
            let answer = reference(answer, indices);
            if *confidence < definition.confidence_floor {
                RouterOutcome::UncertainIntent {
                    confidence: *confidence,
                    floor: definition.confidence_floor,
                    distribution: probabilities.clone(),
                    answer,
                }
            } else if definition.routes.contains_key(selected_label) {
                RouterOutcome::Routed {
                    label: selected_label.clone(),
                    confidence: *confidence,
                    answer,
                }
            } else {
                RouterOutcome::NoRoute {
                    label: selected_label.clone(),
                    confidence: *confidence,
                    answer,
                }
            }
        }
        Some(RouterSettlement::Failed { cause: failure }) => RouterOutcome::RouterFailed {
            cause: cause(failure, plan, indices),
        },
        Some(RouterSettlement::Answered {
            reference: answer, ..
        }) => RouterOutcome::RouterFailed {
            cause: Cause::Request {
                request_index: indices
                    .get(&answer.request)
                    .copied()
                    .unwrap_or(answer.request.0),
            },
        },
        _ => RouterOutcome::NotEvaluated {
            reason: RouterNotEvaluated::Cancelled,
        },
    }
}
