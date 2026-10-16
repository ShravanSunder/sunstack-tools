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
pub(crate) fn encode_units(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    units: &[UnitKey],
    omit_optional: bool,
    backend: &dyn DecisionBackend,
    budget: &RequestBudgetSettings,
) -> RequestAdmission {
    let mut questions = BTreeMap::new();
    let mut bindings = Vec::new();
    let mut unit_bindings = Vec::new();
    for unit in units {
        let Some(OwnerPreparation::Ready { selections, .. }) = state.preparation.get(unit) else {
            continue;
        };
        let reads = selections
            .iter()
            .map(|selection| selection.label.clone())
            .collect::<Vec<_>>();
        let owner = plan.unit_label(*unit);
        let mut add = |question: &sunstack_decisions::JudgmentQuestion,
                       question_owner: QuestionOwner,
                       common: Option<&sunstack_decisions::StructuredEntry>|
         -> Result<(), DiagnosticList> {
            let request_name =
                QuestionName::new(format!("q{:06}", questions.len().saturating_add(1))).map_err(
                    |_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                )?;
            questions.insert(
                request_name.clone(),
                wrap_question(question, &owner, reads.clone(), common)?,
            );
            bindings.push(QuestionBinding {
                request_name: request_name.clone(),
                owner: question_owner,
            });
            unit_bindings.push((*unit, request_name));
            Ok(())
        };
        let result = match *unit {
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
                let mut result = Ok(());
                if let VerdictDeclaration::Noul { question, .. } = &rule.verdict {
                    result = add(
                        question,
                        QuestionOwner::Verdict {
                            rule_id: rule.rule_id.clone(),
                        },
                        common,
                    );
                }
                if result.is_ok() {
                    for (name, question) in &rule.questions {
                        if let Err(error) = add(
                            question,
                            QuestionOwner::Informational {
                                rule_id: rule.rule_id.clone(),
                                name: name.clone(),
                            },
                            common,
                        ) {
                            result = Err(error);
                            break;
                        }
                    }
                }
                result
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
                    ),
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
                        add(question, owner, None)
                    }
                    EntryDefinition::Condition(_) => Err(DiagnosticList::rule(
                        RuleDiagnosticCode::UnexpectedExecutionError,
                    )),
                }
            }
        };
        if let Err(diagnostics) = result {
            return RequestAdmission::Invalid(diagnostics);
        }
    }
    let selections = units
        .iter()
        .flat_map(|unit| match state.preparation.get(unit) {
            Some(OwnerPreparation::Ready { selections, .. }) => selections.clone(),
            _ => vec![],
        });
    let request_state = match build_request_state(selections, omit_optional) {
        Ok(state) => state,
        Err(diagnostics) => return RequestAdmission::Invalid(diagnostics),
    };
    let request = StructuredJudgmentRequest {
        questions,
        state: request_state,
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
