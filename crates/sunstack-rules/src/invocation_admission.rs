use crate::{definition_loading::DefinitionLoader, input_binding::*, *};
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
pub(crate) async fn admit_invocation(
    input: &RuleInvocationInput,
    adapters: &AdapterRegistry,
    cancellation: &CancellationToken,
) -> Result<AdmittedInvocation, DiagnosticList> {
    input.attempts.validate().map_err(|diagnostic| {
        DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
    })?;
    if input.request_budget.max_input_tokens == 0
        || input.request_budget.max_request_bytes == 0
        || input.request_budget.max_questions == 0
        || input.request_budget.max_requests == 0
        || input.definition_root.as_os_str().is_empty()
    {
        return Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
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
        load_invocation(
            &owned.definition_root,
            &owned.definitions,
            &owned.repositories,
            owned.limits.max_file_bytes.get(),
            cancellation,
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
                let path = resolved_path(&root, &requested.path)?;
                if !requested_paths.insert(path.clone()) {
                    return Err(DiagnosticList::rule(RuleDiagnosticCode::DuplicateRuleSet));
                }
                let set = loader.load_set(path)?;
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
    crate::rule_source_admission::admit_markdown_entries(&mut groups);
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
