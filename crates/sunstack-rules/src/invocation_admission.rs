use crate::{definition_admission::locate, input_binding::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
use sunstack_evidence::{
    AdapterRegistry, EvidenceDiscovery, EvidencePlanInput, SourceName, SourceSpec,
};
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone)]
pub struct LocatedRuleDefinition {
    pub source: PathBuf,
    pub line: usize,
    pub definition: Arc<RuleDefinition>,
}
#[derive(Debug, Clone)]
pub struct ValidatedRuleFile {
    pub source: PathBuf,
    pub content_identity: String,
    pub rules: Vec<LocatedRuleDefinition>,
}
#[derive(Debug)]
pub(crate) struct LoadedRuleSet {
    pub path: PathBuf,
    pub definition: RuleSetDefinition,
    pub rules: Vec<LocatedRuleDefinition>,
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
struct DefinitionLoader {
    max_bytes: usize,
    cancellation: CancellationToken,
    files: BTreeMap<PathBuf, Arc<ValidatedRuleFile>>,
    sets: BTreeMap<PathBuf, Arc<LoadedRuleSet>>,
}
impl DefinitionLoader {
    fn read(&self, path: &Path) -> Result<Vec<u8>, DiagnosticList> {
        let invalid = || {
            RuleDiagnostic::new(RuleDiagnosticCode::MalformedJson)
                .definition(path, 1, 1, vec![])
                .into()
        };
        if self.cancellation.is_cancelled() {
            return Err(DiagnosticList::rule(RuleDiagnosticCode::CallerAborted));
        }
        let metadata = std::fs::metadata(path).map_err(|_| invalid())?;
        if !metadata.is_file() || metadata.len() > self.max_bytes as u64 {
            return Err(invalid());
        }
        let mut file = std::fs::File::open(path).map_err(|_| invalid())?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 16384];
        loop {
            if self.cancellation.is_cancelled() {
                return Err(DiagnosticList::rule(RuleDiagnosticCode::CallerAborted));
            }
            let read = file.read(&mut buffer).map_err(|_| invalid())?;
            if read == 0 {
                break;
            }
            if bytes.len().saturating_add(read) > self.max_bytes {
                return Err(invalid());
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        Ok(bytes)
    }
    fn load_set(&mut self, path: PathBuf) -> Result<Arc<LoadedRuleSet>, DiagnosticList> {
        if let Some(set) = self.sets.get(&path) {
            return Ok(set.clone());
        }
        let definition = decode_rule_set(&self.read(&path)?, &path)?;
        let directory = path
            .parent()
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
        let mut source_paths = BTreeSet::new();
        let mut rule_ids = BTreeSet::new();
        let mut rules = Vec::new();
        for source in &definition.rule_paths {
            let source = resolved_path(directory, source)?;
            if !source_paths.insert(source.clone()) {
                return Err(RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleSource)
                    .definition(&path, 1, 1, vec!["rule_paths".into()])
                    .into());
            }
            let file = if let Some(file) = self.files.get(&source) {
                file.clone()
            } else {
                let bytes = self.read(&source)?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::MalformedJson))?;
                let mut located = Vec::new();
                for (position, line) in text.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let rule = decode_rule_definition(line.as_bytes(), &source, position + 1)?;
                    located.push(LocatedRuleDefinition {
                        source: source.clone(),
                        line: position + 1,
                        definition: Arc::new(rule),
                    });
                }
                if located.is_empty() {
                    return Err(RuleDiagnostic::new(RuleDiagnosticCode::EmptyRuleFile)
                        .definition(&source, 1, 1, vec![])
                        .into());
                }
                let file = Arc::new(ValidatedRuleFile {
                    source: source.clone(),
                    content_identity: sunstack_evidence::content_digest(&bytes),
                    rules: located,
                });
                self.files.insert(source, file.clone());
                file
            };
            for located in &file.rules {
                let rule = &located.definition;
                if !rule_ids.insert(rule.rule_id.clone()) {
                    return Err(RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleId)
                        .definition(&located.source, located.line, 1, vec!["rule_id".into()])
                        .into());
                }
                validate_rule_definition(rule, &definition.inputs).map_err(|mut diagnostics| {
                    locate(&mut diagnostics, &located.source, located.line);
                    diagnostics
                })?;
                rules.push(located.clone());
            }
        }
        let set = Arc::new(LoadedRuleSet {
            path: path.clone(),
            definition,
            rules,
        });
        self.sets.insert(path, set.clone());
        Ok(set)
    }
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
    let (groups, router, sources) =
        tokio::task::spawn_blocking(move || load_invocation(&owned, cancellation))
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
    Ok(AdmittedInvocation {
        groups,
        router,
        evidence: EvidencePlanInput {
            repositories: input.repositories.clone(),
            sources,
            payload_schemas,
            cursors: input.cursors.clone(),
            declarations: vec![],
            native_queries: vec![],
            package_bindings: vec![],
            limits: input.limits.clone(),
            discovery: EvidenceDiscovery::None,
        },
    })
}
type LoadedInvocation = (
    Vec<AdmittedGroup>,
    Option<AdmittedRouter>,
    BTreeMap<SourceName, SourceSpec>,
);
fn load_invocation(
    input: &RuleInvocationInput,
    cancellation: CancellationToken,
) -> Result<LoadedInvocation, DiagnosticList> {
    let cwd = std::env::current_dir()
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let root = resolved_path(&cwd, &input.definition_root)?;
    let mut loader = DefinitionLoader {
        max_bytes: input.limits.max_file_bytes.get(),
        cancellation,
        files: BTreeMap::new(),
        sets: BTreeMap::new(),
    };
    let mut groups = Vec::new();
    let mut sources = BTreeMap::new();
    let router = match &input.definitions {
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
                    let bound = admit_group(
                        &set.definition.inputs,
                        group,
                        &input.repositories,
                        &mut sources,
                    )?;
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
                    &input.repositories,
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
                            &input.repositories,
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
