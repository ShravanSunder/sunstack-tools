use crate::{request_planning::*, *};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{EvidenceCode, SelectionReadiness, SourceName};

#[derive(Default)]
pub(crate) struct CandidateExecutionData {
    pub elapsed_ms: BTreeMap<GroupKey, u64>,
    pub evidence_failures: BTreeMap<GroupKey, Vec<QuestionEvidenceFailure>>,
    pub evidence_identities: BTreeMap<GroupKey, BTreeMap<SourceName, BTreeSet<String>>>,
    pub outside_privacy: sunstack_evidence::PrivacyProjection,
}
pub(crate) struct QuestionEvidenceFailure {
    pub at: QuestionRef,
    pub input: SourceName,
    pub diagnostic: Option<EvidenceCode>,
}

pub(crate) fn project_evidence_data(
    plan: &ExecutionPlan,
    state: &ExecutionState,
    data: &mut CandidateExecutionData,
) {
    for (unit, preparation) in &state.preparation {
        let selections = match preparation {
            OwnerPreparation::Ready { selections, .. }
            | OwnerPreparation::Refused { selections, .. }
            | OwnerPreparation::Cancelled { selections } => selections,
            OwnerPreparation::OutsideChangeScope => continue,
        };
        let groups = match unit {
            UnitKey::Rule(index) => vec![(
                plan.occurrences[index.0].group,
                QuestionRef::Verdict {},
                &plan.groups[plan.occurrences[index.0].group.0]
                    .admitted
                    .sources,
            )],
            UnitKey::Gate(index) => {
                let entry = &plan.entries[index.0];
                match entry.group {
                    Some(group) => vec![(
                        group,
                        entry_reference(plan, group, &entry.key),
                        entry.sources.as_ref(),
                    )],
                    None => plan
                        .groups
                        .iter()
                        .enumerate()
                        .filter(|(_, group)| {
                            group
                                .routing
                                .as_ref()
                                .is_some_and(|route| route.entry == *index)
                        })
                        .map(|(position, _)| {
                            (
                                GroupIndex(position),
                                QuestionRef::Router {},
                                entry.sources.as_ref(),
                            )
                        })
                        .collect(),
                }
            }
        };
        for (group, at, sources) in groups {
            let admitted = &plan.groups[group.0].admitted;
            for selection in selections {
                if sources
                    .get(&selection.declaration.input)
                    .is_some_and(|source| {
                        source.candidate_record().is_some() || source.candidate_code().is_some()
                    })
                {
                    continue;
                }
                let input = if matches!(at, QuestionRef::Router {}) {
                    selection.declaration.input.clone()
                } else {
                    root_input(plan, group, &selection.declaration.input)
                };
                let key = GroupKey {
                    rule_set: admitted.set.path.clone(),
                    group: admitted.position,
                };
                let diagnostics = match &selection.result.readiness {
                    SelectionReadiness::Ready { coverage, .. } => {
                        let identities = data
                            .evidence_identities
                            .entry(key)
                            .or_default()
                            .entry(input.clone())
                            .or_default();
                        if let Some(source) = sources.get(&selection.declaration.input)
                            && let crate::input_binding::SourceId::Payload {
                                canonical_value_identity,
                            } = &source.id
                        {
                            identities.insert(canonical_value_identity.clone());
                        } else {
                            let sources = match coverage.as_ref() {
                                sunstack_evidence::SelectionCoverage::SingleSource { capture } => {
                                    vec![&capture.source]
                                }
                                sunstack_evidence::SelectionCoverage::ChangeSet {
                                    sources, ..
                                } => sources.iter().collect(),
                            };
                            for source in sources {
                                match source {
                                    sunstack_evidence::SourceRef::Document {
                                        content_identity,
                                        ..
                                    } => {
                                        identities.insert(content_identity.clone());
                                    }
                                    sunstack_evidence::SourceRef::RepositoryFile { .. } => {
                                        if let Ok(bytes) = serde_json::to_vec(source) {
                                            identities
                                                .insert(sunstack_evidence::content_digest(&bytes));
                                        }
                                    }
                                    sunstack_evidence::SourceRef::Transcript { .. }
                                    | sunstack_evidence::SourceRef::Payload { .. } => {}
                                }
                            }
                        }
                        continue;
                    }
                    SelectionReadiness::Refused { diagnostics } => diagnostics,
                };
                if let Some(diagnostic) = diagnostics.items.first() {
                    data.evidence_failures
                        .entry(GroupKey {
                            rule_set: admitted.set.path.clone(),
                            group: admitted.position,
                        })
                        .or_default()
                        .push(QuestionEvidenceFailure {
                            at: at.clone(),
                            input,
                            diagnostic: Some(diagnostic.code),
                        });
                }
            }
        }
    }
    for (position, settled) in state.entries.iter().enumerate() {
        let EntrySettlement::Condition {
            outcome: ConditionOutcome::Unresolved { diagnostic },
            ..
        } = settled
        else {
            continue;
        };
        let entry = &plan.entries[position];
        let Some(group) = entry.group else { continue };
        let EntryDefinition::Condition(test) = &entry.definition else {
            continue;
        };
        let input = match test {
            ConditionTest::PayloadFieldEquals { input, .. }
            | ConditionTest::PayloadFieldIn { input, .. }
            | ConditionTest::PayloadFieldPresent { input, .. }
            | ConditionTest::PatternPresent { input, .. } => input,
            ConditionTest::OriginFieldEquals { .. } => continue,
        };
        let admitted = &plan.groups[group.0].admitted;
        if admitted.sources.get(input).is_some_and(|source| {
            source.candidate_record().is_some() || source.candidate_code().is_some()
        }) {
            continue;
        }
        let code = match diagnostic {
            ReportDiagnostic::Evidence(diagnostic) => Some(diagnostic.code),
            _ if !admitted.sources.contains_key(input) => None,
            _ => continue,
        };
        let at = entry_reference(plan, group, &entry.key);
        data.evidence_failures
            .entry(GroupKey {
                rule_set: admitted.set.path.clone(),
                group: admitted.position,
            })
            .or_default()
            .push(QuestionEvidenceFailure {
                at,
                input: root_input(plan, group, input),
                diagnostic: code,
            });
    }
}

fn entry_reference(plan: &ExecutionPlan, group: GroupIndex, key: &EntryKey) -> QuestionRef {
    if let EntryOwner::Rule { rule_id, .. } = &key.owner
        && let Some(rule) = plan.groups[group.0]
            .admitted
            .set
            .rules
            .iter()
            .find(|rule| rule.definition.rule_id == *rule_id)
    {
        return crate::candidate_gates::reference(key, &rule.definition);
    }
    match key.owner {
        EntryOwner::Router { .. } => QuestionRef::Router {},
        _ => QuestionRef::RuleSetGate {
            position: key.position,
        },
    }
}

fn root_input(plan: &ExecutionPlan, group: GroupIndex, input: &SourceName) -> SourceName {
    let planned = &plan.groups[group.0];
    if let Some(route) = &planned.routing
        && let EntryDefinition::Router { definition, .. } = &plan.entries[route.entry.0].definition
        && let Some(name) = definition
            .routes
            .get(&route.label)
            .into_iter()
            .flatten()
            .find(|target| target.rule_set == planned.admitted.set.path)
            .and_then(|target| target.inputs.get(input))
    {
        return name.clone();
    }
    input.clone()
}
