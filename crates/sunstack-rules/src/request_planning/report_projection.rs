use super::*;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_decisions::DecisionRunOutcome;

pub(crate) fn report_order(
    plan: &ExecutionPlan,
    request: &PlannedRequest,
) -> (usize, usize, usize) {
    match request.purpose {
        RequestPurpose::Gate { entry } => match plan.entries[entry.0].key.owner {
            EntryOwner::Router { group } => (0, group, 0),
            _ => (
                plan.entries[entry.0]
                    .group
                    .map_or(usize::MAX, |group| group.0.saturating_add(1)),
                0,
                entry.0,
            ),
        },
        RequestPurpose::Rules { group } => (
            group.0.saturating_add(1),
            1,
            match request.mode {
                RequestMode::Split { part, .. } => part,
                _ => 0,
            },
        ),
    }
}
pub(crate) fn project_report(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    origin: &InvocationOrigin,
    mut measurements: InvocationMeasurements,
) -> RuleInvocationReport {
    let ordered = ordered_requests(plan);
    let indices = ordered
        .iter()
        .enumerate()
        .map(|(position, request)| (request.id, position))
        .collect::<BTreeMap<_, _>>();
    let routing = if plan.router_entries.is_empty() {
        RoutingReport::RuleSets
    } else {
        RoutingReport::Router {
            groups: plan
                .router_entries
                .iter()
                .enumerate()
                .map(|(position, index)| RouterGroupResult {
                    position,
                    outcome: router_outcome(*index, plan, state, &indices),
                })
                .collect(),
        }
    };
    let mut groups = Vec::new();
    let mut occurrences = Vec::new();
    let mut seen_groups = BTreeSet::new();
    let mut seen_occurrences = BTreeSet::new();
    for group in &plan.groups {
        if group.routing.as_ref().is_some_and(|routing| {
            matches!(route_state(plan, state, routing), RouteState::NotSelected)
        }) {
            continue;
        }
        if seen_groups.insert((group.admitted.set.path.clone(), group.admitted.position)) {
            let mut entries = group.entries.clone();
            for occurrence in &group.occurrences {
                entries.extend(
                    plan.occurrences[occurrence.0]
                        .entries
                        .iter()
                        .copied()
                        .filter(|entry| {
                            matches!(plan.entries[entry.0].key.owner, EntryOwner::Rule { .. })
                        }),
                );
            }
            for evaluation in &group.evaluations {
                entries.extend(
                    plan.evaluations[evaluation.0]
                        .entries
                        .iter()
                        .copied()
                        .filter(|entry| {
                            matches!(plan.entries[entry.0].key.owner, EntryOwner::Rule { .. })
                        }),
                );
            }
            entries.sort();
            entries.dedup();
            let applicable = if let Some(routing) = &group.routing {
                match route_state(plan, state, routing) {
                    RouteState::Failed { cause: failure } => {
                        ApplicabilityOutcome::ApplicabilityFailed {
                            entry: plan.entries[routing.entry.0].key.clone(),
                            cause: cause(&failure, plan, &indices),
                        }
                    }
                    _ => applicability(&group.entries, plan, state, &indices),
                }
            } else {
                applicability(&group.entries, plan, state, &indices)
            };
            groups.push(GroupResult {
                rule_set: group.admitted.set.path.clone(),
                position: group.admitted.position,
                binding: group.admitted.binding.clone(),
                applicability: applicable,
                entries: entries
                    .into_iter()
                    .map(|entry| evaluated_entry(entry, plan, state, &indices))
                    .collect(),
            });
        }
        for index in &group.occurrences {
            if seen_occurrences.insert(plan.occurrences[index.0].key.clone()) {
                occurrences.push(occurrence_result(*index, plan, state, &indices));
            }
        }
    }
    let mut requests = Vec::new();
    let mut diagnostics = DiagnosticList::default();
    for request in ordered {
        let (status, bindings, unit_bindings) = match state.requests.get(&request.id) {
            Some(RequestSettlement::Settled {
                status,
                bindings,
                unit_bindings,
            }) => (status.clone(), bindings.clone(), unit_bindings.clone()),
            _ => (
                RequestStatus::NotSent {
                    reason: NotSentReason::Cancelled,
                },
                vec![],
                vec![],
            ),
        };
        let mut discarded = Vec::new();
        for (unit, question_name) in unit_bindings {
            if !matches!(&status,RequestStatus::Sent{outcome:DecisionRunOutcome::Completed{answers,..}} if answers.contains_key(&question_name))
            {
                continue;
            }
            let routing = match unit {
                UnitKey::Rule(index) => plan.groups[plan.occurrences[index.0].group.0]
                    .routing
                    .as_ref(),
                UnitKey::Gate(index) => plan.entries[index.0].routing.as_ref(),
            };
            if routing.is_some_and(|routing| {
                matches!(route_state(plan, state, routing), RouteState::NotSelected)
            }) {
                discarded.push(DiscardedAnswer {
                    question_name,
                    reason: DiscardReason::NotRouted,
                });
                continue;
            }
            let parents = match unit {
                UnitKey::Rule(index) => plan.occurrences[index.0].entries.as_slice(),
                UnitKey::Gate(index) => plan.entries[index.0].parents.as_slice(),
            };
            if let PrefixState::Stopped { entry } = prefix_state(parents, state) {
                discarded.push(DiscardedAnswer {
                    question_name,
                    reason: DiscardReason::OwnerNotApplicable {
                        entry: plan.entries[entry.0].key.clone(),
                    },
                });
            }
        }
        if let RequestStatus::Sent {
            outcome:
                DecisionRunOutcome::InvalidRequest {
                    diagnostics: reported,
                }
                | DecisionRunOutcome::InvalidResponse {
                    diagnostics: reported,
                    ..
                }
                | DecisionRunOutcome::ProviderFailure {
                    diagnostics: reported,
                    ..
                }
                | DecisionRunOutcome::Cancelled {
                    diagnostics: reported,
                    ..
                },
        } = &status
        {
            diagnostics.extend(reported.clone().into());
        }
        requests.push(RequestRecord {
            index: indices[&request.id],
            owner: request.owner.clone(),
            mode: request.mode.clone(),
            questions: bindings,
            prerequisites: request
                .prerequisites
                .iter()
                .map(|entry| plan.entries[entry.0].key.clone())
                .collect(),
            status,
            discarded,
        });
    }
    for prepared in state.preparation.values() {
        if let OwnerPreparation::Refused {
            diagnostics: reported,
            ..
        } = prepared
        {
            diagnostics.extend(reported.clone());
        }
    }
    for reported in state.budget_refusals.values() {
        diagnostics.extend(reported.clone());
    }
    for entry in &state.entries {
        if let EntrySettlement::Condition {
            outcome: ConditionOutcome::Unresolved { diagnostic },
            ..
        } = entry
        {
            diagnostics.push(diagnostic.clone());
        }
    }
    measurements.applicability_entries_evaluated = state
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                EntrySettlement::Condition { .. } | EntrySettlement::Decision { .. }
            )
        })
        .count();
    for occurrence in &occurrences {
        match occurrence {
            RuleOccurrenceResult::Judged { .. } => measurements.rules_selected += 1,
            RuleOccurrenceResult::Checked { outcome, .. } => match outcome {
                LocalSettlement::Completed { .. } => measurements.rules_selected += 1,
                LocalSettlement::Unresolved {
                    diagnostics: reported,
                } => {
                    measurements.rules_failed += 1;
                    diagnostics.extend(reported.clone());
                }
                LocalSettlement::Cancelled => measurements.rules_failed += 1,
            },
            RuleOccurrenceResult::NotApplicable { .. }
            | RuleOccurrenceResult::OutsideChangeScope { .. } => measurements.rules_skipped += 1,
            _ => measurements.rules_failed += 1,
        }
    }
    let rule_evaluations = project_rule_evaluations(plan, state, &indices);
    for evaluation in &rule_evaluations {
        match evaluation {
            RuleEvaluationResult::NotApplicable { .. }
            | RuleEvaluationResult::OutsideChangeScope { .. } => measurements.rules_skipped += 1,
            RuleEvaluationResult::SelectionFailed {
                diagnostics: reported,
                ..
            } => {
                measurements.rules_failed += 1;
                diagnostics.extend(reported.clone());
            }
            RuleEvaluationResult::EvaluationFailed { .. }
            | RuleEvaluationResult::Cancelled { .. } => measurements.rules_failed += 1,
            _ => {}
        }
    }
    let source_summary = crate::rule_source_reporting::source_summary(
        plan.groups.iter().map(|group| group.admitted.set.as_ref()),
        &mut measurements,
    );
    RuleInvocationReport {
        rule_evaluations,
        rule_coverage: source_summary.coverage,
        skipped_rules: source_summary.skipped,
        origin: origin.into(),
        routing,
        groups,
        occurrences,
        requests,
        captures: vec![],
        change_selections: vec![],
        cursors: vec![],
        diagnostics,
        measurements,
    }
}

fn ordered_requests(plan: &ExecutionPlan) -> Vec<&PlannedRequest> {
    let mut ordered = plan.requests.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|request| report_order(plan, request));
    ordered
}

pub(crate) fn project_request_evidence(
    plan: &ExecutionPlan,
    state: &ExecutionState,
) -> BTreeMap<usize, sunstack_decisions::SharedEvidenceState> {
    ordered_requests(plan)
        .into_iter()
        .enumerate()
        .filter_map(|(index, request)| {
            state
                .request_evidence
                .get(&request.id)
                .map(|evidence| (index, evidence.clone()))
        })
        .collect()
}
