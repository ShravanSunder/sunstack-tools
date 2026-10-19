use crate::{
    config_decoding::config_error,
    config_validation::config_directory,
    report_contracts::CheckedEntry,
    targets::{TargetSelection, relative_path},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use sunstack_decisions::{BackendSelection, ModelName};
use sunstack_evidence::{
    ChangeCaptureRequest, ChangeScope, DirtyLayer, DiscoveryCoverage, EvidenceDiscovery, QueryPath,
    RepositoryBinding, RepositoryId, RevisionHead,
};
use sunstack_rules::{
    BindingGroup, BoundRuleSet, GroupKey, InputBinding, InvocationDefinitions, InvocationOrigin,
    RuleId, RuleInvocationInput, RulePolicy,
};
use tokio_util::sync::CancellationToken;

pub(crate) struct TargetSettings {
    pub entry: EntryName,
    pub file: PathBuf,
    pub rules: BTreeMap<RuleId, EffectiveSetting>,
}
pub(crate) enum InvocationPreparation {
    NothingSelected {
        exclusions: ExclusionCounts,
        scope: ScopeReport,
    },
    Ready {
        input: Box<RuleInvocationInput>,
        policy: RulePolicy,
        targets: BTreeMap<GroupKey, TargetSettings>,
        exclusions: ExclusionCounts,
        scope: ScopeReport,
    },
}
pub(crate) enum InvocationSetupError {
    Invalid(LintInvalid),
    Incomplete(String),
}
impl From<LintInvalid> for InvocationSetupError {
    fn from(value: LintInvalid) -> Self {
        Self::Invalid(value)
    }
}
pub(crate) fn backend_selection(config: &LintConfig) -> Result<BackendSelection, LintInvalid> {
    match &config.backend {
        Some(BackendConfig::JevOpenrouter { model, endpoint }) => {
            Ok(BackendSelection::JevOpenrouter {
                model: model.clone(),
                endpoint: endpoint
                    .clone()
                    .unwrap_or_else(|| "https://openrouter.ai/api/v1/systemone".into()),
                context_ceiling_tokens: None,
            })
        }
        None => Ok(BackendSelection::JevOpenrouter {
            model: ModelName::new("jev-latest").map_err(|_| {
                config_error(
                    config,
                    "backend",
                    LintInvalidCode::Syntax,
                    "Invalid default model name.",
                )
            })?,
            endpoint: "https://openrouter.ai/api/v1/systemone".into(),
            context_ceiling_tokens: None,
        }),
    }
}
pub(crate) async fn prepare_invocation(
    config: &LintConfig,
    entries: &[CheckedEntry],
    selection: TargetSelection,
    request: &LintRequest,
    limits: &EffectiveLimits,
    cancellation: CancellationToken,
) -> Result<InvocationPreparation, InvocationSetupError> {
    let directory = config_directory(config);
    let root_directory = directory.clone();
    let git_root = tokio::task::spawn_blocking(move || {
        git2::Repository::discover(root_directory)
            .ok()
            .and_then(|repository| repository.workdir().map(PathBuf::from))
    })
    .await
    .map_err(|_| InvocationSetupError::Incomplete("Repository discovery failed.".into()))?;
    let is_full = matches!(request.scope, ScopeChoice::All)
        || (matches!(request.scope, ScopeChoice::Default) && !request.paths.is_empty());
    if git_root.is_none() && !is_full {
        return Err(config_error(
            config,
            "rule_sets",
            LintInvalidCode::NoGitForDefaultScope,
            "This change scope requires Git; use --all outside a Git repository.",
        )
        .into());
    }
    let root = git_root.unwrap_or_else(|| directory.clone());
    let repository = RepositoryId::new("lint").map_err(|_| {
        config_error(
            config,
            "rule_sets",
            LintInvalidCode::Syntax,
            "Invalid repository identity.",
        )
    })?;
    let mut change_scope = if is_full {
        ChangeScope::Full {}
    } else {
        match &request.scope {
            ScopeChoice::Since { rev } => ChangeScope::RevisionDiff {
                base_revision: rev.clone(),
                head: RevisionHead::Worktree {},
            },
            ScopeChoice::Default => ChangeScope::Uncommitted {
                layers: vec![
                    DirtyLayer::Staged,
                    DirtyLayer::Unstaged,
                    DirtyLayer::Untracked,
                ],
            },
            ScopeChoice::All => ChangeScope::Full {},
        }
    };
    let mut scope = ScopeReport {
        kind: if is_full {
            ScopeChoice::All
        } else {
            request.scope.clone()
        },
        widened: None,
    };
    if change_scope.has_change_selection()
        && let Some(source) = &config.source
    {
        let relative = relative_path(&root, &source.location.file);
        let discovery = limits.discovery()?;
        let EvidenceDiscovery::Bounded {
            limits: discovery_limits,
        } = discovery
        else {
            return Err(InvocationSetupError::Incomplete(
                "Change discovery is unavailable.".into(),
            ));
        };
        let capture = sunstack_evidence::capture_change_selection(
            ChangeCaptureRequest {
                binding: RepositoryBinding {
                    key: repository.clone(),
                    root: root.clone(),
                    input_paths: vec![QueryPath::Exact {
                        path: relative.clone(),
                    }],
                    change_scope: change_scope.clone(),
                },
                limits: discovery_limits,
                anchor_utc: chrono::Utc::now(),
            },
            cancellation,
        )
        .await;
        match capture {
            Ok(capture) => {
                if !matches!(
                    capture.coverage,
                    DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
                ) {
                    return Err(InvocationSetupError::Incomplete(
                        "Config change selection reached a discovery limit.".into(),
                    ));
                }
                if capture
                    .members
                    .iter()
                    .flat_map(|member| &member.changes)
                    .any(|change| change.path == relative)
                {
                    change_scope = ChangeScope::Full {};
                    scope.kind = ScopeChoice::All;
                    scope.widened = Some(WidenedReason::ConfigChanged);
                }
            }
            Err(error)
                if matches!(
                    error.code,
                    sunstack_evidence::EvidenceCode::RevisionUnavailable
                        | sunstack_evidence::EvidenceCode::InvalidRepositoryBinding
                ) =>
            {
                return Err(config_error(
                    config,
                    "rule_sets",
                    LintInvalidCode::Syntax,
                    format!(
                        "Cannot resolve the selected Git change scope: {:?}.",
                        error.code
                    ),
                )
                .into());
            }
            Err(error) => {
                return Err(InvocationSetupError::Incomplete(format!(
                    "Config change selection failed: {:?}.",
                    error.code
                )));
            }
        }
    }
    let mut policy = RulePolicy::default();
    let mut targets = BTreeMap::new();
    let mut rule_sets = Vec::new();
    let mut exclusions = ExclusionCounts {
        ignored: selection.ignored,
        ..ExclusionCounts::default()
    };
    for (entry, files) in entries.iter().zip(selection.entries) {
        let mut binding_groups = Vec::new();
        for file in files {
            let config_relative = relative_path(&directory, &file);
            let settings = entry
                .rules
                .iter()
                .map(|rule| {
                    (
                        rule.definition.rule_id.clone(),
                        effective_setting(
                            config,
                            &entry.name,
                            &rule.definition.rule_id,
                            rule.definition.severity,
                            Some(&config_relative),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let off = settings
                .iter()
                .filter(|(_, setting)| setting.setting == SeveritySetting::Off)
                .map(|(rule, _)| rule.clone())
                .collect::<BTreeSet<_>>();
            exclusions.off = exclusions
                .off
                .saturating_add(u32::try_from(off.len()).unwrap_or(u32::MAX));
            if off.len() == settings.len() {
                continue;
            }
            let group = binding_groups.len();
            let key = GroupKey {
                rule_set: entry.path.clone(),
                group,
            };
            binding_groups.push(BindingGroup {
                bindings: BTreeMap::from([(
                    entry.input.clone(),
                    InputBinding::Document {
                        repository: repository.clone(),
                        path: relative_path(&root, &file),
                    },
                )]),
            });
            if !off.is_empty() {
                policy.off.insert(key.clone(), off);
            }
            targets.insert(
                key,
                TargetSettings {
                    entry: entry.name.clone(),
                    file: relative_path(&root, &file),
                    rules: settings,
                },
            );
        }
        if !binding_groups.is_empty() {
            rule_sets.push(BoundRuleSet {
                path: entry.path.clone(),
                binding_groups,
            });
        }
    }
    if rule_sets.is_empty() {
        return Ok(InvocationPreparation::NothingSelected { exclusions, scope });
    }
    let input = RuleInvocationInput {
        definition_root: directory,
        definitions: InvocationDefinitions::RuleSets { rule_sets },
        repositories: vec![RepositoryBinding {
            key: repository,
            root,
            input_paths: vec![QueryPath::Glob {
                pattern: "**".into(),
            }],
            change_scope,
        }],
        payload_schemas: vec![],
        origin: InvocationOrigin::Direct {},
        cursors: vec![],
        execution: limits.execution()?,
        limits: limits.invocation_limits()?,
        backend: backend_selection(config)?,
        attempts: limits.attempts()?,
        request_budget: limits.request_budget()?,
        discovery: limits.discovery()?,
        package_bindings: vec![],
    };
    Ok(InvocationPreparation::Ready {
        input: Box::new(input),
        policy,
        targets,
        exclusions,
        scope,
    })
}
