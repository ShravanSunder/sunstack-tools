use super::*;
use crate::*;
use std::collections::BTreeMap;

fn expanded_key(
    plan: &ExecutionPlan,
    group: GroupIndex,
    authored_order: usize,
) -> RuleEvaluationResult {
    let group = &plan.groups[group.0];
    let rule = &group.admitted.set.rules[authored_order].definition;
    RuleEvaluationResult::Expanded {
        key: RuleEvaluationKey {
            rule_set: group.admitted.set.path.clone(),
            group: group.admitted.position,
            rule_id: rule.rule_id.clone(),
        },
        occurrences: group
            .occurrences
            .iter()
            .filter(|index| plan.occurrences[index.0].authored_order == authored_order)
            .map(|index| plan.occurrences[index.0].key.clone())
            .collect(),
    }
}

fn terminal_evaluation(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    id: EvaluationIndex,
    indices: &BTreeMap<RequestId, usize>,
    final_report: bool,
) -> RuleEvaluationResult {
    let evaluation = &plan.evaluations[id.0];
    let key = evaluation.key.clone();
    // With no children, there is no occurrence projection to apply the gate's
    // final result. The real carrier still governs zero/failed discovery.
    if !matches!(
        state.expansions[id.0],
        ExpansionState::Expanded { .. } | ExpansionState::OutsideChangeScope
    ) {
        if let Some(routing) = &plan.groups[evaluation.group.0].routing {
            match route_state(plan, state, routing) {
                RouteState::Failed { cause: failure } => {
                    return RuleEvaluationResult::EvaluationFailed {
                        key,
                        cause: cause(&failure, plan, indices),
                    };
                }
                RouteState::Cancelled => return RuleEvaluationResult::Cancelled { key },
                _ => {}
            }
        }
        match applicability(&evaluation.entries, plan, state, indices) {
            ApplicabilityOutcome::NotApplicable { entry, basis } => {
                return RuleEvaluationResult::NotApplicable {
                    key,
                    stopped_at: entry,
                    basis,
                };
            }
            ApplicabilityOutcome::ApplicabilityFailed { cause, .. } => {
                return RuleEvaluationResult::EvaluationFailed { key, cause };
            }
            ApplicabilityOutcome::NotEvaluated { .. } if final_report => {
                return RuleEvaluationResult::Cancelled { key };
            }
            _ => {}
        }
    }
    match &state.expansions[id.0] {
        ExpansionState::Expanded { children } => RuleEvaluationResult::Expanded {
            key,
            occurrences: children
                .iter()
                .map(|index| plan.occurrences[index.0].key.clone())
                .collect(),
        },
        ExpansionState::ZeroMatches { source } => {
            let Some(target) = evaluation
                .rule
                .evidence
                .iter()
                .find(|declaration| declaration.id == evaluation.rule.target)
            else {
                return RuleEvaluationResult::EvaluationFailed {
                    key,
                    cause: Cause::Evidence {
                        diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget),
                    },
                };
            };
            RuleEvaluationResult::ZeroMatches {
                key,
                input: target.input.clone(),
                source: source.clone(),
            }
        }
        ExpansionState::OutsideChangeScope => RuleEvaluationResult::OutsideChangeScope {
            key,
            reason: OutsideChangeReason::NoDeclaredChange,
        },
        ExpansionState::SelectionFailed { diagnostics } => RuleEvaluationResult::SelectionFailed {
            key,
            diagnostics: diagnostics.clone(),
        },
        ExpansionState::Failed { cause: failure } => RuleEvaluationResult::EvaluationFailed {
            key,
            cause: cause(failure, plan, indices),
        },
        ExpansionState::Stopped { entry } => match applicability(&[*entry], plan, state, indices) {
            ApplicabilityOutcome::NotApplicable { entry, basis } => {
                RuleEvaluationResult::NotApplicable {
                    key,
                    stopped_at: entry,
                    basis,
                }
            }
            _ => RuleEvaluationResult::EvaluationFailed {
                key,
                cause: Cause::Gate {
                    entry: plan.entries[entry.0].key.clone(),
                },
            },
        },
        ExpansionState::Cancelled => RuleEvaluationResult::Cancelled { key },
        ExpansionState::Waiting => RuleEvaluationResult::EvaluationFailed {
            key,
            cause: Cause::Evidence {
                diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
            },
        },
    }
}

pub(crate) fn project_rule_evaluations(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> Vec<RuleEvaluationResult> {
    let mut results = Vec::new();
    for (position, group) in plan.groups.iter().enumerate() {
        if group.routing.as_ref().is_some_and(|routing| {
            matches!(route_state(plan, state, routing), RouteState::NotSelected)
        }) {
            continue;
        }
        for authored in 0..group.admitted.set.rules.len() {
            let record = group
                .evaluations
                .iter()
                .find(|index| plan.evaluations[index.0].authored_order == authored);
            results.push(match record {
                Some(index) => terminal_evaluation(plan, state, *index, indices, true),
                None => expanded_key(plan, GroupIndex(position), authored),
            });
        }
    }
    results
}

pub(crate) fn project_prepared_evaluations(
    plan: &ExecutionPlan,
    state: &ExecutionState,
) -> Vec<PreparedRuleEvaluation> {
    let mut results = Vec::new();
    for (position, group) in plan.groups.iter().enumerate() {
        for authored in 0..group.admitted.set.rules.len() {
            let record = group
                .evaluations
                .iter()
                .find(|index| plan.evaluations[index.0].authored_order == authored);
            let result = match record {
                Some(index) if matches!(state.expansions[index.0], ExpansionState::Waiting) => {
                    let on = plan.requests.iter().find(|request| matches!(request.purpose, RequestPurpose::Rules { group } if group.0 == position)).map(|request| crate::rule_preparation::waiting_on(plan, state, request)).unwrap_or_default();
                    PreparedRuleEvaluation::WaitingForExpansion {
                        key: plan.evaluations[index.0].key.clone(),
                        on,
                    }
                }
                Some(index) => {
                    terminal_evaluation(plan, state, *index, &BTreeMap::new(), false).into()
                }
                None => expanded_key(plan, GroupIndex(position), authored).into(),
            };
            results.push(result);
        }
    }
    results
}
