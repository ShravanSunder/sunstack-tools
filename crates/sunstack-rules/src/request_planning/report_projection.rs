use super::*;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_decisions::{DecisionRunOutcome, JudgmentAnswer};

fn report_order(plan: &ExecutionPlan, request: &PlannedRequest) -> (usize, usize, usize) {
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
fn reference(
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
fn cause(
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
fn evaluated_entry(
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
fn applicability(
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
fn router_outcome(
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
fn request_for_unit<'state>(
    unit: UnitKey,
    state: &'state ExecutionState,
) -> Option<(
    RequestId,
    &'state RequestStatus,
    &'state [QuestionBinding],
    &'state [(UnitKey, sunstack_decisions::QuestionName)],
)> {
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
                Some((*id, status, bindings.as_slice(), unit_bindings.as_slice()))
            }
            _ => None,
        })
}
fn occurrence_result(
    index: OccurrenceIndex,
    plan: &ExecutionPlan,
    state: &ExecutionState,
    indices: &BTreeMap<RequestId, usize>,
) -> RuleOccurrenceResult {
    let occurrence = &plan.occurrences[index.0];
    let key = occurrence.key.clone();
    if let Some(routing) = &plan.groups[occurrence.group.0].routing {
        match route_state(plan, state, routing) {
            RouteState::Failed { cause: failure } => {
                return RuleOccurrenceResult::EvaluationFailed {
                    key,
                    cause: cause(&failure, plan, indices),
                };
            }
            RouteState::Cancelled | RouteState::Pending => {
                return RuleOccurrenceResult::Cancelled { key };
            }
            _ => {}
        }
    }
    let applicability = applicability(&occurrence.entries, plan, state, indices);
    if let Some(result) =
        crate::applicability_evaluation::stopped_occurrence(key.clone(), &applicability)
    {
        return result;
    }
    let unit = UnitKey::Rule(index);
    if matches!(
        state.preparation.get(&unit),
        Some(OwnerPreparation::Cancelled { .. })
    ) {
        return RuleOccurrenceResult::Cancelled { key };
    }
    if let Some(OwnerPreparation::Refused { diagnostics, .. }) = state.preparation.get(&unit) {
        return RuleOccurrenceResult::SelectionFailed {
            key,
            diagnostics: diagnostics.clone(),
        };
    }
    if let Some(diagnostics) = state.budget_refusals.get(&unit) {
        return RuleOccurrenceResult::EvaluationFailed {
            key,
            cause: Cause::Budget {
                diagnostics: diagnostics.clone(),
            },
        };
    }
    let Some((id, status, bindings, unit_bindings)) = request_for_unit(unit, state) else {
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
                key,
                reason: NotStartedReason::RequestLimit,
            }
        } else if state.preparation.contains_key(&unit) {
            RuleOccurrenceResult::Cancelled { key }
        } else {
            RuleOccurrenceResult::NotStarted {
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
            RuleOccurrenceResult::Cancelled { key }
        } else {
            RuleOccurrenceResult::EvaluationFailed {
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
        Ok(verdict) => RuleOccurrenceResult::Judged {
            key: key.clone(),
            judgment: RuleJudgment {
                key,
                verdict,
                declared_severity: occurrence.rule.severity,
                answers: informational,
                target: target.clone(),
            },
        },
        Err(_) => RuleOccurrenceResult::EvaluationFailed {
            key,
            cause: Cause::Request {
                request_index: indices.get(&id).copied().unwrap_or(id.0),
            },
        },
    }
}
pub(crate) fn project_report(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    origin: &InvocationOrigin,
    mut measurements: InvocationMeasurements,
) -> RuleInvocationReport {
    let mut ordered = plan.requests.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|request| report_order(plan, request));
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
        if let RequestStatus::Sent { outcome } = &status {
            match outcome {
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
                } => diagnostics.extend(reported.clone().into()),
                _ => {}
            }
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
            RuleOccurrenceResult::NotApplicable { .. }
            | RuleOccurrenceResult::OutsideChangeScope { .. } => measurements.rules_skipped += 1,
            _ => measurements.rules_failed += 1,
        }
    }
    RuleInvocationReport {
        origin: origin.into(),
        routing,
        groups,
        occurrences,
        requests,
        captures: vec![],
        cursors: vec![],
        diagnostics,
        measurements,
    }
}
