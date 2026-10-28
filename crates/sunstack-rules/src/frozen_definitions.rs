use crate::*;
use std::{
    collections::BTreeMap,
    path::{Component, PathBuf},
};

/// Assemble the named rule exclusively from caller-verified, root-relative definition bytes.
pub fn frozen_rule_pass_means(
    rule: &RuleRef,
    files: BTreeMap<PathBuf, Vec<u8>>,
) -> Result<PassMeans, DiagnosticList> {
    let relative = |path: &std::path::Path| {
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    };
    if !relative(&rule.rule_set) || files.keys().any(|path| !relative(path)) {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    let missing = || {
        RuleDiagnostic::new(RuleDiagnosticCode::MissingRuleEntry)
            .definition(
                &rule.rule_set,
                1,
                1,
                vec!["rule_id".into(), rule.rule_id.as_str().into()],
            )
            .into()
    };
    if !files.contains_key(&rule.rule_set) {
        return Err(missing());
    }
    let mut loader = crate::definition_loading::DefinitionLoader::from_frozen(files);
    let set = loader.load_set(rule.rule_set.clone())?;
    if !set.skipped.is_empty() {
        let mut diagnostics = DiagnosticList::default();
        for skipped in &set.skipped {
            diagnostics.extend(skipped.diagnostics().clone());
        }
        return Err(diagnostics);
    }
    set.rules
        .iter()
        .find(|located| located.definition.rule_id == rule.rule_id)
        .map(|located| located.definition.verdict.pass_means())
        .ok_or_else(missing)
}
