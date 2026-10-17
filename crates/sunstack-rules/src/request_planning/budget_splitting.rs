use super::*;
use crate::*;
use sunstack_decisions::{DecisionBackend, RequestBudgetSettings};

pub(crate) struct BudgetPart {
    pub units: Vec<UnitKey>,
    pub admission: RequestAdmission,
}
fn finish_part(
    parts: &mut Vec<BudgetPart>,
    units: &mut Vec<UnitKey>,
    admission: &mut Option<RequestAdmission>,
) {
    if let Some(admission) = admission.take() {
        parts.push(BudgetPart {
            units: std::mem::take(units),
            admission,
        });
    }
}
/// Assess complete rule blocks in authored order. Gates stay with the first block when it fits.
pub(crate) fn split_rules_request(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    units: &[UnitKey],
    omit_optional: bool,
    backend: &dyn DecisionBackend,
    budget: &RequestBudgetSettings,
) -> Vec<BudgetPart> {
    let mut parts = Vec::new();
    let mut current = units
        .iter()
        .copied()
        .filter(|unit| matches!(unit, UnitKey::Gate(_)))
        .collect::<Vec<_>>();
    let mut accepted = None;
    for rule in units
        .iter()
        .copied()
        .filter(|unit| matches!(unit, UnitKey::Rule(_)))
    {
        let mut candidate = current.clone();
        candidate.push(rule);
        match encode_units(plan, state, &candidate, omit_optional, backend, budget) {
            admission @ RequestAdmission::Within(_) => {
                current = candidate;
                accepted = Some(admission);
                continue;
            }
            RequestAdmission::Exceeded(_) => {}
            admission => {
                finish_part(&mut parts, &mut current, &mut accepted);
                parts.push(BudgetPart {
                    units: vec![rule],
                    admission,
                });
                continue;
            }
        }
        if current.iter().any(|unit| matches!(unit, UnitKey::Rule(_))) {
            finish_part(&mut parts, &mut current, &mut accepted);
        } else {
            // The carried gates cannot share split zero with its first rule. Their slots keep
            // one controlling request each, sent concurrently rather than made prerequisites.
            for gate in current.drain(..) {
                parts.push(BudgetPart {
                    units: vec![gate],
                    admission: encode_units(plan, state, &[gate], omit_optional, backend, budget),
                });
            }
        }
        match encode_units(plan, state, &[rule], omit_optional, backend, budget) {
            admission @ RequestAdmission::Within(_) => {
                current = vec![rule];
                accepted = Some(admission)
            }
            RequestAdmission::Exceeded(_) => parts.push(BudgetPart {
                units: vec![rule],
                admission: RequestAdmission::Exceeded(DiagnosticList::rule(
                    RuleDiagnosticCode::RuleExceedsBudget,
                )),
            }),
            admission => parts.push(BudgetPart {
                units: vec![rule],
                admission,
            }),
        }
    }
    finish_part(&mut parts, &mut current, &mut accepted);
    parts
}
/// Replace the over-budget node; private ids remain stable until final report projection.
pub(crate) fn install_budget_parts(
    plan: &mut ExecutionPlan,
    state: &mut ExecutionState,
    original: &PlannedRequest,
    parts: Vec<BudgetPart>,
) -> Vec<(RequestId, Vec<UnitKey>, EncodedRequest)> {
    let total = parts
        .iter()
        .filter(|part| {
            part.units
                .iter()
                .any(|unit| matches!(unit, UnitKey::Rule(_)))
        })
        .count();
    let mut ordinal = 0;
    let mut encoded = Vec::new();
    for (position, part) in parts.into_iter().enumerate() {
        let id = if position == 0 {
            original.id
        } else {
            RequestId(plan.requests.len())
        };
        let only_gate = match part.units.as_slice() {
            [UnitKey::Gate(entry)] => Some(*entry),
            _ => None,
        };
        let mut request = original.clone();
        request.id = id;
        request.units = part.units.clone();
        if let Some(entry) = only_gate {
            request.purpose = RequestPurpose::Gate { entry };
            request.mode = RequestMode::Speculative;
            if let EntryOwner::Router { group } = plan.entries[entry.0].key.owner {
                request.owner = RequestOwner::Router { group };
                request.routing = None;
                request.waits_for_route = false;
            }
        } else {
            request.mode = RequestMode::Split {
                part: ordinal,
                of: total,
            };
            ordinal += 1;
        }
        for unit in &part.units {
            if let UnitKey::Gate(entry) = unit {
                plan.controls.insert(*entry, id);
            }
        }
        if position == 0 {
            plan.requests[id.0] = request.clone()
        } else {
            plan.requests.push(request.clone())
        }
        state.requests.insert(id, RequestSettlement::Waiting);
        match part.admission {
            RequestAdmission::Within(admitted) => encoded.push((id, part.units, admitted)),
            RequestAdmission::Exceeded(diagnostics)
            | RequestAdmission::Unverifiable(diagnostics)
            | RequestAdmission::Invalid(diagnostics) => {
                for unit in &part.units {
                    state.budget_refusals.insert(*unit, diagnostics.clone());
                }
                mark_not_sent(
                    plan,
                    state,
                    &request,
                    RequestRefusal::Budget { diagnostics },
                );
            }
        }
    }
    encoded
}
