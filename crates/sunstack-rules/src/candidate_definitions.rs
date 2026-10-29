use crate::{
    candidate_inputs::invalid_candidate_field, input_binding::resolved_path,
    invocation_admission::LoadedRuleSet, *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use sunstack_evidence::SourceName;

pub(crate) struct CandidateDefinitionsLoaded {
    pub sets: Vec<Arc<LoadedRuleSet>>,
    pub router: Option<CandidateRouter>,
}
pub(crate) struct CandidateRouter {
    pub path: PathBuf,
    pub definition: RouterDefinition,
    pub targets: BTreeMap<(PathBuf, RuleId), SourceName>,
}
pub(crate) async fn load_definitions(
    input: &CandidateRunInput,
    context: &RuleExecutionContext,
) -> Result<CandidateDefinitionsLoaded, DiagnosticList> {
    let cwd = std::env::current_dir().map_err(|_| invalid_candidate_field(vec![]))?;
    let root = resolved_path(&cwd, &input.definition_root)?;
    let definitions = input.definitions.clone();
    let max_bytes = input.limits.max_file_bytes.get();
    let cancellation = context.cancellation.clone();
    let loaded = tokio::task::spawn_blocking(move || {
        let mut loader = crate::definition_loading::DefinitionLoader::new(max_bytes, cancellation);
        match definitions {
            CandidateDefinitions::RuleSets { rule_sets } => {
                if rule_sets.is_empty() {
                    return Err(invalid_candidate_field(vec!["definitions".into()]));
                }
                let mut seen = BTreeSet::new();
                let sets = rule_sets
                    .into_iter()
                    .map(|path| {
                        let path = resolved_path(&root, &path)?;
                        if !seen.insert(path.clone()) {
                            return Err(DiagnosticList::rule(RuleDiagnosticCode::DuplicateRuleSet));
                        }
                        loader.load_set(path)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CandidateDefinitionsLoaded { sets, router: None })
            }
            CandidateDefinitions::Router { path } => {
                let path = resolved_path(&root, &path)?;
                let mut definition = decode_router(&loader.read(&path)?, &path)?;
                let directory = path
                    .parent()
                    .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
                let mut sets = BTreeMap::new();
                let mut targets = BTreeMap::new();
                for route in definition.routes.values_mut() {
                    let mut seen = BTreeSet::new();
                    for target in route {
                        target.rule_set = resolved_path(directory, &target.rule_set)?;
                        if !seen.insert(target.rule_set.clone()) {
                            return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidRoute));
                        }
                        let set = loader.load_set(target.rule_set.clone())?;
                        crate::definition_routing::validate_route_mapping(
                            target,
                            &set.definition.inputs,
                            &definition.inputs,
                        )?;
                        for rule in &set.rules {
                            let subject = rule
                                .definition
                                .evidence
                                .iter()
                                .find(|evidence| evidence.id == rule.definition.target)
                                .ok_or_else(|| {
                                    DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget)
                                })?;
                            let name = target.inputs.get(&subject.input).ok_or_else(|| {
                                DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring)
                            })?;
                            let key = (set.path.clone(), rule.definition.rule_id.clone());
                            if targets
                                .insert(key, name.clone())
                                .is_some_and(|previous| previous != *name)
                            {
                                return Err(DiagnosticList::rule(
                                    RuleDiagnosticCode::InvalidWiring,
                                ));
                            }
                        }
                        sets.insert(set.path.clone(), set);
                    }
                }
                Ok(CandidateDefinitionsLoaded {
                    sets: sets.into_values().collect(),
                    router: Some(CandidateRouter {
                        path,
                        definition,
                        targets,
                    }),
                })
            }
        }
    })
    .await
    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))??;
    let mut skipped_diagnostics = DiagnosticList::default();
    for set in &loaded.sets {
        for skipped in &set.skipped {
            skipped_diagnostics.extend(skipped.diagnostics().clone());
        }
    }
    if !skipped_diagnostics.is_empty() {
        return Err(skipped_diagnostics);
    }
    for set in &loaded.sets {
        for rule in &set.rules {
            if let Some(pass) = &rule.definition.mechanical_pass {
                pass.validate().map_err(|error| {
                    let mut diagnostics = DiagnosticList::from(error.diagnostics());
                    crate::definition_admission::locate(&mut diagnostics, &rule.source, rule.line);
                    diagnostics
                })?;
            }
        }
    }
    Ok(loaded)
}
