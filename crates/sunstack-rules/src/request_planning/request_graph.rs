use super::plan_contracts::*;
use crate::{
    applicability_evaluation::{ConditionContext, evaluate_condition},
    *,
};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::EvidenceSession;
use tokio_util::sync::CancellationToken;

pub(crate) fn condition_is_leading(plan: &ExecutionPlan, index: EntryIndex) -> bool {
    let entry = &plan.entries[index.0];
    !entry.parents.iter().any(|parent| {
        let parent = &plan.entries[parent.0];
        parent.key.owner == entry.key.owner
            && matches!(parent.definition, EntryDefinition::Decision { .. })
    })
}
fn scan_waits_for_routing(
    plan: &ExecutionPlan,
    index: EntryIndex,
    speculation: Speculation,
) -> bool {
    let entry = &plan.entries[index.0];
    let (Some(routing), EntryDefinition::Condition(ConditionTest::PatternPresent { input, .. })) =
        (&entry.routing, &entry.definition)
    else {
        return false;
    };
    let Some(group) = entry.group else {
        return false;
    };
    let read_set = &plan.groups[group.0].read_set;
    (speculation == Speculation::Off && !plan.entry_read_set(routing.entry).is_subset(read_set))
        || entry
            .sources
            .get(input)
            .is_some_and(|source| !read_set.contains(&source.id))
}
pub(crate) fn group_waits_for_leading_scan(plan: &ExecutionPlan, group: GroupIndex) -> bool {
    let group = &plan.groups[group.0];
    group
        .entries
        .iter()
        .copied()
        .chain(
            group
                .occurrences
                .iter()
                .flat_map(|index| plan.occurrences[index.0].entries.iter().copied()),
        )
        .any(|index| {
            condition_is_leading(plan, index)
                && scan_waits_for_routing(plan, index, Speculation::On)
        })
}
pub(crate) fn entry_outside_scope(plan: &ExecutionPlan, index: EntryIndex) -> bool {
    plan.entries[index.0].group.is_some()
        && !plan.occurrences.iter().any(|occurrence| {
            occurrence.entries.contains(&index)
                && !matches!(
                    occurrence.scope.disposition,
                    ScopeDisposition::OutsideChangeScope
                )
        })
}
pub(crate) async fn precompute_conditions(
    plan: &mut ExecutionPlan,
    origin: &InvocationOrigin,
    session: &mut EvidenceSession,
    cancellation: &CancellationToken,
    speculation: Speculation,
) {
    for position in 0..plan.entries.len() {
        if cancellation.is_cancelled() {
            break;
        }
        if entry_outside_scope(plan, EntryIndex(position)) {
            continue;
        }
        let entry = &plan.entries[position];
        let EntryDefinition::Condition(test) = &entry.definition else {
            continue;
        };
        if !condition_is_leading(plan, EntryIndex(position)) {
            continue;
        }
        let scans = matches!(test, ConditionTest::PatternPresent { .. });
        if scans
            && (scan_waits_for_routing(plan, EntryIndex(position), speculation)
                || entry
                    .parents
                    .iter()
                    .any(|parent| plan.entries[parent.0].definition.is_gate())
                || entry.parents.iter().any(|parent| {
                    plan.precomputed
                        .get(parent)
                        .is_some_and(|outcome| !matches!(outcome, ConditionOutcome::Passed))
                }))
        {
            continue;
        }
        let outcome = evaluate_condition(
            test,
            ConditionContext {
                sources: &entry.sources,
                origin,
                session,
            },
        )
        .await;
        plan.precomputed.insert(EntryIndex(position), outcome);
    }
}
pub(crate) fn rule_pruned(plan: &ExecutionPlan, index: OccurrenceIndex) -> bool {
    matches!(
        plan.occurrences[index.0].scope.disposition,
        ScopeDisposition::OutsideChangeScope
    ) || plan.occurrences[index.0].entries.iter().any(|entry| {
        plan.precomputed
            .get(entry)
            .is_some_and(|outcome| !matches!(outcome, ConditionOutcome::Passed))
    })
}
fn push_request(plan: &mut ExecutionPlan, mut request: PlannedRequest) -> RequestId {
    let id = RequestId(plan.requests.len());
    request.id = id;
    plan.requests.push(request);
    id
}
/// Gate ownership is fixed once. Source identity determines carrying; only different-input gates stage.
pub(crate) fn build_request_graph(plan: &mut ExecutionPlan, settings: &ExecutionSettings) {
    let mut group_requests = BTreeMap::new();
    for group_position in 0..plan.groups.len() {
        let group = GroupIndex(group_position);
        let rules = plan.groups[group_position]
            .occurrences
            .iter()
            .copied()
            .filter(|index| !rule_pruned(plan, *index))
            .collect::<Vec<_>>();
        let mut read_set = BTreeSet::new();
        for rule in &rules {
            read_set.extend(selection_read_set(
                &plan.occurrences[rule.0].rule.evidence,
                &plan.groups[group_position].admitted.sources,
            ));
        }
        plan.groups[group_position].read_set = read_set;
        if rules.is_empty() {
            continue;
        }
        let admitted = &plan.groups[group_position].admitted;
        let id = push_request(
            plan,
            PlannedRequest {
                id: RequestId(0),
                purpose: RequestPurpose::Rules { group },
                owner: RequestOwner::RuleSetGroup {
                    rule_set: admitted.set.path.clone(),
                    group: admitted.position,
                    route: admitted.binding.route().cloned(),
                },
                mode: RequestMode::Rules,
                units: rules.into_iter().map(UnitKey::Rule).collect(),
                prerequisites: vec![],
                routing: plan.groups[group_position].routing.clone(),
                waits_for_route: false,
            },
        );
        group_requests.insert(group, id);
    }
    let mut staged = BTreeSet::new();
    if settings.speculation == Speculation::Off {
        for (position, entry) in plan.entries.iter().enumerate() {
            let EntryDefinition::Decision { .. } = &entry.definition else {
                continue;
            };
            let Some(group) = entry.group else { continue };
            let index = EntryIndex(position);
            if !plan
                .entry_read_set(index)
                .is_subset(&plan.groups[group.0].read_set)
            {
                staged.insert(index);
                for parent in &entry.parents {
                    if plan.entries[parent.0].definition.is_gate() {
                        staged.insert(*parent);
                    }
                }
            }
        }
    }
    // A carried ancestor of a staged gate would deadlock the rules request waiting on that gate.
    let mut changed = true;
    while changed {
        changed = false;
        for index in staged.clone() {
            for parent in &plan.entries[index.0].parents {
                if plan.entries[parent.0].definition.is_gate() && staged.insert(*parent) {
                    changed = true
                }
            }
        }
    }
    for (group, id) in &group_requests {
        let mut prerequisites = staged
            .iter()
            .copied()
            .filter(|entry| plan.entries[entry.0].group == Some(*group))
            .collect::<Vec<_>>();
        prerequisites.sort();
        plan.requests[id.0].prerequisites = prerequisites;
    }
    for position in 0..plan.entries.len() {
        let index = EntryIndex(position);
        let entry = &plan.entries[position];
        if !matches!(entry.definition, EntryDefinition::Decision { .. }) {
            continue;
        }
        let Some(group) = entry.group else { continue };
        if entry_outside_scope(plan, index) {
            continue;
        }
        let carrier = group_requests.get(&group).copied().filter(|_| {
            !staged.contains(&index)
                && plan
                    .entry_read_set(index)
                    .is_subset(&plan.groups[group.0].read_set)
        });
        if let Some(id) = carrier {
            plan.requests[id.0].units.insert(0, UnitKey::Gate(index));
            plan.requests[id.0].mode = RequestMode::Speculative;
            plan.controls.insert(index, id);
        } else {
            let admitted = &plan.groups[group.0].admitted;
            let request = PlannedRequest {
                id: RequestId(0),
                purpose: RequestPurpose::Gate { entry: index },
                owner: RequestOwner::RuleSetGroup {
                    rule_set: admitted.set.path.clone(),
                    group: admitted.position,
                    route: admitted.binding.route().cloned(),
                },
                mode: if settings.speculation == Speculation::On {
                    RequestMode::Speculative
                } else {
                    RequestMode::StagedGate
                },
                units: vec![UnitKey::Gate(index)],
                prerequisites: if settings.speculation == Speculation::On {
                    vec![]
                } else {
                    entry.parents.clone()
                },
                routing: entry.routing.clone(),
                waits_for_route: false,
            };
            let id = push_request(plan, request);
            plan.controls.insert(index, id);
        }
    }
    let mut router_carriers = BTreeMap::new();
    for index in plan.router_entries.clone() {
        let read_set = plan.entry_read_set(index);
        let carrier = group_requests
            .iter()
            .filter(|(group, id)| {
                plan.groups[group.0]
                    .routing
                    .as_ref()
                    .is_some_and(|routing| routing.entry == index)
                    && plan.requests[id.0].prerequisites.is_empty()
                    && !group_waits_for_leading_scan(plan, **group)
                    && read_set.is_subset(&plan.groups[group.0].read_set)
            })
            .map(|(_, id)| *id)
            .next();
        let id = if let Some(id) = carrier {
            plan.requests[id.0].units.insert(0, UnitKey::Gate(index));
            plan.requests[id.0].mode = RequestMode::Speculative;
            router_carriers.insert(index, Some(id));
            id
        } else {
            router_carriers.insert(index, None);
            let position = match plan.entries[index.0].key.owner {
                EntryOwner::Router { group } => group,
                _ => 0,
            };
            push_request(
                plan,
                PlannedRequest {
                    id: RequestId(0),
                    purpose: RequestPurpose::Gate { entry: index },
                    owner: RequestOwner::Router { group: position },
                    mode: if settings.speculation == Speculation::On {
                        RequestMode::Speculative
                    } else {
                        RequestMode::StagedGate
                    },
                    units: vec![UnitKey::Gate(index)],
                    prerequisites: vec![],
                    routing: None,
                    waits_for_route: false,
                },
            )
        };
        plan.controls.insert(index, id);
    }
    // Sort gate units by authored entry order, then rules by authored occurrence order.
    for request in &mut plan.requests {
        request.units.sort_by_key(|unit| match unit {
            UnitKey::Gate(index) => (0, index.0),
            UnitKey::Rule(index) => (1, index.0),
        });
    }
    for position in 0..plan.requests.len() {
        let routing = plan.requests[position].routing.clone();
        let Some(routing) = routing else { continue };
        let group = match plan.requests[position].purpose {
            RequestPurpose::Rules { group } => Some(group),
            RequestPurpose::Gate { entry } => plan.entries[entry.0].group,
        };
        let Some(group) = group else { continue };
        let carried = router_carriers.get(&routing.entry).copied().flatten();
        let can_fan_out = if group_waits_for_leading_scan(plan, group) {
            false
        } else if settings.speculation == Speculation::On {
            true
        } else if carried.is_some() {
            plan.entry_read_set(routing.entry)
                .is_subset(&plan.groups[group.0].read_set)
                && group_requests
                    .get(&group)
                    .is_some_and(|id| plan.requests[id.0].prerequisites.is_empty())
        } else {
            false
        };
        plan.requests[position].waits_for_route = !can_fan_out;
        if can_fan_out {
            plan.requests[position].mode = RequestMode::Speculative;
        }
    }
}
impl ExecutionState {
    pub fn new(plan: &ExecutionPlan) -> Self {
        let mut gates = BTreeMap::new();
        let mut routers = BTreeMap::new();
        for (position, entry) in plan.entries.iter().enumerate() {
            match entry.definition {
                EntryDefinition::Decision { .. } => {
                    gates.insert(EntryIndex(position), GateSettlement::Pending);
                }
                EntryDefinition::Router { .. } => {
                    routers.insert(EntryIndex(position), RouterSettlement::Pending);
                }
                EntryDefinition::Condition(_) => {}
            }
        }
        Self {
            entries: plan
                .entries
                .iter()
                .map(|entry| {
                    if matches!(entry.definition, EntryDefinition::Router { .. }) {
                        EntrySettlement::Router
                    } else {
                        EntrySettlement::Pending
                    }
                })
                .collect(),
            gates,
            routers,
            requests: plan
                .requests
                .iter()
                .map(|request| (request.id, RequestSettlement::Waiting))
                .collect(),
            preparation: BTreeMap::new(),
            budget_refusals: BTreeMap::new(),
            scopes: BTreeMap::new(),
        }
    }
}
