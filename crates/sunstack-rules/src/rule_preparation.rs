use crate::{invocation_admission::admit_invocation, request_planning::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use sunstack_evidence::EvidenceSession;

fn cancelled(measurements: InvocationMeasurements) -> RulePreparationOutcome {
    RulePreparationOutcome::Cancelled {
        progress: PreparationCancellationProgress::BeforePreparationAdmission,
        diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
        measurements,
    }
}
/// Prepare only the requests the scheduler can send before any decision answers.
pub async fn prepare_rules(
    input: RuleInvocationInput,
    context: RuleExecutionContext,
) -> RulePreparationOutcome {
    let started = Instant::now();
    let cancellation = context.cancellation.child_token();
    let _guard = cancellation.clone().drop_guard();
    let mut measurements = InvocationMeasurements::default();
    if cancellation.is_cancelled() {
        return cancelled(measurements);
    }
    if input.backend != *context.backend.selection() {
        return RulePreparationOutcome::Invalid {
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration),
        };
    }
    let admitted = match admit_invocation(&input, &context.adapters, &cancellation).await {
        Ok(admitted) => admitted,
        Err(diagnostics) => {
            return if cancellation.is_cancelled() {
                cancelled(measurements)
            } else {
                RulePreparationOutcome::Invalid { diagnostics }
            };
        }
    };
    let mut session = match EvidenceSession::open(
        admitted.evidence,
        context.adapters,
        cancellation.clone(),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => {
            return if cancellation.is_cancelled() {
                cancelled(measurements)
            } else {
                RulePreparationOutcome::Invalid {
                    diagnostics: error.diagnostics().into(),
                }
            };
        }
    };
    let mut plan = definition_topology(admitted.groups, admitted.router);
    precompute_conditions(
        &mut plan,
        &input.origin,
        &mut session,
        &cancellation,
        input.execution.speculation,
    )
    .await;
    build_request_graph(&mut plan, &input.execution);
    if cancellation.is_cancelled() {
        crate::rule_execution::copy_evidence_measurements(
            &mut measurements,
            session.close().measurements,
        );
        return cancelled(measurements);
    }
    let mut state = ExecutionState::new(&plan);
    advance_entries(
        &plan,
        &mut state,
        &input.origin,
        &mut session,
        &cancellation,
    )
    .await;
    let mut records = BTreeMap::new();
    let mut diagnostics = DiagnosticList::default();
    let mut omitted = BTreeSet::new();
    let mut encoded_parts = BTreeMap::new();
    let mut position = 0;
    while position < plan.requests.len() {
        let request = plan.requests[position].clone();
        position += 1;
        if cancellation.is_cancelled() {
            break;
        }
        match request_readiness(&plan, &state, &request) {
            RequestReadiness::Stopped { .. } => continue,
            RequestReadiness::Waiting => {
                let questions = match describe_questions(&plan, &request.units) {
                    Ok(descriptions) => descriptions
                        .into_iter()
                        .map(|description| description.binding)
                        .collect(),
                    Err(diagnostics) => {
                        return RulePreparationOutcome::PreparationFailed {
                            diagnostics,
                            measurements,
                        };
                    }
                };
                records.insert(
                    request.id,
                    (
                        questions,
                        RequestPreparationReadiness::Waiting {
                            on: waiting_on(&plan, &state, &request),
                        },
                    ),
                );
                continue;
            }
            RequestReadiness::Ready => {}
        }
        if let Some(RequestSettlement::Settled {
            status:
                RequestStatus::NotSent {
                    reason:
                        NotSentReason::BudgetRefused {
                            diagnostics: reported,
                        },
                },
            ..
        }) = state.requests.get(&request.id)
        {
            let reported = reported.clone();
            diagnostics.extend(reported.clone());
            let questions = match describe_questions(&plan, &request.units) {
                Ok(descriptions) => descriptions
                    .into_iter()
                    .map(|description| description.binding)
                    .collect(),
                Err(diagnostics) => {
                    return RulePreparationOutcome::PreparationFailed {
                        diagnostics,
                        measurements,
                    };
                }
            };
            records.insert(
                request.id,
                (
                    questions,
                    RequestPreparationReadiness::Prepared {
                        budget: PreparedRequestBudget::Exceeded {
                            diagnostics: reported,
                        },
                        evidence: owner_evidence(&state, &request.units),
                    },
                ),
            );
            continue;
        }
        let units = prepare_units(&plan, &mut state, &request, &mut session, &cancellation).await;
        if cancellation.is_cancelled() {
            break;
        }
        let evidence = owner_evidence(&state, &request.units);
        if units.is_empty() {
            let refused = preparation_diagnostics(&state, &request.units);
            diagnostics.extend(refused.clone());
            records.insert(
                request.id,
                (
                    vec![],
                    RequestPreparationReadiness::Prepared {
                        budget: PreparedRequestBudget::Unverifiable {
                            diagnostics: refused,
                        },
                        evidence,
                    },
                ),
            );
            continue;
        }
        let mut omit_optional = omitted.contains(&request.id);
        let mut admission = encoded_parts.remove(&request.id).unwrap_or_else(|| {
            encode_units(
                &plan,
                &state,
                &units,
                omit_optional,
                context.backend.as_ref(),
                &input.request_budget,
            )
        });
        if matches!(admission, RequestAdmission::Exceeded(_))
            && !omit_optional
            && has_optional(&state, &units)
        {
            omit_optional = true;
            admission = encode_units(
                &plan,
                &state,
                &units,
                true,
                context.backend.as_ref(),
                &input.request_budget,
            );
        }
        if matches!(admission, RequestAdmission::Exceeded(_))
            && matches!(request.purpose, RequestPurpose::Rules { .. })
            && units.iter().any(|unit| matches!(unit, UnitKey::Rule(_)))
        {
            let parts = split_rules_request(
                &plan,
                &state,
                &units,
                omit_optional,
                context.backend.as_ref(),
                &input.request_budget,
            );
            let admitted = install_budget_parts(&mut plan, &mut state, &request, parts);
            for (id, _, encoded) in admitted {
                if omit_optional {
                    omitted.insert(id);
                }
                encoded_parts.insert(id, RequestAdmission::Within(encoded));
            }
            // Revisit the replacement at the same index; appended split nodes follow normally.
            position -= 1;
            continue;
        }
        let bindings = match &admission {
            RequestAdmission::Within(encoded) => encoded.bindings.clone(),
            _ => match render_units(&plan, &state, &units, omit_optional) {
                Ok(rendered) => rendered.bindings,
                Err(_) => vec![],
            },
        };
        let budget = match admission {
            RequestAdmission::Within(encoded) => {
                let counts = encoded.admitted.counts().clone();
                PreparedRequestBudget::WithinConfiguredBudget {
                    basis: counts.basis.clone(),
                    counts,
                }
            }
            RequestAdmission::Exceeded(reported) => {
                diagnostics.extend(reported.clone());
                PreparedRequestBudget::Exceeded {
                    diagnostics: reported,
                }
            }
            RequestAdmission::Unverifiable(reported) | RequestAdmission::Invalid(reported) => {
                diagnostics.extend(reported.clone());
                PreparedRequestBudget::Unverifiable {
                    diagnostics: reported,
                }
            }
        };
        diagnostics.extend(preparation_diagnostics(&state, &request.units));
        records.insert(
            request.id,
            (
                bindings,
                RequestPreparationReadiness::Prepared { budget, evidence },
            ),
        );
    }
    measurements.local_elapsed_ms =
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let closed = session.close();
    crate::rule_execution::copy_evidence_measurements(&mut measurements, closed.measurements);
    let preparation = project_preparation(
        &plan,
        &state,
        &input.origin,
        PreparationProjection {
            records,
            measurements,
            captures: closed.captures,
            cursors: closed.cursors,
        },
        &mut diagnostics,
    );
    if cancellation.is_cancelled() {
        let measurements = preparation.measurements.clone();
        RulePreparationOutcome::Cancelled {
            progress: PreparationCancellationProgress::PreparationAdmitted {
                partial: Box::new(preparation),
            },
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
            measurements,
        }
    } else if diagnostics.is_empty() {
        RulePreparationOutcome::Prepared { preparation }
    } else {
        RulePreparationOutcome::PreparedWithFailures {
            preparation,
            diagnostics,
        }
    }
}
fn owner_evidence(
    state: &ExecutionState,
    units: &[UnitKey],
) -> Vec<sunstack_evidence::SelectionResult> {
    units
        .iter()
        .flat_map(|unit| match state.preparation.get(unit) {
            Some(
                OwnerPreparation::Ready { selections, .. }
                | OwnerPreparation::Refused { selections, .. }
                | OwnerPreparation::Cancelled { selections },
            ) => selections
                .iter()
                .map(|selection| selection.result.clone())
                .collect(),
            None => vec![],
        })
        .collect()
}
fn gate_frontier(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    index: EntryIndex,
) -> Option<EntryIndex> {
    let entry = &plan.entries[index.0];
    if let Some(routing) = &entry.routing
        && matches!(route_state(plan, state, routing), RouteState::Pending)
    {
        return Some(routing.entry);
    }
    for prior in &entry.parents {
        if matches!(entry_state(*prior, state), PrefixState::Pending)
            && let Some(frontier) = gate_frontier(plan, state, *prior)
        {
            return Some(frontier);
        }
    }
    entry.definition.is_gate().then_some(index)
}
fn waiting_on(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    request: &PlannedRequest,
) -> Vec<EntryKey> {
    let mut waiting = BTreeSet::new();
    if request.waits_for_route
        && let Some(routing) = &request.routing
    {
        waiting.insert(routing.entry);
    }
    for entry in &request.prerequisites {
        if matches!(entry_state(*entry, state), PrefixState::Pending)
            && let Some(frontier) = gate_frontier(plan, state, *entry)
        {
            waiting.insert(frontier);
        }
    }
    waiting
        .into_iter()
        .map(|index| plan.entries[index.0].key.clone())
        .collect()
}
