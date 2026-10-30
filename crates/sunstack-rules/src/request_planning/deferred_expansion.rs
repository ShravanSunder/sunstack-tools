use super::*;
use crate::{
    rule_evaluation::request_state::{LabeledSelection, select_owner_evidence},
    *,
};
use sunstack_evidence::{
    MarkdownByteRange, MarkdownOccurrence, SelectionContent, SelectionCoverage, SelectionReadiness,
};
use tokio_util::sync::CancellationToken;

fn byte_range(start: usize, end: usize) -> MarkdownByteRange {
    MarkdownByteRange { start, end }
}

fn target_identity(content: &SelectionContent) -> Option<RuleOccurrenceTarget> {
    match content {
        SelectionContent::MarkdownUnits { units } if units.len() == 1 => match &units[0].occurrence
        {
            MarkdownOccurrence::Unit { span } => Some(RuleOccurrenceTarget::MarkdownUnit {
                source: span.source.clone(),
                range: byte_range(span.byte_start, span.byte_end),
            }),
            MarkdownOccurrence::CallSite {
                unit_span,
                match_span,
                visible_range,
            } => Some(RuleOccurrenceTarget::CallSite {
                source: unit_span.source.clone(),
                unit_range: byte_range(unit_span.byte_start, unit_span.byte_end),
                match_range: byte_range(match_span.byte_start, match_span.byte_end),
                visible_range: visible_range.clone(),
            }),
        },
        SelectionContent::FrontmatterField { result } => {
            use sunstack_evidence::FrontmatterFieldContent::*;
            let (source, field) = match result.as_ref() {
                Present { source, field, .. }
                | Absent { source, field, .. }
                | BlockUnavailable { source, field, .. } => (source, field),
            };
            Some(RuleOccurrenceTarget::FrontmatterField {
                source: source.clone(),
                field: field.clone(),
            })
        }
        _ => None,
    }
}

fn terminal_stop(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    entry: EntryIndex,
) -> ExpansionState {
    if let Some(routing) = plan
        .evaluations
        .iter()
        .filter_map(|evaluation| plan.groups[evaluation.group.0].routing.as_ref())
        .find(|routing| routing.entry == entry)
    {
        match route_state(plan, state, routing) {
            RouteState::Failed { cause } => return ExpansionState::Failed { cause },
            RouteState::Cancelled => return ExpansionState::Cancelled,
            _ => {}
        }
    }
    match entry_state(entry, state) {
        PrefixState::Failed { cause, .. } => ExpansionState::Failed { cause },
        PrefixState::Cancelled { .. } => ExpansionState::Cancelled,
        _ => match state.gates.get(&entry) {
            Some(GateSettlement::Failed { cause }) => ExpansionState::Failed {
                cause: cause.clone(),
            },
            Some(GateSettlement::Cancelled) => ExpansionState::Cancelled,
            _ => ExpansionState::Stopped { entry },
        },
    }
}

/// Discovery obeys the existing carrier frontier, before encoding or dispatch.
pub(crate) async fn discover_ready_rules(
    plan: &mut ExecutionPlan,
    state: &mut ExecutionState,
    session: &mut sunstack_evidence::EvidenceSession,
    cancellation: &CancellationToken,
) -> bool {
    let mut changed = false;
    for position in 0..plan.evaluations.len() {
        if !matches!(state.expansions[position], ExpansionState::Waiting) {
            continue;
        }
        let id = EvaluationIndex(position);
        let evaluation = &plan.evaluations[position];
        if cancellation.is_cancelled() {
            state.expansions[position] = ExpansionState::Cancelled;
            changed = true;
            continue;
        }
        if matches!(
            evaluation.scope.disposition,
            ScopeDisposition::OutsideChangeScope
        ) {
            state.expansions[position] = ExpansionState::OutsideChangeScope;
            changed = true;
            continue;
        }
        if let Some(entry) = stopped_by_entries(
            plan,
            state,
            &evaluation.entries,
            plan.groups[evaluation.group.0].routing.as_ref(),
        ) {
            state.expansions[position] = terminal_stop(plan, state, entry);
            changed = true;
            continue;
        }
        let carrier = plan.requests.iter().find(|request| matches!(request.purpose, RequestPurpose::Rules { group } if group == evaluation.group));
        let Some(carrier) = carrier else {
            state.expansions[position] = ExpansionState::Failed {
                cause: FailureCause::Evidence {
                    diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                },
            };
            changed = true;
            continue;
        };
        if matches!(
            request_readiness(plan, state, carrier),
            RequestReadiness::Waiting
        ) {
            continue;
        }
        let carrier_id = carrier.id;
        let scope = match finish_evaluation_scope(plan, id, session).await {
            Ok(scope) => scope,
            Err(diagnostics) => {
                state.expansions[position] = ExpansionState::SelectionFailed { diagnostics };
                changed = true;
                continue;
            }
        };
        plan.evaluations[position].scope = scope;
        if matches!(
            plan.evaluations[position].scope.disposition,
            ScopeDisposition::OutsideChangeScope
        ) {
            state.expansions[position] = ExpansionState::OutsideChangeScope;
            changed = true;
            continue;
        }
        let evaluation = &plan.evaluations[position];
        let Some(target) = evaluation
            .rule
            .evidence
            .iter()
            .find(|declaration| declaration.id == evaluation.rule.target)
        else {
            continue;
        };
        let group = &plan.groups[evaluation.group.0];
        let mut selections = select_owner_evidence(
            "rule_discovery",
            std::slice::from_ref(target),
            &group.admitted.sources,
            group.admitted.position,
            session,
        )
        .await;
        if cancellation.is_cancelled() {
            state.expansions[position] = ExpansionState::Cancelled;
            changed = true;
            continue;
        }
        let Some(selection) = selections.pop() else {
            continue;
        };
        let result =
            install_discovered_occurrences(plan, state, id, carrier_id, selection, cancellation);
        if let Err(diagnostics) = result {
            state.expansions[position] = ExpansionState::SelectionFailed { diagnostics };
        }
        changed = true;
    }
    changed
}

/// Append stable child indices into a waiting carrier; gate ownership never moves.
fn install_discovered_occurrences(
    plan: &mut ExecutionPlan,
    state: &mut ExecutionState,
    id: EvaluationIndex,
    carrier: RequestId,
    selection: LabeledSelection,
    cancellation: &CancellationToken,
) -> Result<(), DiagnosticList> {
    if cancellation.is_cancelled() {
        state.expansions[id.0] = ExpansionState::Cancelled;
        return Ok(());
    }
    if !matches!(
        state.requests.get(&carrier),
        Some(RequestSettlement::Waiting)
    ) {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::UnexpectedExecutionError,
        ));
    }
    let (contents, source) = match &selection.result.readiness {
        SelectionReadiness::Refused { diagnostics } => return Err(diagnostics.clone().into()),
        SelectionReadiness::Ready {
            content, coverage, ..
        } => {
            let SelectionCoverage::SingleSource { capture } = coverage.as_ref() else {
                return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget));
            };
            let contents = match content {
                SelectionContent::MarkdownUnits { units } => units
                    .iter()
                    .cloned()
                    .map(|unit| SelectionContent::MarkdownUnits { units: vec![unit] })
                    .collect::<Vec<_>>(),
                SelectionContent::FrontmatterField { .. } => vec![content.clone()],
                _ => return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget)),
            };
            (contents, capture.source.clone())
        }
    };
    if contents.is_empty() {
        state.expansions[id.0] = ExpansionState::ZeroMatches { source };
        return Ok(());
    }
    let mut children = Vec::new();
    for content in contents {
        let Some(target) = target_identity(&content) else {
            return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget));
        };
        let mut selected = selection.clone();
        if let SelectionReadiness::Ready {
            content: selected_content,
            ..
        } = &mut selected.result.readiness
        {
            *selected_content = content;
        }
        let evaluation = &plan.evaluations[id.0];
        let index = OccurrenceIndex(plan.occurrences.len());
        let key = RuleOccurrenceKey {
            rule_set: evaluation.key.rule_set.clone(),
            group: evaluation.key.group,
            rule_id: evaluation.key.rule_id.clone(),
            target: Box::new(target),
        };
        plan.occurrences.push(PlannedOccurrence {
            key,
            rule: evaluation.rule.clone(),
            group: evaluation.group,
            entries: evaluation.entries.clone(),
            scope: evaluation.scope.clone(),
            authored_order: evaluation.authored_order,
            target_selection: Some(selected),
        });
        children.push(index);
        plan.groups[evaluation.group.0].occurrences.push(index);
        plan.requests[carrier.0].units.push(UnitKey::Rule(index));
    }
    let occurrences = &plan.occurrences;
    children.sort_by_key(|index| occurrence_order(&occurrences[index.0]));
    let group = plan.evaluations[id.0].group;
    plan.groups[group.0]
        .occurrences
        .sort_by_key(|index| occurrence_order(&occurrences[index.0]));
    plan.requests[carrier.0]
        .units
        .sort_by_key(|unit| match unit {
            UnitKey::Gate(index) => (0, index.0, 0, 0, 0),
            UnitKey::Rule(index) => {
                let (authored, unit, matched, visible) = occurrence_order(&occurrences[index.0]);
                (1, authored, unit, matched, visible)
            }
        });
    state.expansions[id.0] = ExpansionState::Expanded { children };
    Ok(())
}

pub(crate) fn finish_empty_carriers(plan: &ExecutionPlan, state: &mut ExecutionState) {
    for request in &plan.requests {
        let no_questions = request.units.is_empty()
            || request.units.iter().any(|unit| plan.unit_is_mechanical(*unit))
                && request.units.iter().all(|unit| {
                    unit_stopped(plan,state,*unit).is_some()
                        || matches!(unit,UnitKey::Rule(index) if plan.unit_is_mechanical(*unit) && state.local_settlements.contains_key(index))
                });
        if no_questions
            && matches!(
                state.requests.get(&request.id),
                Some(RequestSettlement::Waiting)
            )
        {
            let RequestPurpose::Rules { group } = request.purpose else {
                continue;
            };
            if plan.groups[group.0]
                .evaluations
                .iter()
                .all(|index| !matches!(state.expansions[index.0], ExpansionState::Waiting))
            {
                state.requests.insert(
                    request.id,
                    RequestSettlement::Settled {
                        status: RequestStatus::NotSent {
                            reason: NotSentReason::NoQuestions,
                        },
                        bindings: vec![],
                        unit_bindings: vec![],
                    },
                );
            }
        }
    }
}
