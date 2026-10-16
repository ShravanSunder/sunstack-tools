use super::*;
use crate::*;
use std::collections::BTreeMap;
use sunstack_decisions::DecisionRunOutcome;
struct SettledUnitRequest<'state> {
    id: RequestId,
    status: &'state RequestStatus,
    bindings: &'state [QuestionBinding],
    unit_bindings: &'state [(UnitKey, sunstack_decisions::QuestionName)],
}
fn request_for_unit(unit: UnitKey, state: &ExecutionState) -> Option<SettledUnitRequest<'_>> {
    state
        .requests
        .iter()
        .find_map(|(id, request)| match request {
            RequestSettlement::Settled {
                status,
                bindings,
                unit_bindings,
            } if unit_bindings
                .iter()
                .any(|(candidate, _)| *candidate == unit) =>
            {
                Some(SettledUnitRequest {
                    id: *id,
                    status,
                    bindings,
                    unit_bindings,
                })
            }
            _ => None,
        })
}
pub(crate) fn occurrence_result(
    index: OccurrenceIndex,
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> RuleOccurrenceResult {
    let occurrence = &plan.occurrences[index.0];
    let key = occurrence.key.clone();
    if matches!(
        occurrence.scope.disposition,
        ScopeDisposition::OutsideChangeScope
    ) {
        return RuleOccurrenceResult::OutsideChangeScope {
            key,
            reason: OutsideChangeReason::NoDeclaredChange,
        };
    }
    let selection = occurrence_selection(plan, state, index);
    if let Some(routing) = &plan.groups[occurrence.group.0].routing {
        match route_state(plan, state, routing) {
            RouteState::Failed { cause: failure } => {
                return RuleOccurrenceResult::EvaluationFailed {
                    selection: selection.clone(),
                    key,
                    cause: cause(&failure, plan, indices),
                };
            }
            RouteState::Cancelled | RouteState::Pending => {
                return RuleOccurrenceResult::Cancelled {
                    key,
                    selection: selection.clone(),
                };
            }
            _ => {}
        }
    }
    let applicability = applicability(&occurrence.entries, plan, state, indices);
    if let Some(result) = crate::applicability_evaluation::stopped_occurrence(
        key.clone(),
        &applicability,
        selection.clone(),
    ) {
        return result;
    }
    if matches!(
        effective_scope(plan, state, index).disposition,
        ScopeDisposition::OutsideChangeScope
    ) {
        return RuleOccurrenceResult::OutsideChangeScope {
            key,
            reason: OutsideChangeReason::NoDeclaredChange,
        };
    }
    let unit = UnitKey::Rule(index);
    if matches!(
        state.preparation.get(&unit),
        Some(OwnerPreparation::Cancelled { .. })
    ) {
        return RuleOccurrenceResult::Cancelled {
            key,
            selection: selection.clone(),
        };
    }
    if let Some(OwnerPreparation::Refused { diagnostics, .. }) = state.preparation.get(&unit) {
        return RuleOccurrenceResult::SelectionFailed {
            key,
            diagnostics: diagnostics.clone(),
        };
    }
    if let Some(diagnostics) = state.budget_refusals.get(&unit) {
        return RuleOccurrenceResult::EvaluationFailed {
            selection: selection.clone(),
            key,
            cause: Cause::Budget {
                diagnostics: diagnostics.clone(),
            },
        };
    }
    let Some(SettledUnitRequest {
        id,
        status,
        bindings,
        unit_bindings,
    }) = request_for_unit(unit, state)
    else {
        let limited = plan
            .requests
            .iter()
            .filter(|request| request.units.contains(&unit))
            .any(|request| {
                matches!(
                    state.requests.get(&request.id),
                    Some(RequestSettlement::Settled {
                        status: RequestStatus::NotSent {
                            reason: NotSentReason::RequestLimit
                        },
                        ..
                    })
                )
            });
        return if limited {
            RuleOccurrenceResult::NotStarted {
                selection: selection.clone(),
                key,
                reason: NotStartedReason::RequestLimit,
            }
        } else if state.preparation.contains_key(&unit) {
            RuleOccurrenceResult::Cancelled {
                key,
                selection: selection.clone(),
            }
        } else {
            RuleOccurrenceResult::NotStarted {
                selection: selection.clone(),
                key,
                reason: NotStartedReason::CallerCancelled,
            }
        };
    };
    let RequestStatus::Sent {
        outcome: DecisionRunOutcome::Completed { answers, .. },
    } = status
    else {
        return if matches!(
            status,
            RequestStatus::Sent {
                outcome: DecisionRunOutcome::Cancelled { .. }
            } | RequestStatus::NotSent {
                reason: NotSentReason::Cancelled
            }
        ) {
            RuleOccurrenceResult::Cancelled {
                key,
                selection: selection.clone(),
            }
        } else {
            RuleOccurrenceResult::EvaluationFailed {
                selection: selection.clone(),
                key,
                cause: Cause::Request {
                    request_index: indices.get(&id).copied().unwrap_or(id.0),
                },
            }
        };
    };
    let mut native = VerdictAnswers {
        noul_verdict: None,
        informational: BTreeMap::new(),
    };
    let mut informational = BTreeMap::new();
    for (candidate, name) in unit_bindings {
        if *candidate != unit {
            continue;
        }
        let Some(answer) = answers.get(name) else {
            continue;
        };
        let answer_reference = JudgmentReference {
            request_index: indices.get(&id).copied().unwrap_or(id.0),
            question_name: name.clone(),
        };
        let Some(binding) = bindings
            .iter()
            .find(|binding| binding.request_name == *name)
        else {
            continue;
        };
        match &binding.owner {
            QuestionOwner::Verdict { .. } => {
                native.noul_verdict = Some(ReferencedAnswer {
                    answer: answer.clone(),
                    reference: answer_reference,
                })
            }
            QuestionOwner::Informational { name, .. } => {
                native.informational.insert(
                    name.clone(),
                    ReferencedAnswer {
                        answer: answer.clone(),
                        reference: answer_reference.clone(),
                    },
                );
                informational.insert(name.clone(), answer_reference);
            }
            _ => {}
        }
    }
    let Some(OwnerPreparation::Ready {
        target: OwnerTarget::Rule(target),
        ..
    }) = state.preparation.get(&unit)
    else {
        return RuleOccurrenceResult::SelectionFailed {
            key,
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget),
        };
    };
    match evaluate_verdict(&occurrence.rule, &native) {
        Ok(verdict) => {
            let Some(selected_by) = selection.selected_reasons() else {
                return RuleOccurrenceResult::SelectionFailed {
                    key,
                    diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                };
            };
            RuleOccurrenceResult::Judged {
                selected_by: selected_by.clone(),
                key: key.clone(),
                judgment: RuleJudgment {
                    key,
                    verdict,
                    declared_severity: occurrence.rule.severity,
                    answers: informational,
                    target: target.clone(),
                },
            }
        }
        Err(_) => RuleOccurrenceResult::EvaluationFailed {
            selection: selection.clone(),
            key,
            cause: Cause::Request {
                request_index: indices.get(&id).copied().unwrap_or(id.0),
            },
        },
    }
}
