use super::*;
use std::{collections::BTreeMap, time::Instant};

enum CandidateClock {
    Running(Instant),
    Settled(u64),
}

/// Scheduler-owned clocks stop at logical settlement, independently of other groups' work.
#[derive(Default)]
pub(crate) struct CandidateTimings(BTreeMap<GroupIndex, CandidateClock>);
impl CandidateTimings {
    pub fn sent(&mut self, group: GroupIndex, now: Instant) {
        self.0.entry(group).or_insert(CandidateClock::Running(now));
    }
    pub fn settled(&mut self, group: GroupIndex, now: Instant) {
        if let Some(clock @ CandidateClock::Running(_)) = self.0.get_mut(&group) {
            let CandidateClock::Running(started) = clock else {
                return;
            };
            let elapsed = u64::try_from(now.saturating_duration_since(*started).as_millis())
                .unwrap_or(u64::MAX);
            *clock = CandidateClock::Settled(elapsed);
        }
    }
    pub fn request_sent(&mut self, plan: &ExecutionPlan, units: &[UnitKey], now: Instant) {
        for unit in units {
            let groups = match unit {
                UnitKey::Rule(index) => vec![plan.occurrences[index.0].group],
                UnitKey::Gate(index) => match plan.entries[index.0].group {
                    Some(group) => vec![group],
                    None => plan
                        .groups
                        .iter()
                        .enumerate()
                        .filter_map(|(position, group)| {
                            group
                                .routing
                                .as_ref()
                                .filter(|route| route.entry == *index)
                                .map(|_| GroupIndex(position))
                        })
                        .collect(),
                },
            };
            for group in groups {
                if plan.groups[group.0]
                    .admitted
                    .sources
                    .values()
                    .any(|source| source.candidate.is_some())
                {
                    self.sent(group, now);
                }
            }
        }
    }
    pub fn settle_ready(&mut self, plan: &ExecutionPlan, state: &ExecutionState, now: Instant) {
        if self.0.is_empty() {
            return;
        }
        for position in 0..plan.groups.len() {
            let group = GroupIndex(position);
            if group_settled(plan, state, group) {
                self.settled(group, now);
            }
        }
    }
    pub fn measurements(&self, plan: &ExecutionPlan) -> BTreeMap<crate::GroupKey, u64> {
        self.0
            .iter()
            .filter_map(|(index, clock)| match clock {
                CandidateClock::Running(_) => None,
                CandidateClock::Settled(elapsed) => {
                    let group = &plan.groups[index.0].admitted;
                    Some((
                        crate::GroupKey {
                            rule_set: group.set.path.clone(),
                            group: group.position,
                        },
                        *elapsed,
                    ))
                }
            })
            .fold(BTreeMap::new(), |mut timings, (key, elapsed)| {
                timings
                    .entry(key)
                    .and_modify(|current: &mut u64| *current = (*current).max(elapsed))
                    .or_insert(elapsed);
                timings
            })
    }
}

fn group_settled(plan: &ExecutionPlan, state: &ExecutionState, index: GroupIndex) -> bool {
    let group = &plan.groups[index.0];
    if let Some(route) = &group.routing {
        match route_state(plan, state, route) {
            RouteState::Pending => return false,
            RouteState::NotSelected | RouteState::Failed { .. } | RouteState::Cancelled => {
                return true;
            }
            RouteState::Selected => {}
        }
    }
    match prefix_state(&group.entries, state) {
        PrefixState::Pending => return false,
        PrefixState::Passed => {}
        PrefixState::Stopped { .. }
        | PrefixState::Uncertain { .. }
        | PrefixState::Failed { .. }
        | PrefixState::Cancelled { .. } => return true,
    }
    if group
        .evaluations
        .iter()
        .any(|index| matches!(state.expansions[index.0], ExpansionState::Waiting))
    {
        return false;
    }
    group.occurrences.iter().all(|index| {
        match prefix_state(&plan.occurrences[index.0].entries, state) {
            PrefixState::Pending => return false,
            PrefixState::Passed => {}
            PrefixState::Stopped { .. }
            | PrefixState::Uncertain { .. }
            | PrefixState::Failed { .. }
            | PrefixState::Cancelled { .. } => return true,
        }
        let unit = UnitKey::Rule(*index);
        if state.budget_refusals.contains_key(&unit)
            || state.local_settlements.contains_key(index)
            || matches!(
                state.preparation.get(&unit),
                Some(
                    OwnerPreparation::Refused { .. }
                        | OwnerPreparation::Cancelled { .. }
                        | OwnerPreparation::OutsideChangeScope
                )
            )
        {
            return true;
        }
        let mut requests = plan
            .requests
            .iter()
            .filter(|request| request.units.contains(&unit))
            .peekable();
        requests.peek().is_some()
            && requests.all(|request| {
                matches!(
                    state.requests.get(&request.id),
                    Some(RequestSettlement::Settled { .. })
                )
            })
    })
}

#[cfg(test)]
#[path = "candidate_timing_tests.rs"]
mod tests;
