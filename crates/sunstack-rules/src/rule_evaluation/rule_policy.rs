use crate::{
    DiagnosticList, RuleDiagnostic, RuleDiagnosticCode, RuleId, RuleInvocationOutcome,
    rule_definitions::input_binding::resolved_path,
    rule_definitions::invocation_admission::AdmittedGroup,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use sunstack_decisions::SharedEvidenceState;

/// A binding group's native identity; paths follow invocation definition resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupKey {
    pub rule_set: PathBuf,
    pub group: usize,
}

/// Native exclusions applied before registering rules or their applicability work.
#[derive(Debug, Clone, Default)]
pub struct RulePolicy {
    pub off: BTreeMap<GroupKey, BTreeSet<RuleId>>,
}
impl RulePolicy {
    pub(crate) fn resolve(
        self,
        root: &std::path::Path,
        groups: &[AdmittedGroup],
    ) -> Result<Self, DiagnosticList> {
        let cwd = std::env::current_dir()
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
        let root = resolved_path(&cwd, root)?;
        let mut resolved = Self::default();
        for (key, rules) in self.off {
            let key = GroupKey {
                rule_set: resolved_path(&root, &key.rule_set)?,
                group: key.group,
            };
            let invalid = || {
                RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration).definition(
                    &key.rule_set,
                    1,
                    1,
                    vec!["binding_groups".into(), key.group.to_string()],
                )
            };
            let matching = groups
                .iter()
                .filter(|group| group.set.path == key.rule_set && group.position == key.group)
                .collect::<Vec<_>>();
            if matching.is_empty()
                || rules.iter().any(|rule_id| {
                    matching.iter().any(|group| {
                        !group
                            .set
                            .rules
                            .iter()
                            .any(|rule| &rule.definition.rule_id == rule_id)
                    })
                })
            {
                let mut diagnostic = invalid();
                diagnostic.guidance =
                    "Name a loaded rule and binding group in this rule set's native policy.".into();
                return Err(diagnostic.into());
            }
            resolved.off.entry(key).or_default().extend(rules);
        }
        Ok(resolved)
    }
    pub(crate) fn is_off(&self, path: &std::path::Path, group: usize, rule: &RuleId) -> bool {
        self.off
            .get(&GroupKey {
                rule_set: path.into(),
                group,
            })
            .is_some_and(|rules| rules.contains(rule))
    }
}

#[derive(Debug)]
pub struct PolicyEvaluation {
    pub outcome: RuleInvocationOutcome,
    pub request_evidence: BTreeMap<usize, SharedEvidenceState>,
}
