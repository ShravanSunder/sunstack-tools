use super::*;
use crate::*;
use std::collections::BTreeSet;
use sunstack_evidence::{
    BeforeUnknownReason, ChangeComparison, ChangeComparisonSubject, ComparisonLevel,
    EvidenceDeclaration, EvidenceSession,
};

#[derive(Clone, Copy)]
enum DependencyRole {
    Definition,
    Target,
    Context,
    Applicability,
}
enum DependencySubject {
    Evidence(ChangeComparisonSubject),
    Origin,
}
struct SelectionDependency {
    role: DependencyRole,
    subject: DependencySubject,
}

pub(crate) async fn plan_change_selection(plan: &mut ExecutionPlan, session: &mut EvidenceSession) {
    for position in 0..plan.occurrences.len() {
        let index = OccurrenceIndex(position);
        if rule_pruned(plan, index) {
            continue;
        }
        plan.occurrences[position].scope =
            compare_occurrence(plan, index, ComparisonLevel::Path, session)
                .await
                .unwrap_or_else(|_| ScopeEvaluation::pending());
    }
    for position in 0..plan.evaluations.len() {
        let evaluation = &plan.evaluations[position];
        plan.evaluations[position].scope = compare_rule(
            plan,
            &evaluation.rule,
            evaluation.group,
            &evaluation.entries,
            ComparisonLevel::Path,
            session,
        )
        .await
        .unwrap_or_else(|_| ScopeEvaluation::pending());
    }
}

pub(crate) async fn finish_evaluation_scope(
    plan: &ExecutionPlan,
    index: EvaluationIndex,
    session: &mut EvidenceSession,
) -> Result<ScopeEvaluation, DiagnosticList> {
    let evaluation = &plan.evaluations[index.0];
    if matches!(
        evaluation.scope.disposition,
        ScopeDisposition::InScope {
            selection: OccurrenceSelection::ComparisonPending {}
        }
    ) {
        compare_rule(
            plan,
            &evaluation.rule,
            evaluation.group,
            &evaluation.entries,
            ComparisonLevel::Content,
            session,
        )
        .await
    } else {
        Ok(evaluation.scope.clone())
    }
}
pub(crate) fn effective_scope<'state>(
    plan: &'state ExecutionPlan,
    state: &'state ExecutionState,
    index: OccurrenceIndex,
) -> &'state ScopeEvaluation {
    state
        .scopes
        .get(&index)
        .unwrap_or(&plan.occurrences[index.0].scope)
}
pub(crate) fn occurrence_selection(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    index: OccurrenceIndex,
) -> OccurrenceSelection {
    match &effective_scope(plan, state, index).disposition {
        ScopeDisposition::InScope { selection } => selection.clone(),
        ScopeDisposition::OutsideChangeScope => {
            match &plan.occurrences[index.0].scope.disposition {
                ScopeDisposition::InScope { selection } => selection.clone(),
                ScopeDisposition::OutsideChangeScope => OccurrenceSelection::ComparisonPending {},
            }
        }
    }
}
pub(crate) fn request_outside_scope(state: &ExecutionState, units: &[UnitKey]) -> bool {
    !units.is_empty()
        && units.iter().all(|unit| {
            matches!(
                state.preparation.get(unit),
                Some(OwnerPreparation::OutsideChangeScope)
            )
        })
}
pub(crate) async fn finish_change_selection(
    plan: &ExecutionPlan,
    state: &mut ExecutionState,
    index: OccurrenceIndex,
    session: &mut EvidenceSession,
) -> Result<TargetPresence, DiagnosticList> {
    if matches!(
        &effective_scope(plan, state, index).disposition,
        ScopeDisposition::InScope {
            selection: OccurrenceSelection::ComparisonPending {}
        }
    ) {
        let rule = &plan.occurrences[index.0].rule;
        let change_presence_target = matches!(
            &rule.verdict,
            VerdictDeclaration::Mechanical {
                check: MechanicalCheck::Expect {
                    expect: ExpectPredicate::Present { .. }
                },
                ..
            }
        ) && rule.evidence.iter().any(|declaration| {
            declaration.id == rule.target
                && matches!(
                    declaration.select,
                    sunstack_evidence::EvidenceSelection::ChangeSetLiteralText { .. }
                        | sunstack_evidence::EvidenceSelection::ChangeSetRegexText { .. }
                        | sunstack_evidence::EvidenceSelection::ChangeSetAstPattern { .. }
                )
        });
        if change_presence_target {
            let selected_by = SelectedReasons::try_from(vec![SelectedReason::TargetChanged])
                .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
            state.scopes.insert(
                index,
                ScopeEvaluation {
                    disposition: ScopeDisposition::InScope {
                        selection: OccurrenceSelection::Selected { selected_by },
                    },
                    target: TargetPresence::ResolveCurrent,
                },
            );
            return Ok(TargetPresence::ResolveCurrent);
        }
        let result = compare_occurrence(plan, index, ComparisonLevel::Content, session).await?;
        state.scopes.insert(index, result);
    }
    Ok(effective_scope(plan, state, index).target)
}
async fn compare_occurrence(
    plan: &ExecutionPlan,
    index: OccurrenceIndex,
    level: ComparisonLevel,
    session: &mut EvidenceSession,
) -> Result<ScopeEvaluation, DiagnosticList> {
    let occurrence = &plan.occurrences[index.0];
    compare_rule(
        plan,
        &occurrence.rule,
        occurrence.group,
        &occurrence.entries,
        level,
        session,
    )
    .await
}

async fn compare_rule(
    plan: &ExecutionPlan,
    rule: &RuleDefinition,
    group: GroupIndex,
    entries: &[EntryIndex],
    level: ComparisonLevel,
    session: &mut EvidenceSession,
) -> Result<ScopeEvaluation, DiagnosticList> {
    let dependencies = dependencies(plan, rule, group, entries)?;
    let mut reasons = BTreeSet::new();
    let mut pending = false;
    let mut deferred_refusal = false;
    let mut target = TargetPresence::ResolveCurrent;
    for dependency in dependencies {
        let subject = match dependency.subject {
            DependencySubject::Origin => {
                reasons.insert(SelectedReason::FullScope);
                continue;
            }
            DependencySubject::Evidence(subject) => subject,
        };
        let dependency_level = if matches!(dependency.role, DependencyRole::Definition) {
            ComparisonLevel::Path
        } else {
            level
        };
        let comparison = session
            .compare_change_sides(subject, dependency_level)
            .await;
        if matches!(dependency.role, DependencyRole::Target)
            && matches!(comparison, ChangeComparison::Deleted { .. })
        {
            target = TargetPresence::Deleted;
        }
        match comparison {
            ChangeComparison::Unchanged {} => {}
            ChangeComparison::PathChanged { .. } => match dependency.role {
                DependencyRole::Definition => {
                    reasons.insert(SelectedReason::RuleChanged);
                }
                DependencyRole::Target
                | DependencyRole::Context
                | DependencyRole::Applicability => pending = true,
            },
            ChangeComparison::Added { .. } => {
                reasons.insert(changed_reason(dependency.role, true));
            }
            ChangeComparison::Changed { .. } | ChangeComparison::Deleted { .. } => {
                reasons.insert(changed_reason(dependency.role, false));
            }
            ChangeComparison::NoBeforeSide { .. } => {
                reasons.insert(SelectedReason::FullScope);
            }
            ChangeComparison::BeforeUnknown { reason } => {
                reasons.insert(match reason {
                    BeforeUnknownReason::BaselineUnavailable => SelectedReason::BaselineUnavailable,
                    BeforeUnknownReason::HistoryIncomplete => SelectedReason::BeforeAnalysisUnknown,
                });
            }
            ChangeComparison::Refused { diagnostics } => {
                if level == ComparisonLevel::Content {
                    return Err(diagnostics.into());
                }
                deferred_refusal = true;
            }
        }
    }
    let disposition = if deferred_refusal {
        ScopeDisposition::InScope {
            selection: OccurrenceSelection::ComparisonPending {},
        }
    } else if !reasons.is_empty() {
        let selected_by = SelectedReasons::try_from(reasons.into_iter().collect::<Vec<_>>())
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
        ScopeDisposition::InScope {
            selection: OccurrenceSelection::Selected { selected_by },
        }
    } else if pending {
        ScopeDisposition::InScope {
            selection: OccurrenceSelection::ComparisonPending {},
        }
    } else {
        ScopeDisposition::OutsideChangeScope
    };
    Ok(ScopeEvaluation {
        disposition,
        target,
    })
}
fn changed_reason(role: DependencyRole, added: bool) -> SelectedReason {
    match role {
        DependencyRole::Definition if added => SelectedReason::NewRule,
        DependencyRole::Definition => SelectedReason::RuleChanged,
        DependencyRole::Target => SelectedReason::TargetChanged,
        DependencyRole::Context => SelectedReason::ContextChanged,
        DependencyRole::Applicability => SelectedReason::ApplicabilityChanged,
    }
}
fn dependencies(
    plan: &ExecutionPlan,
    rule: &RuleDefinition,
    group: GroupIndex,
    entries: &[EntryIndex],
) -> Result<Vec<SelectionDependency>, DiagnosticList> {
    let group = &plan.groups[group.0];
    let located = group
        .admitted
        .set
        .rules
        .iter()
        .find(|located| located.definition.rule_id == rule.rule_id)
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    let mut dependencies = vec![
        SelectionDependency {
            role: DependencyRole::Definition,
            subject: DependencySubject::Evidence(ChangeComparisonSubject::DefinitionFile {
                path: located.source.clone(),
            }),
        },
        SelectionDependency {
            role: DependencyRole::Definition,
            subject: DependencySubject::Evidence(ChangeComparisonSubject::DefinitionFile {
                path: group.admitted.set.path.clone(),
            }),
        },
    ];
    if let Some(provenance) = &located.provenance {
        for path in [&provenance.markdown, &provenance.rules] {
            if path != &located.source {
                dependencies.push(SelectionDependency {
                    role: DependencyRole::Definition,
                    subject: DependencySubject::Evidence(ChangeComparisonSubject::DefinitionFile {
                        path: path.clone(),
                    }),
                });
            }
        }
    }
    for declaration in &rule.evidence {
        let role = if declaration.id == rule.target {
            DependencyRole::Target
        } else {
            DependencyRole::Context
        };
        push_declaration(
            &mut dependencies,
            role,
            declaration,
            &group.admitted.sources,
        )?;
    }
    for entry_index in entries {
        let entry = &plan.entries[entry_index.0];
        match &entry.definition {
            EntryDefinition::Decision { evidence, .. } => {
                for declaration in evidence {
                    push_declaration(
                        &mut dependencies,
                        DependencyRole::Applicability,
                        declaration,
                        &entry.sources,
                    )?;
                }
            }
            EntryDefinition::Condition(test) => match test {
                ConditionTest::OriginFieldEquals { .. } => dependencies.push(SelectionDependency {
                    role: DependencyRole::Applicability,
                    subject: DependencySubject::Origin,
                }),
                ConditionTest::PayloadFieldEquals { input, .. }
                | ConditionTest::PayloadFieldIn { input, .. }
                | ConditionTest::PayloadFieldPresent { input, .. }
                | ConditionTest::PatternPresent { input, .. } => {
                    let source = entry
                        .sources
                        .get(input)
                        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UndeclaredInput))?;
                    dependencies.push(SelectionDependency {
                        role: DependencyRole::Applicability,
                        subject: DependencySubject::Evidence(ChangeComparisonSubject::Input {
                            input: source.name.clone(),
                        }),
                    });
                }
            },
            EntryDefinition::Router { .. } => {}
        }
    }
    Ok(dependencies)
}
fn push_declaration(
    dependencies: &mut Vec<SelectionDependency>,
    role: DependencyRole,
    declaration: &EvidenceDeclaration,
    sources: &crate::input_binding::AdmittedSources,
) -> Result<(), DiagnosticList> {
    let source = sources
        .get(&declaration.input)
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UndeclaredInput))?;
    let mut declaration = declaration.clone();
    declaration.input = source.name.clone();
    dependencies.push(SelectionDependency {
        role,
        subject: DependencySubject::Evidence(ChangeComparisonSubject::Declaration { declaration }),
    });
    Ok(())
}
