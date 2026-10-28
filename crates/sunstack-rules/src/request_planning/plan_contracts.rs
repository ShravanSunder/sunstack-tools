use super::expansion_contracts::*;
use crate::{
    input_binding::{AdmittedSources, SourceId},
    invocation_admission::{AdmittedGroup, AdmittedRouter},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use sunstack_decisions::JudgmentQuestion;
use sunstack_evidence::{EvidenceDeclaration, TargetLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntryIndex(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OccurrenceIndex(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupIndex(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RequestId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum UnitKey {
    Rule(OccurrenceIndex),
    Gate(EntryIndex),
}
#[derive(Debug, Clone)]
pub(crate) struct RouteDependency {
    pub entry: EntryIndex,
    pub label: sunstack_decisions::ChoiceLabel,
}
#[derive(Debug, Clone)]
pub(crate) enum EntryDefinition {
    Condition(ConditionTest),
    Decision {
        question: JudgmentQuestion,
        evidence: Vec<EvidenceDeclaration>,
        threshold: f64,
        uncertain_below: Option<f64>,
    },
    Router {
        definition: Arc<RouterDefinition>,
        position: usize,
    },
}
impl EntryDefinition {
    pub fn is_gate(&self) -> bool {
        !matches!(self, Self::Condition(_))
    }
}
#[derive(Debug, Clone)]
pub(crate) struct PlannedEntry {
    pub key: EntryKey,
    pub definition: EntryDefinition,
    pub sources: Arc<AdmittedSources>,
    pub parents: Vec<EntryIndex>,
    pub routing: Option<RouteDependency>,
    pub group: Option<GroupIndex>,
}
#[derive(Debug)]
pub(crate) struct PlannedGroup {
    pub admitted: AdmittedGroup,
    pub entries: Vec<EntryIndex>,
    pub occurrences: Vec<OccurrenceIndex>,
    pub evaluations: Vec<EvaluationIndex>,
    pub routing: Option<RouteDependency>,
    pub read_set: BTreeSet<SourceId>,
}
#[derive(Debug)]
pub(crate) struct PlannedOccurrence {
    pub scope: ScopeEvaluation,
    pub key: RuleOccurrenceKey,
    pub rule: Arc<RuleDefinition>,
    pub group: GroupIndex,
    pub entries: Vec<EntryIndex>,
    pub authored_order: usize,
    pub target_selection: Option<crate::request_state::LabeledSelection>,
}
#[derive(Debug, Clone)]
pub(crate) enum RequestPurpose {
    Rules { group: GroupIndex },
    Gate { entry: EntryIndex },
}
#[derive(Debug, Clone)]
pub(crate) struct PlannedRequest {
    pub id: RequestId,
    pub purpose: RequestPurpose,
    pub owner: RequestOwner,
    pub mode: RequestMode,
    pub units: Vec<UnitKey>,
    pub prerequisites: Vec<EntryIndex>,
    pub routing: Option<RouteDependency>,
    pub waits_for_route: bool,
}
pub(crate) struct ExecutionPlan {
    pub groups: Vec<PlannedGroup>,
    pub occurrences: Vec<PlannedOccurrence>,
    pub evaluations: Vec<PlannedRuleEvaluation>,
    pub entries: Vec<PlannedEntry>,
    pub requests: Vec<PlannedRequest>,
    pub router_entries: Vec<EntryIndex>,
    pub controls: BTreeMap<EntryIndex, RequestId>,
    pub precomputed: BTreeMap<EntryIndex, ConditionOutcome>,
}
#[derive(Debug, Clone)]
pub(crate) struct AnswerReference {
    pub request: RequestId,
    pub question: sunstack_decisions::QuestionName,
}
#[derive(Debug, Clone)]
pub(crate) enum FailureCause {
    Request {
        request: RequestId,
    },
    Evidence {
        diagnostics: DiagnosticList,
    },
    Budget {
        diagnostics: DiagnosticList,
    },
    Gate {
        entry: EntryIndex,
    },
    Condition {
        entry: EntryIndex,
        diagnostic: ReportDiagnostic,
    },
}
#[derive(Debug, Clone)]
pub(crate) enum GateSettlement {
    Pending,
    Answered {
        reference: AnswerReference,
        answer: sunstack_decisions::JudgmentAnswer,
    },
    Failed {
        cause: FailureCause,
    },
    Cancelled,
}
#[derive(Debug, Clone)]
pub(crate) enum EntrySettlement {
    Router,
    Pending,
    Condition {
        outcome: ConditionOutcome,
        basis: ConditionBasis,
    },
    Decision {
        threshold: f64,
        result: DecisionSettlement,
    },
    NotEvaluated {
        reason: NotEvaluatedReason,
    },
}
#[derive(Debug, Clone)]
pub(crate) enum DecisionSettlement {
    Passed { reference: AnswerReference },
    DidNotPass { reference: AnswerReference },
    Uncertain { reference: AnswerReference },
    Failed { cause: FailureCause },
    Cancelled,
}
#[derive(Debug, Clone)]
pub(crate) enum RouterSettlement {
    Pending,
    Answered {
        answer: sunstack_decisions::JudgmentAnswer,
        reference: AnswerReference,
    },
    Failed {
        cause: FailureCause,
    },
    Cancelled,
}
#[derive(Debug, Clone)]
pub(crate) enum OwnerPreparation {
    OutsideChangeScope,
    Cancelled {
        selections: Vec<crate::request_state::LabeledSelection>,
    },
    Ready {
        selections: Vec<crate::request_state::LabeledSelection>,
        target: OwnerTarget,
    },
    Refused {
        selections: Vec<crate::request_state::LabeledSelection>,
        diagnostics: DiagnosticList,
    },
}
#[derive(Debug, Clone)]
pub(crate) enum OwnerTarget {
    Rule(TargetLocation),
    Gate,
}
pub(crate) enum RequestSettlement {
    Waiting,
    InFlight,
    Settled {
        status: RequestStatus,
        bindings: Vec<QuestionBinding>,
        unit_bindings: Vec<(UnitKey, sunstack_decisions::QuestionName)>,
    },
}
pub(crate) struct ExecutionState {
    pub request_evidence: BTreeMap<RequestId, sunstack_decisions::SharedEvidenceState>,
    pub local_settlements: BTreeMap<OccurrenceIndex, LocalSettlement>,
    pub expansions: Vec<ExpansionState>,
    pub entries: Vec<EntrySettlement>,
    pub gates: BTreeMap<EntryIndex, GateSettlement>,
    pub routers: BTreeMap<EntryIndex, RouterSettlement>,
    pub requests: BTreeMap<RequestId, RequestSettlement>,
    pub preparation: BTreeMap<UnitKey, OwnerPreparation>,
    pub budget_refusals: BTreeMap<UnitKey, DiagnosticList>,
    pub scopes: BTreeMap<OccurrenceIndex, ScopeEvaluation>,
}
pub(crate) fn selection_read_set(
    declarations: &[EvidenceDeclaration],
    sources: &AdmittedSources,
) -> BTreeSet<SourceId> {
    declarations
        .iter()
        .filter_map(|declaration| {
            sources
                .get(&declaration.input)
                .map(|source| source.id.clone())
        })
        .collect()
}
impl ExecutionPlan {
    pub fn unit_is_mechanical(&self, unit: UnitKey) -> bool {
        matches!(unit, UnitKey::Rule(index) if matches!(self.occurrences[index.0].rule.verdict, VerdictDeclaration::Mechanical { .. }))
    }
    pub fn entry_evidence(&self, index: EntryIndex) -> &[EvidenceDeclaration] {
        match &self.entries[index.0].definition {
            EntryDefinition::Decision { evidence, .. } => evidence,
            EntryDefinition::Router { definition, .. } => &definition.evidence,
            EntryDefinition::Condition(_) => &[],
        }
    }
    pub fn entry_read_set(&self, index: EntryIndex) -> BTreeSet<SourceId> {
        selection_read_set(self.entry_evidence(index), &self.entries[index.0].sources)
    }
    pub fn unit_label(&self, unit: UnitKey) -> String {
        match unit {
            UnitKey::Rule(index) => self.occurrences[index.0].key.evidence_label(),
            UnitKey::Gate(index) => match &self.entries[index.0].key.owner {
                EntryOwner::Router { .. } => "router".into(),
                EntryOwner::RuleSet { .. } => {
                    format!("rule_set_decision:{}", self.entries[index.0].key.position)
                }
                EntryOwner::Rule { rule_id, .. } => format!(
                    "rule:{}:decision:{}",
                    rule_id.as_str(),
                    self.entries[index.0].key.position
                ),
            },
        }
    }
}
// Input admission has already fixed paths, schemas and source identities. The planner reads data only.
pub(crate) fn definition_topology(
    groups: Vec<AdmittedGroup>,
    router: Option<AdmittedRouter>,
) -> ExecutionPlan {
    definition_topology_with_policy(groups, router, &RulePolicy::default())
}
pub(crate) fn definition_topology_with_policy(
    groups: Vec<AdmittedGroup>,
    router: Option<AdmittedRouter>,
    policy: &RulePolicy,
) -> ExecutionPlan {
    let mut plan = ExecutionPlan {
        groups: vec![],
        occurrences: vec![],
        evaluations: vec![],
        entries: vec![],
        requests: vec![],
        router_entries: vec![],
        controls: BTreeMap::new(),
        precomputed: BTreeMap::new(),
    };
    if let Some(router) = router {
        let definition = Arc::new(router.definition);
        for (position, sources) in router.groups.into_iter().enumerate() {
            let index = EntryIndex(plan.entries.len());
            plan.router_entries.push(index);
            plan.entries.push(PlannedEntry {
                key: EntryKey {
                    owner: EntryOwner::Router { group: position },
                    position: 0,
                },
                definition: EntryDefinition::Router {
                    definition: definition.clone(),
                    position,
                },
                sources: Arc::new(sources),
                parents: vec![],
                routing: None,
                group: None,
            });
        }
    }
    for admitted in groups {
        let group_index = GroupIndex(plan.groups.len());
        let sources = Arc::new(admitted.sources.clone());
        let routing = match &admitted.binding {
            GroupBinding::Routed {
                router_group,
                label,
            } => plan
                .router_entries
                .get(*router_group)
                .copied()
                .map(|entry| RouteDependency {
                    entry,
                    label: label.clone(),
                }),
            _ => None,
        };
        let owner = EntryOwner::RuleSet {
            path: admitted.set.path.clone(),
            group: admitted.position,
            route: admitted.binding.route().cloned(),
        };
        let enabled_rules = admitted
            .set
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                !policy.is_off(
                    &admitted.set.path,
                    admitted.position,
                    &rule.definition.rule_id,
                )
            })
            .collect::<Vec<_>>();
        let group_entries = if enabled_rules.is_empty() && !admitted.set.rules.is_empty() {
            vec![]
        } else {
            append_entries(
                &mut plan,
                &admitted.set.definition.applicability,
                owner,
                sources.clone(),
                vec![],
                routing.clone(),
                group_index,
            )
        };
        let mut occurrence_indices = Vec::new();
        let mut evaluation_indices = Vec::new();
        let mut read_set = BTreeSet::new();
        for (authored_order, located) in enabled_rules {
            let rule = &located.definition;
            let occurrence_index = OccurrenceIndex(plan.occurrences.len());
            let owner = EntryOwner::Rule {
                path: admitted.set.path.clone(),
                group: admitted.position,
                route: admitted.binding.route().cloned(),
                rule_id: rule.rule_id.clone(),
            };
            let candidate_entries;
            let applicability = if sources.values().any(|source| source.candidate.is_some()) {
                candidate_entries = crate::candidate_gates::rule_entries(rule);
                &candidate_entries
            } else {
                &rule.applicability
            };
            let entries = append_entries(
                &mut plan,
                applicability,
                owner,
                sources.clone(),
                group_entries.clone(),
                routing.clone(),
                group_index,
            );
            let mut all_entries = group_entries.clone();
            all_entries.extend(entries);
            read_set.extend(selection_read_set(&rule.evidence, &sources));
            if rule.evidence.iter().any(|declaration| {
                declaration.id == rule.target
                    && matches!(
                        declaration.select,
                        sunstack_evidence::EvidenceSelection::MarkdownUnits(_)
                            | sunstack_evidence::EvidenceSelection::FrontmatterField { .. }
                    )
            }) {
                let index = EvaluationIndex(plan.evaluations.len());
                evaluation_indices.push(index);
                plan.evaluations.push(PlannedRuleEvaluation {
                    key: RuleEvaluationKey {
                        rule_set: admitted.set.path.clone(),
                        group: admitted.position,
                        rule_id: rule.rule_id.clone(),
                    },
                    rule: rule.clone(),
                    group: group_index,
                    entries: all_entries,
                    scope: ScopeEvaluation::full_scope(),
                    authored_order,
                });
                continue;
            }
            occurrence_indices.push(occurrence_index);
            plan.occurrences.push(PlannedOccurrence {
                scope: ScopeEvaluation::full_scope(),
                key: RuleOccurrenceKey {
                    rule_set: admitted.set.path.clone(),
                    group: admitted.position,
                    rule_id: rule.rule_id.clone(),
                    target: Box::new(RuleOccurrenceTarget::Legacy),
                },
                rule: rule.clone(),
                group: group_index,
                entries: all_entries,
                authored_order,
                target_selection: None,
            });
        }
        plan.groups.push(PlannedGroup {
            admitted,
            entries: group_entries,
            occurrences: occurrence_indices,
            evaluations: evaluation_indices,
            routing,
            read_set,
        });
    }
    plan
}
fn append_entries(
    plan: &mut ExecutionPlan,
    entries: &[ApplicabilityEntry],
    owner: EntryOwner,
    sources: Arc<AdmittedSources>,
    mut parents: Vec<EntryIndex>,
    routing: Option<RouteDependency>,
    group: GroupIndex,
) -> Vec<EntryIndex> {
    let mut indices = Vec::new();
    for (position, entry) in entries.iter().enumerate() {
        let index = EntryIndex(plan.entries.len());
        indices.push(index);
        let definition = match entry {
            ApplicabilityEntry::Condition { test } => EntryDefinition::Condition(test.clone()),
            ApplicabilityEntry::Decision {
                question,
                evidence,
                threshold,
                uncertain_below,
            } => EntryDefinition::Decision {
                question: question.clone(),
                evidence: evidence.clone(),
                threshold: *threshold,
                uncertain_below: if sources.values().any(|source| source.candidate.is_some()) {
                    *uncertain_below
                } else {
                    None
                },
            },
        };
        plan.entries.push(PlannedEntry {
            key: EntryKey {
                owner: owner.clone(),
                position,
            },
            definition,
            sources: sources.clone(),
            parents: parents.clone(),
            routing: routing.clone(),
            group: Some(group),
        });
        parents.push(index);
    }
    indices
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TargetPresence {
    ResolveCurrent,
    Deleted,
}
#[derive(Debug, Clone)]
pub(crate) enum ScopeDisposition {
    InScope { selection: OccurrenceSelection },
    OutsideChangeScope,
}
#[derive(Debug, Clone)]
pub(crate) struct ScopeEvaluation {
    pub disposition: ScopeDisposition,
    pub target: TargetPresence,
}
impl ScopeEvaluation {
    pub fn full_scope() -> Self {
        Self {
            disposition: ScopeDisposition::InScope {
                selection: OccurrenceSelection::full_scope(),
            },
            target: TargetPresence::ResolveCurrent,
        }
    }
    pub fn pending() -> Self {
        Self {
            disposition: ScopeDisposition::InScope {
                selection: OccurrenceSelection::ComparisonPending {},
            },
            target: TargetPresence::ResolveCurrent,
        }
    }
}
