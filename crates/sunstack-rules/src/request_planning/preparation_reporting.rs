use super::*;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{CaptureCoverage, ReturnedCursor};

enum LeadingConditionStop {
    DidNotPass {
        index: EntryIndex,
        basis: EntryBasis,
    },
    Unresolved {
        index: EntryIndex,
        diagnostic: ReportDiagnostic,
    },
}
fn leading_stop(plan: &ExecutionPlan, entries: &[EntryIndex]) -> Option<LeadingConditionStop> {
    entries
        .iter()
        .find_map(|index| match plan.precomputed.get(index)? {
            ConditionOutcome::Passed => None,
            ConditionOutcome::DidNotPass => {
                let EntryDefinition::Condition(test) = &plan.entries[index.0].definition else {
                    return None;
                };
                Some(LeadingConditionStop::DidNotPass {
                    index: *index,
                    basis: EntryBasis::Condition {
                        test: crate::rule_evaluation::applicability_evaluation::condition_basis(
                            test,
                        ),
                    },
                })
            }
            ConditionOutcome::Unresolved { diagnostic } => Some(LeadingConditionStop::Unresolved {
                index: *index,
                diagnostic: diagnostic.clone(),
            }),
        })
}
pub(crate) struct PreparationProjection {
    pub records: BTreeMap<RequestId, (Vec<QuestionBinding>, RequestPreparationReadiness)>,
    pub measurements: InvocationMeasurements,
    pub captures: Vec<CaptureCoverage>,
    pub change_selections: Vec<sunstack_evidence::ResolvedChangeSelection>,
    pub cursors: Vec<ReturnedCursor>,
}
pub(crate) fn project_preparation(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    origin: &InvocationOrigin,
    projection: PreparationProjection,
    diagnostics: &mut DiagnosticList,
) -> PreparedRules {
    let PreparationProjection {
        mut records,
        mut measurements,
        captures,
        change_selections,
        cursors,
    } = projection;
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for group in &plan.groups {
        if seen.insert(group.admitted.set.path.clone()) {
            candidates.push(group.admitted.set.path.clone());
        }
    }
    let routing = if plan.router_entries.is_empty() {
        PreparedRouting::RuleSets
    } else {
        PreparedRouting::Router {
            status: RouterPreparationStatus::NotEvaluated,
            candidate_rule_sets: candidates,
            groups: (0..plan.router_entries.len()).collect(),
        }
    };
    let groups = plan
        .groups
        .iter()
        .map(|group| {
            let indices = group
                .entries
                .iter()
                .copied()
                .chain(group.occurrences.iter().flat_map(|index| {
                    plan.occurrences[index.0]
                        .entries
                        .iter()
                        .copied()
                        .filter(|entry| {
                            matches!(plan.entries[entry.0].key.owner, EntryOwner::Rule { .. })
                        })
                }))
                .chain(
                    group
                        .evaluations
                        .iter()
                        .flat_map(|index| plan.evaluations[index.0].entries.iter().copied()),
                )
                .collect::<BTreeSet<_>>();
            let leading_entries = indices
                .into_iter()
                .filter_map(|index| {
                    let outcome = plan.precomputed.get(&index)?;
                    let EntryDefinition::Condition(test) = &plan.entries[index.0].definition else {
                        return None;
                    };
                    Some(EvaluatedEntry {
                        key: plan.entries[index.0].key.clone(),
                        result: EntryResult::Condition {
                            outcome: outcome.clone(),
                            basis:
                                crate::rule_evaluation::applicability_evaluation::condition_basis(
                                    test,
                                ),
                        },
                    })
                })
                .collect();
            let status = match leading_stop(plan, &group.entries) {
                Some(LeadingConditionStop::DidNotPass { index, basis }) => {
                    PreparedGroupStatus::NotApplicable {
                        entry: plan.entries[index.0].key.clone(),
                        basis,
                    }
                }
                Some(LeadingConditionStop::Unresolved { index, diagnostic }) => {
                    PreparedGroupStatus::ApplicabilityFailed {
                        entry: plan.entries[index.0].key.clone(),
                        cause: Cause::Condition {
                            entry: plan.entries[index.0].key.clone(),
                            diagnostic,
                        },
                    }
                }
                _ => PreparedGroupStatus::Planned,
            };
            PreparedGroup {
                rule_set: group.admitted.set.path.clone(),
                position: group.admitted.position,
                binding: group.admitted.binding.clone(),
                leading_entries,
                status,
            }
        })
        .collect();
    let occurrences = plan
        .groups
        .iter()
        .flat_map(|group| group.occurrences.iter().copied())
        .map(|index| {
            let position = index.0;
            let occurrence = &plan.occurrences[position];
            let status = if matches!(
                effective_scope(plan, state, OccurrenceIndex(position)).disposition,
                ScopeDisposition::OutsideChangeScope
            ) {
                measurements.rules_skipped += 1;
                PreparedOccurrenceStatus::OutsideChangeScope {
                    reason: OutsideChangeReason::NoDeclaredChange,
                }
            } else {
                match leading_stop(plan, &occurrence.entries) {
                    Some(LeadingConditionStop::DidNotPass { index, basis }) => {
                        measurements.rules_skipped += 1;
                        PreparedOccurrenceStatus::NotApplicable {
                            entry: plan.entries[index.0].key.clone(),
                            basis,
                        }
                    }
                    _ => match state
                        .preparation
                        .get(&UnitKey::Rule(OccurrenceIndex(position)))
                    {
                        Some(OwnerPreparation::Refused { diagnostics, .. }) => {
                            measurements.rules_failed += 1;
                            PreparedOccurrenceStatus::SelectionFailed {
                                diagnostics: diagnostics.clone(),
                            }
                        }
                        Some(OwnerPreparation::Ready { .. }) => {
                            measurements.rules_selected += 1;
                            PreparedOccurrenceStatus::Planned {
                                selection: occurrence_selection(
                                    plan,
                                    state,
                                    OccurrenceIndex(position),
                                ),
                            }
                        }
                        _ => PreparedOccurrenceStatus::Planned {
                            selection: occurrence_selection(plan, state, OccurrenceIndex(position)),
                        },
                    },
                }
            };
            PreparedOccurrence {
                key: occurrence.key.clone(),
                status,
            }
        })
        .collect();
    let mut ordered = plan
        .requests
        .iter()
        .filter(|request| records.contains_key(&request.id))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|request| report_order(plan, request));
    let planned_requests = ordered
        .into_iter()
        .enumerate()
        .filter_map(|(index, request)| {
            let (questions, readiness) = records.remove(&request.id)?;
            Some(PlannedRequestRecord {
                index,
                owner: request.owner.clone(),
                mode: request.mode.clone(),
                questions,
                prerequisites: request
                    .prerequisites
                    .iter()
                    .map(|entry| plan.entries[entry.0].key.clone())
                    .collect(),
                readiness,
            })
        })
        .collect();
    for outcome in plan.precomputed.values() {
        if let ConditionOutcome::Unresolved { diagnostic } = outcome {
            diagnostics.push(diagnostic.clone());
        }
    }
    measurements.applicability_entries_evaluated = plan.precomputed.len();
    let rule_evaluations = project_prepared_evaluations(plan, state);
    for evaluation in &rule_evaluations {
        match evaluation {
            PreparedRuleEvaluation::NotApplicable { .. }
            | PreparedRuleEvaluation::OutsideChangeScope { .. } => measurements.rules_skipped += 1,
            PreparedRuleEvaluation::SelectionFailed {
                diagnostics: reported,
                ..
            } => {
                measurements.rules_failed += 1;
                diagnostics.extend(reported.clone());
            }
            PreparedRuleEvaluation::EvaluationFailed { cause, .. } => {
                measurements.rules_failed += 1;
                match cause {
                    Cause::Evidence {
                        diagnostics: reported,
                    }
                    | Cause::Budget {
                        diagnostics: reported,
                    } => diagnostics.extend(reported.clone()),
                    Cause::Condition { diagnostic, .. } => diagnostics.push(diagnostic.clone()),
                    _ => diagnostics.extend(DiagnosticList::rule(
                        RuleDiagnosticCode::UnexpectedExecutionError,
                    )),
                }
            }
            PreparedRuleEvaluation::Cancelled { .. } => measurements.rules_failed += 1,
            _ => {}
        }
    }
    let source_summary = crate::rule_evaluation::rule_coverage_reporting::source_summary(
        plan.groups.iter().map(|group| group.admitted.set.as_ref()),
        &mut measurements,
    );
    PreparedRules {
        rule_evaluations,
        rule_coverage: source_summary.coverage,
        skipped_rules: source_summary.skipped,
        origin: origin.into(),
        routing,
        groups,
        occurrences,
        planned_requests,
        captures,
        change_selections,
        cursors,
        measurements,
    }
}
