use crate::*;
use std::{
    collections::BTreeMap,
    path::{Component, PathBuf},
};

/// Assemble the named rule exclusively from caller-verified, root-relative definition bytes.
pub fn admit_frozen_entry(
    rule: &RuleRef,
    files: BTreeMap<PathBuf, Vec<u8>>,
    entry: &CandidateDefinitions,
    payload_schemas: &[sunstack_evidence::PayloadSchemaPath],
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
    let entry_paths = match entry {
        CandidateDefinitions::Router { path } => std::slice::from_ref(path),
        CandidateDefinitions::RuleSets { rule_sets } => rule_sets.as_slice(),
    };
    for path in entry_paths {
        if !relative(path) || !files.contains_key(path) {
            return Err(RuleDiagnostic::new(RuleDiagnosticCode::MissingRuleEntry)
                .definition(path, 1, 1, vec!["entry".into()])
                .into());
        }
    }
    let loader = crate::definition_loading::DefinitionLoader::from_frozen(files);
    let sets = crate::definition_checking::load_declared_definitions(
        entry,
        payload_schemas,
        std::path::Path::new(""),
        loader,
    )?;
    let set = sets
        .iter()
        .find(|set| set.path == rule.rule_set)
        .ok_or_else(missing)?;
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
