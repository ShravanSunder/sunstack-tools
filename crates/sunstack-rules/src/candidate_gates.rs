use crate::*;

pub(crate) fn rule_entries(rule: &RuleDefinition) -> Vec<ApplicabilityEntry> {
    rule.narrowing
        .iter()
        .map(NarrowingDecision::entry)
        .chain(rule.applicability.iter().cloned())
        .collect()
}

pub(crate) fn reference(key: &EntryKey, rule: &RuleDefinition) -> QuestionRef {
    match key.owner {
        EntryOwner::Router { .. } => QuestionRef::Router {},
        EntryOwner::RuleSet { .. } => QuestionRef::RuleSetGate {
            position: key.position,
        },
        EntryOwner::Rule { .. } if rule.narrowing.is_some() && key.position == 0 => {
            QuestionRef::Narrowing {}
        }
        EntryOwner::Rule { .. } => QuestionRef::RuleGate {
            position: key
                .position
                .saturating_sub(usize::from(rule.narrowing.is_some())),
        },
    }
}
