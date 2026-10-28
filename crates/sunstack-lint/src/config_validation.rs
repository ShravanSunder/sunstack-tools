use crate::{
    config_decoding::{config_error, validate_config_shape},
    report_contracts::CheckedEntry,
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};
use sunstack_evidence::{ChangeScope, QueryPath, RepositoryBinding, RepositoryId};
use sunstack_rules::{
    BindingGroup, BoundRuleSet, CheckRulesInput, CheckRulesOutcome, CheckRulesOutput, InputBinding,
    InputDeclaration, InvocationDefinitions, PrintedRuleSource, VerdictDeclaration,
};
use tokio_util::sync::CancellationToken;

enum HeaderReadError {
    Unreadable,
    Invalid(sunstack_rules::DiagnosticList),
}
fn read_header(
    path: PathBuf,
    max_bytes: u64,
) -> Result<(PathBuf, sunstack_rules::RuleSetDefinition), HeaderReadError> {
    let path = path
        .canonicalize()
        .map_err(|_| HeaderReadError::Unreadable)?;
    let file = std::fs::File::open(&path).map_err(|_| HeaderReadError::Unreadable)?;
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| HeaderReadError::Unreadable)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(HeaderReadError::Invalid(
            sunstack_rules::RuleDiagnostic::new(
                sunstack_rules::RuleDiagnosticCode::RuleExceedsBudget,
            )
            .definition(&path, 1, 1, vec![])
            .into(),
        ));
    }
    let header =
        sunstack_rules::decode_rule_set(&bytes, &path).map_err(HeaderReadError::Invalid)?;
    Ok((path, header))
}

pub(crate) fn config_directory(config: &LintConfig) -> PathBuf {
    config
        .source
        .as_ref()
        .map(|source| source.location.directory.clone())
        .unwrap_or_else(|| PathBuf::from("."))
}
pub(crate) async fn checked_entries(
    config: &LintConfig,
    cancellation: CancellationToken,
) -> Result<Vec<CheckedEntry>, Vec<LintDiagnostic>> {
    validate_config_shape(config).map_err(|error| vec![error.into()])?;
    let directory = config_directory(config);
    let limits = config
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    let native_limits = limits
        .invocation_limits()
        .map_err(|error| vec![error.into()])?;
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for (name, entry) in &config.rule_sets {
        if cancellation.is_cancelled() {
            return Err(vec![
                config_error(
                    config,
                    "rule_sets",
                    LintInvalidCode::Syntax,
                    "Config checking cancelled.",
                )
                .into(),
            ]);
        }
        let joined = directory.join(&entry.path);
        let max_bytes = limits.max_file_bytes;
        let header_path = joined.clone();
        let loaded = tokio::task::spawn_blocking(move || read_header(header_path, max_bytes))
            .await
            .map_err(|_| {
                vec![
                    config_error(
                        config,
                        name.as_str(),
                        LintInvalidCode::Syntax,
                        "Rule-set loading failed.",
                    )
                    .into(),
                ]
            })?;
        let (path, header) = loaded.map_err(|error| match error {
            HeaderReadError::Invalid(diagnostics) => native_diagnostics(config, diagnostics),
            HeaderReadError::Unreadable => vec![
                config_error(
                    config,
                    name.as_str(),
                    LintInvalidCode::MissingRuleSet,
                    format!("Cannot read rule-set file {}.", joined.display()),
                )
                .into(),
            ],
        })?;
        if !seen.insert(path.clone()) {
            return Err(vec![
                config_error(
                    config,
                    name.as_str(),
                    LintInvalidCode::SameRuleSetTwice,
                    "Two entries name the same rule-set file.",
                )
                .into(),
            ]);
        }
        if header.inputs.len() != 1
            || !header
                .inputs
                .values()
                .all(|kind| matches!(kind, InputDeclaration::Document {}))
        {
            let kinds = header
                .inputs
                .values()
                .map(|kind| match kind {
                    InputDeclaration::Document {} => "document",
                    InputDeclaration::Conversation {} => "conversation",
                    InputDeclaration::ChangeSet {} => "change_set",
                    InputDeclaration::Payload { .. } => "payload",
                })
                .collect::<Vec<_>>()
                .join(", ");
            return Err(vec![
                config_error(
                    config,
                    name.as_str(),
                    LintInvalidCode::UnsupportedInputs,
                    format!(
                        "Lint requires exactly one document input; this rule set has: {kinds}."
                    ),
                )
                .into(),
            ]);
        }
        let Some(input_name) = header.inputs.keys().next().cloned() else {
            return Err(vec![
                config_error(
                    config,
                    name.as_str(),
                    LintInvalidCode::UnsupportedInputs,
                    "Lint requires one document input.",
                )
                .into(),
            ]);
        };
        let repository = RepositoryId::new("lint").map_err(|_| {
            vec![
                config_error(
                    config,
                    name.as_str(),
                    LintInvalidCode::Syntax,
                    "Invalid repository identity.",
                )
                .into(),
            ]
        })?;
        let document = config
            .source
            .as_ref()
            .and_then(|source| source.location.file.file_name())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("sunstack.lint.json"));
        let native = CheckRulesInput {
            definition_root: directory.clone(),
            definitions: InvocationDefinitions::RuleSets {
                rule_sets: vec![BoundRuleSet {
                    path: path.clone(),
                    binding_groups: vec![BindingGroup {
                        bindings: BTreeMap::from([(
                            input_name.clone(),
                            InputBinding::Document {
                                repository: repository.clone(),
                                path: document,
                            },
                        )]),
                    }],
                }],
            },
            repositories: vec![RepositoryBinding {
                key: repository,
                root: directory.clone(),
                input_paths: vec![QueryPath::Glob {
                    pattern: "**".into(),
                }],
                change_scope: ChangeScope::Full {},
            }],
            payload_schemas: vec![],
            limits: native_limits.clone(),
            discovery: limits.discovery().map_err(|error| vec![error.into()])?,
            output: CheckRulesOutput::AssembledRules,
        };
        let printed = match sunstack_rules::check_rules(native, cancellation.clone()).await {
            CheckRulesOutcome::Checked { printed_rules, .. } => printed_rules,
            CheckRulesOutcome::Refused { diagnostics, .. }
            | CheckRulesOutcome::CheckFailed { diagnostics, .. }
            | CheckRulesOutcome::Cancelled { diagnostics, .. } => {
                return Err(native_diagnostics(config, diagnostics));
            }
        };
        entries.push(CheckedEntry {
            name: name.clone(),
            path,
            input: input_name,
            rules: printed,
        });
    }
    for selector in config
        .rules
        .keys()
        .chain(config.overrides.iter().flat_map(|block| block.rules.keys()))
    {
        let Some(entry) = entries.iter().find(|entry| &entry.name == selector.entry()) else {
            return Err(vec![
                config_error(
                    config,
                    &selector.written(),
                    LintInvalidCode::UnknownSelector,
                    "The selector names an unknown rule-set entry.",
                )
                .into(),
            ]);
        };
        if let RuleSelector::Exact { rule, .. } = selector
            && !entry
                .rules
                .iter()
                .any(|loaded| &loaded.definition.rule_id == rule)
        {
            return Err(vec![
                config_error(
                    config,
                    &selector.written(),
                    LintInvalidCode::UnknownSelector,
                    "The selector names an unknown rule ID.",
                )
                .into(),
            ]);
        }
    }
    Ok(entries)
}
pub async fn check(config: &LintConfig, cancellation: CancellationToken) -> CheckResult {
    match checked_entries(config, cancellation).await {
        Ok(entries) => CheckResult::Valid {
            rules: entry_listings(config, &entries, None),
        },
        Err(diagnostics) => CheckResult::Invalid { diagnostics },
    }
}
pub fn list_rules(
    config: &LintConfig,
    checked: &CheckResult,
    file: Option<&Path>,
) -> Vec<RuleListing> {
    match checked {
        CheckResult::Invalid { .. } => vec![],
        CheckResult::Valid { rules } => rules
            .iter()
            .map(|listing| {
                let mut listing = listing.clone();
                if let RuleSelector::Exact { entry, rule } = &listing.rule {
                    listing.for_file = file.map(|file| {
                        effective_setting(config, entry, rule, listing.authored, Some(file))
                    });
                }
                listing
            })
            .collect(),
    }
}
pub(crate) fn entry_listings(
    config: &LintConfig,
    entries: &[CheckedEntry],
    file: Option<&Path>,
) -> Vec<RuleListing> {
    entries
        .iter()
        .flat_map(|entry| {
            entry.rules.iter().map(|rule| {
                let source = match &rule.source {
                    PrintedRuleSource::Jsonl { file, line } => SourceLocation {
                        file: file.clone(),
                        line: u32::try_from(*line).unwrap_or(u32::MAX),
                    },
                    PrintedRuleSource::Markdown { provenance } => SourceLocation {
                        file: provenance.entry_location.file.clone(),
                        line: u32::try_from(provenance.entry_location.line).unwrap_or(u32::MAX),
                    },
                };
                let setting = effective_setting(
                    config,
                    &entry.name,
                    &rule.definition.rule_id,
                    rule.definition.severity,
                    None,
                );
                RuleListing {
                    rule: RuleSelector::Exact {
                        entry: entry.name.clone(),
                        rule: rule.definition.rule_id.clone(),
                    },
                    source,
                    kind: match rule.definition.verdict {
                        VerdictDeclaration::Mechanical { .. } => RuleKind::Mechanical,
                        VerdictDeclaration::Noul { .. } | VerdictDeclaration::Composite { .. } => {
                            RuleKind::Judged
                        }
                    },
                    authored: rule.definition.severity,
                    configured: setting.setting,
                    for_file: file.map(|file| {
                        effective_setting(
                            config,
                            &entry.name,
                            &rule.definition.rule_id,
                            rule.definition.severity,
                            Some(file),
                        )
                    }),
                }
            })
        })
        .collect()
}
pub(crate) fn native_diagnostics(
    config: &LintConfig,
    diagnostics: sunstack_rules::DiagnosticList,
) -> Vec<LintDiagnostic> {
    diagnostics
        .items
        .into_iter()
        .map(|item| {
            let (file, line, column, message) = match item {
                sunstack_rules::ReportDiagnostic::Rule(diagnostic) => {
                    let (file, line, column) = match diagnostic.location {
                        sunstack_rules::RuleDiagnosticLocation::Definition {
                            file,
                            line,
                            column,
                            ..
                        } => (
                            Some(file),
                            u32::try_from(line).ok(),
                            u32::try_from(column).ok(),
                        ),
                        _ => (None, None, None),
                    };
                    (file, line, column, diagnostic.guidance)
                }
                sunstack_rules::ReportDiagnostic::Evidence(diagnostic) => (
                    None,
                    None,
                    None,
                    format!("Evidence admission refused: {:?}.", diagnostic.code),
                ),
                sunstack_rules::ReportDiagnostic::Decision(diagnostic) => (
                    None,
                    None,
                    None,
                    format!("Decision admission refused: {:?}.", diagnostic.code),
                ),
            };
            let code =
                if message.contains("old severity") || message.contains("old confidence-band") {
                    LintInvalidCode::OldSeverityWord
                } else if message == "Lint backend configuration is missing." {
                    LintInvalidCode::BackendMissing
                } else if message.starts_with("OpenRouter credential is missing.") {
                    LintInvalidCode::CredentialMissing
                } else {
                    LintInvalidCode::Syntax
                };
            LintDiagnostic {
                file: file.or_else(|| {
                    config
                        .source
                        .as_ref()
                        .map(|source| source.location.file.clone())
                }),
                line: line.or(Some(1)),
                column: column.or(Some(1)),
                code,
                message,
            }
        })
        .collect()
}
