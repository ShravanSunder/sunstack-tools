use crate::{
    rule_definitions::definition_loading::DefinitionLoader, rule_definitions::input_binding::*, *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use sunstack_evidence::{AdapterRegistry, EvidencePlanInput, SourceName, SourceSpec};
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone)]
pub(crate) struct LoadedRuleSet {
    pub path: PathBuf,
    pub definition: RuleSetDefinition,
    pub rules: Vec<LocatedRuleDefinition>,
    pub skipped: Vec<SkippedRule>,
}
#[derive(Debug, Clone)]
pub(crate) struct AdmittedGroup {
    pub set: Arc<LoadedRuleSet>,
    pub position: usize,
    pub binding: GroupBinding,
    pub sources: AdmittedSources,
}
pub(crate) struct AdmittedRouter {
    pub definition: RouterDefinition,
    pub groups: Vec<AdmittedSources>,
}
pub(crate) struct AdmittedInvocation {
    pub groups: Vec<AdmittedGroup>,
    pub router: Option<AdmittedRouter>,
    pub evidence: EvidencePlanInput,
}
/// Validated rule definitions stored by the native engine for a later binding.
#[derive(Clone)]
pub struct CapturedRuleSet {
    definition_root: PathBuf,
    requested_path: PathBuf,
    set: Arc<LoadedRuleSet>,
}

pub async fn capture_rule_set(
    definition_root: PathBuf,
    requested_path: PathBuf,
    input_contract: BTreeMap<SourceName, InputDeclaration>,
    max_file_bytes: usize,
    cancellation: CancellationToken,
) -> Result<CapturedRuleSet, DiagnosticList> {
    if requested_path.as_os_str().is_empty() || definition_root.as_os_str().is_empty() {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    tokio::task::spawn_blocking(move || {
        let cwd = std::env::current_dir()
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
        let root = resolved_path(&cwd, &definition_root)?;
        let path = resolved_path(&root, &requested_path)?;
        let set = DefinitionLoader::new(max_file_bytes, cancellation).load_set(path)?;
        if set.definition.inputs != input_contract {
            let mut diagnostic = RuleDiagnostic::new(RuleDiagnosticCode::InputKindMismatch);
            diagnostic.guidance =
                "Match the companion's input names and declarations to the captured rule set."
                    .into();
            diagnostic = diagnostic.definition(&set.path, 1, 1, vec!["inputs".into()]);
            return Err(diagnostic.into());
        }
        Ok(CapturedRuleSet {
            definition_root,
            requested_path,
            set,
        })
    })
    .await
    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?
}

pub fn validate_rule_settings(
    definition_root: &Path,
    attempts: &sunstack_decisions::AttemptSettings,
    request_budget: &sunstack_decisions::RequestBudgetSettings,
) -> Result<(), DiagnosticList> {
    attempts.validate().map_err(|diagnostic| {
        DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
    })?;
    if request_budget.max_input_tokens == 0
        || request_budget.max_request_bytes == 0
        || request_budget.max_questions == 0
        || request_budget.max_requests == 0
        || definition_root.as_os_str().is_empty()
    {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    Ok(())
}

pub(crate) async fn admit_invocation(
    input: &RuleInvocationInput,
    adapters: &AdapterRegistry,
    cancellation: &CancellationToken,
    capture: Option<CapturedRuleSet>,
) -> Result<AdmittedInvocation, DiagnosticList> {
    validate_rule_settings(
        &input.definition_root,
        &input.attempts,
        &input.request_budget,
    )?;
    if let InvocationOrigin::Hook {
        host,
        event,
        session,
        turn,
        agent,
        ..
    } = &input.origin
        && (host.is_empty()
            || event.is_empty()
            || session.as_ref().is_some_and(String::is_empty)
            || turn.as_ref().is_some_and(String::is_empty)
            || matches!(agent,Some(OriginAgent::Subagent{id,agent_type}) if id.is_empty() || agent_type.as_ref().is_some_and(String::is_empty)))
    {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    let owned = input.clone();
    let cancellation = cancellation.clone();
    let (groups, router, sources) = tokio::task::spawn_blocking(move || {
        load_invocation_with_capture(
            &owned.definition_root,
            &owned.definitions,
            &owned.repositories,
            owned.limits.max_file_bytes.get(),
            cancellation,
            capture,
        )
    })
    .await
    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))??;
    for spec in sources.values() {
        if let SourceSpec::Conversation { format, .. } = spec {
            adapters
                .adapter(format)
                .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        }
    }
    let cwd = std::env::current_dir()
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let root = resolved_path(&cwd, &input.definition_root)?;
    let mut payload_schemas = input.payload_schemas.clone();
    for entry in &mut payload_schemas {
        entry.path = resolved_path(&root, &entry.path)?;
    }
    let evidence = EvidencePlanInput {
        repositories: input.repositories.clone(),
        sources,
        payload_schemas,
        cursors: input.cursors.clone(),
        declarations: mapped_declarations(&groups, router.as_ref())?,
        native_queries: vec![],
        package_bindings: input.package_bindings.clone(),
        limits: input.limits.clone(),
        discovery: input.discovery.clone(),
    };
    Ok(AdmittedInvocation {
        groups,
        router,
        evidence,
    })
}
type LoadedInvocation = (
    Vec<AdmittedGroup>,
    Option<AdmittedRouter>,
    BTreeMap<SourceName, SourceSpec>,
);
pub(crate) fn load_invocation(
    definition_root: &Path,
    definitions: &InvocationDefinitions,
    repositories: &[sunstack_evidence::RepositoryBinding],
    max_bytes: usize,
    cancellation: CancellationToken,
) -> Result<LoadedInvocation, DiagnosticList> {
    load_invocation_with_capture(
        definition_root,
        definitions,
        repositories,
        max_bytes,
        cancellation,
        None,
    )
}

fn load_invocation_with_capture(
    definition_root: &Path,
    definitions: &InvocationDefinitions,
    repositories: &[sunstack_evidence::RepositoryBinding],
    max_bytes: usize,
    cancellation: CancellationToken,
    capture: Option<CapturedRuleSet>,
) -> Result<LoadedInvocation, DiagnosticList> {
    if let Some(captured) = &capture
        && (captured.definition_root != definition_root
            || !matches!(definitions, InvocationDefinitions::RuleSets { rule_sets }
                if rule_sets.len() == 1 && rule_sets[0].path == captured.requested_path))
    {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    let cwd = std::env::current_dir()
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let root = resolved_path(&cwd, definition_root)?;
    let mut loader = DefinitionLoader::new(max_bytes, cancellation);
    let mut groups = Vec::new();
    let mut sources = BTreeMap::new();
    let router = match definitions {
        InvocationDefinitions::RuleSets { rule_sets } => {
            if rule_sets.is_empty() {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::InvalidConfiguration,
                ));
            }
            let mut requested_paths = BTreeSet::new();
            for requested in rule_sets {
                if requested.binding_groups.is_empty() || requested.path.as_os_str().is_empty() {
                    return Err(DiagnosticList::rule(
                        RuleDiagnosticCode::InvalidConfiguration,
                    ));
                }
                let set = if let Some(captured) = &capture {
                    captured.set.clone()
                } else {
                    let path = resolved_path(&root, &requested.path)?;
                    if !requested_paths.insert(path.clone()) {
                        return Err(DiagnosticList::rule(RuleDiagnosticCode::DuplicateRuleSet));
                    }
                    loader.load_set(path)?
                };
                for (position, group) in requested.binding_groups.iter().enumerate() {
                    let bound =
                        admit_group(&set.definition.inputs, group, repositories, &mut sources)?;
                    groups.push(AdmittedGroup {
                        set: set.clone(),
                        position,
                        binding: GroupBinding::Bound,
                        sources: bound,
                    });
                }
            }
            None
        }
        InvocationDefinitions::Router {
            path,
            binding_groups,
        } => {
            if binding_groups.is_empty() || path.as_os_str().is_empty() {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::InvalidConfiguration,
                ));
            }
            let path = resolved_path(&root, path)?;
            let definition = decode_router(&loader.read(&path)?, &path)?;
            let directory = path
                .parent()
                .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidRoute))?;
            let mut router_groups = Vec::new();
            for group in binding_groups {
                router_groups.push(admit_group(
                    &definition.inputs,
                    group,
                    repositories,
                    &mut sources,
                )?);
            }
            for (label, targets) in &definition.routes {
                let mut target_paths = BTreeSet::new();
                for target in targets {
                    let path = resolved_path(directory, &target.rule_set)?;
                    if !target_paths.insert(path.clone()) {
                        return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidRoute));
                    }
                    let set = loader.load_set(path)?;
                    for (position, group) in binding_groups.iter().enumerate() {
                        let wired =
                            wire_group(target, group, &set.definition.inputs, &definition.inputs)?;
                        let bound = admit_group(
                            &set.definition.inputs,
                            &wired,
                            repositories,
                            &mut sources,
                        )?;
                        groups.push(AdmittedGroup {
                            set: set.clone(),
                            position,
                            binding: GroupBinding::Routed {
                                router_group: position,
                                label: label.clone(),
                            },
                            sources: bound,
                        });
                    }
                }
            }
            Some(AdmittedRouter {
                definition,
                groups: router_groups,
            })
        }
    };
    Ok((groups, router, sources))
}

pub(crate) fn mapped_declarations(
    groups: &[AdmittedGroup],
    router: Option<&AdmittedRouter>,
) -> Result<Vec<sunstack_evidence::EvidenceDeclaration>, DiagnosticList> {
    let mut mapped = Vec::new();
    for group in groups {
        append_entry_declarations(
            &mut mapped,
            &group.set.definition.applicability,
            &group.sources,
        )?;
        for rule in &group.set.rules {
            append_declarations(&mut mapped, &rule.definition.evidence, &group.sources)?;
            append_entry_declarations(&mut mapped, &rule.definition.applicability, &group.sources)?;
        }
    }
    if let Some(router) = router {
        for sources in &router.groups {
            append_declarations(&mut mapped, &router.definition.evidence, sources)?;
        }
    }
    Ok(mapped)
}
fn append_entry_declarations(
    mapped: &mut Vec<sunstack_evidence::EvidenceDeclaration>,
    entries: &[ApplicabilityEntry],
    sources: &AdmittedSources,
) -> Result<(), DiagnosticList> {
    for entry in entries {
        if let ApplicabilityEntry::Decision { evidence, .. } = entry {
            append_declarations(mapped, evidence, sources)?;
        }
    }
    Ok(())
}
fn append_declarations(
    mapped: &mut Vec<sunstack_evidence::EvidenceDeclaration>,
    declarations: &[sunstack_evidence::EvidenceDeclaration],
    sources: &AdmittedSources,
) -> Result<(), DiagnosticList> {
    for declaration in declarations {
        let source = sources
            .get(&declaration.input)
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UndeclaredInput))?;
        let mut declaration = declaration.clone();
        declaration.input = source.name.clone();
        declaration.id = sunstack_evidence::DeclarationId::new(format!("d{:06}", mapped.len()))
            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        mapped.push(declaration);
    }
    Ok(())
}
