use crate::{
    InvocationMeasurements, RuleCoverage, SkippedRule, invocation_admission::LoadedRuleSet,
};
use std::collections::BTreeSet;

pub(crate) struct RuleSourceSummary {
    pub coverage: RuleCoverage,
    pub skipped: Vec<SkippedRule>,
}
pub(crate) fn source_summary<'sets>(
    sets: impl IntoIterator<Item = &'sets LoadedRuleSet>,
    measurements: &mut InvocationMeasurements,
) -> RuleSourceSummary {
    let mut seen = BTreeSet::new();
    let mut valid = 0;
    let mut skipped = Vec::new();
    for set in sets {
        if seen.insert(&set.path) {
            valid += set.rules.len();
            skipped.extend(set.skipped.iter().cloned());
        }
    }
    measurements.invalid_rule_entries = skipped
        .iter()
        .filter(|item| matches!(item, SkippedRule::Entry { .. }))
        .count();
    measurements.skipped_rule_sources = skipped
        .iter()
        .filter(|item| matches!(item, SkippedRule::Source { .. }))
        .count();
    let coverage = if valid == 0 {
        RuleCoverage::None
    } else if skipped.is_empty() {
        RuleCoverage::Full
    } else {
        RuleCoverage::Partial
    };
    RuleSourceSummary { coverage, skipped }
}
