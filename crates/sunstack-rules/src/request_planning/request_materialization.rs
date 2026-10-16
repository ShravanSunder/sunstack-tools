use super::*;
use crate::{
    request_state::{
        build_request_state, select_owner_evidence, select_rule_evidence, wrap_question,
    },
    *,
};
use std::collections::BTreeMap;
use sunstack_decisions::{
    AdmittedBackendRequest, DecisionBackend, QuestionName, RequestBudgetAssessment,
    RequestBudgetSettings, StructuredJudgmentRequest, assess_request_budget,
};
use sunstack_evidence::{EvidenceRequirement, EvidenceSession, SelectionReadiness};
pub(crate) struct EncodedRequest {
    pub request: StructuredJudgmentRequest,
    pub admitted: AdmittedBackendRequest,
    pub bindings: Vec<QuestionBinding>,
    pub units: Vec<(UnitKey, QuestionName)>,
}
pub(crate) enum RequestAdmission {
    Within(EncodedRequest),
    Exceeded(DiagnosticList),
    Unverifiable(DiagnosticList),
    Invalid(DiagnosticList),
}
pub(crate) async fn prepare_units(
    plan: &ExecutionPlan,
    state: &mut ExecutionState,
    request: &PlannedRequest,
    session: &mut EvidenceSession,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Vec<UnitKey> {
    let mut ready = Vec::new();
    for unit in &request.units {
        if cancellation.is_cancelled() {
            break;
        }
        if unit_stopped(plan, state, *unit).is_some() {
            continue;
        }
        if !state.preparation.contains_key(unit) {
            let mut prepared = match *unit {
                UnitKey::Rule(index) => {
                    let occurrence = &plan.occurrences[index.0];
                    let group = &plan.groups[occurrence.group.0];
                    let evidence = select_rule_evidence(
                        &occurrence.rule,
                        &group.admitted.sources,
                        group.admitted.position,
                        session,
                    )
                    .await;
                    match evidence.target {
                        Ok(target) => OwnerPreparation::Ready {
                            selections: evidence.selections,
                            target: OwnerTarget::Rule(target),
                        },
                        Err(diagnostics) => OwnerPreparation::Refused {
                            selections: evidence.selections,
                            diagnostics,
                        },
                    }
                }
                UnitKey::Gate(index) => {
                    let entry = &plan.entries[index.0];
                    let group_position = match &entry.key.owner {
                        EntryOwner::Router { group }
                        | EntryOwner::RuleSet { group, .. }
                        | EntryOwner::Rule { group, .. } => *group,
                    };
                    let selections = select_owner_evidence(
                        &plan.unit_label(*unit),
                        plan.entry_evidence(index),
                        &entry.sources,
                        group_position,
                        session,
                    )
                    .await;
                    let mut refused = DiagnosticList::default();
                    for selection in &selections {
                        if selection.declaration.requirement == EvidenceRequirement::Required
                            && let SelectionReadiness::Refused { diagnostics } =
                                &selection.result.readiness
                        {
                            refused.extend(diagnostics.clone().into());
                        }
                    }
                    if refused.is_empty() {
                        OwnerPreparation::Ready {
                            selections,
                            target: OwnerTarget::Gate,
                        }
                    } else {
                        OwnerPreparation::Refused {
                            selections,
                            diagnostics: refused,
                        }
                    }
                }
            };
            if cancellation.is_cancelled() {
                let selections = match prepared {
                    OwnerPreparation::Ready { selections, .. }
                    | OwnerPreparation::Refused { selections, .. }
                    | OwnerPreparation::Cancelled { selections } => selections,
                };
                prepared = OwnerPreparation::Cancelled { selections };
            }
            if let (UnitKey::Gate(index), OwnerPreparation::Refused { diagnostics, .. }) =
                (*unit, &prepared)
            {
                let cause = FailureCause::Evidence {
                    diagnostics: diagnostics.clone(),
                };
                match plan.entries[index.0].definition {
                    EntryDefinition::Router { .. } => {
                        state
                            .routers
                            .insert(index, RouterSettlement::Failed { cause });
                    }
                    _ => {
                        state.gates.insert(index, GateSettlement::Failed { cause });
                    }
                }
            }
            if let (UnitKey::Gate(index), OwnerPreparation::Cancelled { .. }) = (*unit, &prepared) {
                match plan.entries[index.0].definition {
                    EntryDefinition::Router { .. } => {
                        state.routers.insert(index, RouterSettlement::Cancelled);
                    }
                    _ => {
                        state.gates.insert(index, GateSettlement::Cancelled);
                    }
                }
            }
            state.preparation.insert(*unit, prepared);
        }
        if matches!(
            state.preparation.get(unit),
            Some(OwnerPreparation::Ready { .. })
        ) {
            ready.push(*unit);
        }
    }
    ready
}
pub(crate) struct PlannedQuestion<'plan> {
    pub unit: UnitKey,
    pub question: &'plan sunstack_decisions::JudgmentQuestion,
    pub binding: QuestionBinding,
    common: Option<&'plan sunstack_decisions::StructuredEntry>,
}
pub(crate) fn describe_questions<'plan>(
    plan: &'plan ExecutionPlan,
    units: &[UnitKey],
) -> Result<Vec<PlannedQuestion<'plan>>, DiagnosticList> {
    let mut descriptions = Vec::new();
    for unit in units {
        let mut add = |question: &'plan sunstack_decisions::JudgmentQuestion,
                       owner: QuestionOwner,
                       common: Option<&'plan sunstack_decisions::StructuredEntry>|
         -> Result<(), DiagnosticList> {
            let request_name =
                QuestionName::new(format!("q{:06}", descriptions.len().saturating_add(1)))
                    .map_err(|_| {
                        DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError)
                    })?;
            descriptions.push(PlannedQuestion {
                unit: *unit,
                question,
                binding: QuestionBinding {
                    request_name,
                    owner,
                },
                common,
            });
            Ok(())
        };
        match *unit {
            UnitKey::Rule(index) => {
                let occurrence = &plan.occurrences[index.0];
                let rule = &occurrence.rule;
                let common = Some(
                    &plan.groups[occurrence.group.0]
                        .admitted
                        .set
                        .definition
                        .common_ruleset_instructions,
                );
                if let VerdictDeclaration::Noul { question, .. } = &rule.verdict {
                    add(
                        question,
                        QuestionOwner::Verdict {
                            rule_id: rule.rule_id.clone(),
                        },
                        common,
                    )?;
                }
                for (name, question) in &rule.questions {
                    add(
                        question,
                        QuestionOwner::Informational {
                            rule_id: rule.rule_id.clone(),
                            name: name.clone(),
                        },
                        common,
                    )?;
                }
            }
            UnitKey::Gate(index) => {
                let entry = &plan.entries[index.0];
                match &entry.definition {
                    EntryDefinition::Router {
                        definition,
                        position,
                    } => add(
                        &definition.question,
                        QuestionOwner::Router { group: *position },
                        None,
                    )?,
                    EntryDefinition::Decision { question, .. } => {
                        let owner = match &entry.key.owner {
                            EntryOwner::Rule { rule_id, .. } => QuestionOwner::RuleDecision {
                                rule_id: rule_id.clone(),
                                entry: entry.key.clone(),
                            },
                            _ => QuestionOwner::RuleSetDecision {
                                entry: entry.key.clone(),
                            },
                        };
                        add(question, owner, None)?;
                    }
                    EntryDefinition::Condition(_) => {
                        return Err(DiagnosticList::rule(
                            RuleDiagnosticCode::UnexpectedExecutionError,
                        ));
                    }
                }
            }
        }
    }
    Ok(descriptions)
}
pub(crate) struct RenderedRequest {
    pub request: StructuredJudgmentRequest,
    pub bindings: Vec<QuestionBinding>,
    pub units: Vec<(UnitKey, QuestionName)>,
}
pub(crate) fn render_units(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    units: &[UnitKey],
    omit_optional: bool,
) -> Result<RenderedRequest, DiagnosticList> {
    let ready = units
        .iter()
        .copied()
        .filter(|unit| {
            matches!(
                state.preparation.get(unit),
                Some(OwnerPreparation::Ready { .. })
            )
        })
        .collect::<Vec<_>>();
    let mut questions = BTreeMap::new();
    let mut bindings = Vec::new();
    let mut unit_bindings = Vec::new();
    for described in describe_questions(plan, &ready)? {
        let Some(OwnerPreparation::Ready { selections, .. }) =
            state.preparation.get(&described.unit)
        else {
            continue;
        };
        let reads = selections
            .iter()
            .map(|selection| selection.label.clone())
            .collect();
        questions.insert(
            described.binding.request_name.clone(),
            wrap_question(
                described.question,
                &plan.unit_label(described.unit),
                reads,
                described.common,
            )?,
        );
        unit_bindings.push((described.unit, described.binding.request_name.clone()));
        bindings.push(described.binding);
    }
    let selections = ready
        .iter()
        .flat_map(|unit| match state.preparation.get(unit) {
            Some(OwnerPreparation::Ready { selections, .. }) => selections.clone(),
            _ => vec![],
        });
    Ok(RenderedRequest {
        request: StructuredJudgmentRequest {
            questions,
            state: build_request_state(selections, omit_optional)?,
        },
        bindings,
        units: unit_bindings,
    })
}
pub(crate) fn encode_units(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    units: &[UnitKey],
    omit_optional: bool,
    backend: &dyn DecisionBackend,
    budget: &RequestBudgetSettings,
) -> RequestAdmission {
    let RenderedRequest {
        request,
        bindings,
        units: unit_bindings,
    } = match render_units(plan, state, units, omit_optional) {
        Ok(rendered) => rendered,
        Err(diagnostics) => return RequestAdmission::Invalid(diagnostics),
    };
    let encoded = match backend.encode_request(&request) {
        Ok(encoded) => encoded,
        Err(diagnostic) => {
            return RequestAdmission::Invalid(
                sunstack_decisions::DiagnosticList::from(diagnostic).into(),
            );
        }
    };
    let accounting = backend.account_request(&encoded);
    match assess_request_budget(encoded, budget, accounting) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => {
            RequestAdmission::Within(EncodedRequest {
                request,
                admitted: admitted_request,
                bindings,
                units: unit_bindings,
            })
        }
        RequestBudgetAssessment::Exceeded { diagnostics } => {
            RequestAdmission::Exceeded(diagnostics.into())
        }
        RequestBudgetAssessment::Unverifiable { diagnostics } => {
            RequestAdmission::Unverifiable(diagnostics.into())
        }
    }
}
pub(crate) fn has_optional(state: &ExecutionState, units: &[UnitKey]) -> bool {
    units.iter().any(|unit|matches!(state.preparation.get(unit),Some(OwnerPreparation::Ready{selections,..}) if selections.iter().any(|selection|selection.declaration.requirement==EvidenceRequirement::Optional)))
}
pub(crate) fn preparation_diagnostics(state: &ExecutionState, units: &[UnitKey]) -> DiagnosticList {
    let mut diagnostics = DiagnosticList::default();
    for unit in units {
        if let Some(OwnerPreparation::Refused {
            diagnostics: refused,
            ..
        }) = state.preparation.get(unit)
        {
            diagnostics.extend(refused.clone());
        }
    }
    diagnostics
}
