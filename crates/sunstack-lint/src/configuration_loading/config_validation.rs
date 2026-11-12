use crate::{
    configuration_loading::{
        config_decoding::validate_config_shape,
        definition_bundle::{
            DefinitionBundle, EvidenceAcquisition, RootDefinition, acquire_file_definitions,
        },
    },
    *,
};
use std::path::{Path, PathBuf};
use sunstack_evidence::{
    ConfigPath, DefinitionOrigin, EvidenceProject, InlineRule, KindMatcher, ProjectReadFailure,
    ProjectRefusal,
};
use sunstack_rules::{RulebookAssembly, admit_evidence_kinds, assemble_rulebook_sources};
use tokio_util::sync::CancellationToken;

pub(crate) struct CheckedRulebook {
    pub filters: sunstack_rules::FilterRegistry,
    pub project: EvidenceProject,
    pub assembly: RulebookAssembly,
    pub inputs: Vec<PathBuf>,
    pub selections: Vec<sunstack_rules::AdmittedSelection>,
}
/// The file location of a decoded config, or the CLI's fallback for detached values.
pub(crate) fn config_location(config: &RulesConfig) -> ConfigLocation {
    config
        .source
        .as_ref()
        .map(|source| source.location.clone())
        .unwrap_or_else(|| ConfigLocation {
            file: PathBuf::from("sunstack.lint.json"),
            directory: PathBuf::from("."),
        })
}
pub(crate) fn load_checked_rulebook(
    config: &RulesConfig,
) -> Result<CheckedRulebook, Vec<LintDiagnostic>> {
    validate_config_shape(config).map_err(|error| vec![error.into()])?;
    let bundle = acquire_file_definitions(config, &config_location(config));
    admit_definitions(&bundle)
}

fn project_refusal(root: &RootDefinition, error: ProjectRefusal) -> Vec<LintDiagnostic> {
    if let Some(origin) = &error.origin {
        return vec![
            root.field_error(
                origin,
                LintInvalidCode::Syntax,
                format!("{origin}: {}", error.message),
            )
            .into(),
        ];
    }
    let mut diagnostic = LintInvalid::at(error.path, LintInvalidCode::Syntax, error.message)
        .in_definition(&error.definition);
    diagnostic.line = u32::try_from(error.line).ok();
    diagnostic.column = error.column.and_then(|column| u32::try_from(column).ok());
    vec![diagnostic.into()]
}

/// Admit captured definitions with the one shared semantic policy.
pub(crate) fn admit_definitions(
    bundle: &DefinitionBundle,
) -> Result<CheckedRulebook, Vec<LintDiagnostic>> {
    let root = &bundle.root;
    let settings = &bundle.settings;
    let field_error = |path: &ConfigPath, message: String| -> LintDiagnostic {
        root.field_error(path, LintInvalidCode::Syntax, message)
            .into()
    };
    let inline_rules = settings
        .kinds
        .iter()
        .filter_map(|(kind, definition)| match &definition.matcher {
            KindMatcher::Named(_) => None,
            KindMatcher::Inline { text } => Some(InlineRule {
                kind: kind.clone(),
                text: text.clone(),
                origin: ConfigPath::keys(["evidence", "kinds", kind.as_str(), "inline_rule"]),
            }),
        })
        .collect::<Vec<_>>();
    let project = match &bundle.evidence {
        EvidenceAcquisition::ProjectFiles(files) => match files.as_ref() {
            Ok(files) => sunstack_evidence::compile_evidence_project_files(
                files,
                &inline_rules,
                &bundle.inline_kind_origin,
            ),
            Err(ProjectReadFailure::Unavailable(failure)) => Err(failure.refusal.clone()),
            Err(ProjectReadFailure::Refused(refusal)) => Err(refusal.clone()),
        },
        EvidenceAcquisition::Definitions(definitions) => {
            sunstack_evidence::compile_evidence_project(
                definitions,
                &inline_rules,
                &bundle.inline_kind_origin,
            )
        }
    }
    .map_err(|error| project_refusal(root, error))?;
    let limits = settings
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    let filters = sunstack_rules::admit_filters(&settings.filters, &settings.kinds, &project)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    field_error(
                        &error.field_path,
                        format!("{}: {}", error.field_path, error.message),
                    )
                })
                .collect::<Vec<LintDiagnostic>>()
        })?;
    let errors = admit_evidence_kinds(
        &settings.kinds,
        &project,
        limits.evidence_max_chars as usize,
    );
    if !errors.is_empty() {
        return Err(errors
            .into_iter()
            .map(|error| field_error(&error.field_path, error.message))
            .collect());
    }
    let sources = bundle
        .rulebooks
        .iter()
        .map(|source| source.clone().map_err(|failure| failure.diagnostics))
        .collect::<Vec<_>>();
    let assembly = assemble_rulebook_sources(
        &sources,
        &settings.marker_prefix,
        settings.max_question_words.get() as usize,
        &settings.kinds,
        &project,
        &filters,
    )
    .map_err(|errors| native_diagnostics(root, errors))?;
    let mut inputs = project.inputs.clone();
    for source in sources.iter().filter_map(|source| source.as_ref().ok()) {
        inputs.extend(source.markdown_origin.file_path().map(Path::to_path_buf));
        inputs.extend(source.entries_origin.file_path().map(Path::to_path_buf));
    }
    if let Some(path) = root.origin.file_path() {
        inputs.push(path.to_path_buf());
    }
    let selections = sunstack_rules::admit_selections(&settings.kinds).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| field_error(&error.field_path, error.message))
            .collect::<Vec<LintDiagnostic>>()
    })?;
    Ok(CheckedRulebook {
        filters,
        selections,
        project,
        assembly,
        inputs,
    })
}
pub async fn check(config: &RulesConfig, cancellation: CancellationToken) -> CheckResult {
    if cancellation.is_cancelled() {
        return CheckResult::Invalid {
            diagnostics: vec![],
        };
    }
    let config = config.clone();
    let result = tokio::task::spawn_blocking(move || {
        validate_config_shape(&config).map_err(|error| vec![error.into()])?;
        let bundle = acquire_file_definitions(&config, &config_location(&config));
        check_definitions(&bundle)
    })
    .await;
    match result {
        Ok(Ok(rules)) => CheckResult::Valid { rules },
        Ok(Err(diagnostics)) => CheckResult::Invalid { diagnostics },
        Err(_) => CheckResult::Invalid {
            diagnostics: vec![
                LintInvalid::at(
                    "sunstack.lint.json",
                    LintInvalidCode::Syntax,
                    "config checking failed",
                )
                .into(),
            ],
        },
    }
}

/// Admit a bundle and list its rules, refusing any skipped rule. No checked input,
/// credential or provider is touched.
pub fn check_definitions(
    bundle: &DefinitionBundle,
) -> Result<Vec<CheckListing>, Vec<LintDiagnostic>> {
    let checked = admit_definitions(bundle)?;
    let mut diagnostics = Vec::new();
    for skipped in &checked.assembly.skipped {
        diagnostics.extend(native_diagnostics(
            &bundle.root,
            skipped.diagnostics.clone(),
        ));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut rows = checked
        .assembly
        .rules
        .iter()
        .map(|rule| CheckListing::Rule(listing(rule)))
        .collect::<Vec<_>>();
    rows.extend(
        bundle
            .settings
            .kinds
            .iter()
            .filter(|(_, kind)| kind.select.is_some() || kind.within.is_some())
            .map(|(name, kind)| {
                CheckListing::Kind(KindListing {
                    kind: name.clone(),
                    select: kind.select.clone(),
                    within: kind.within.clone(),
                })
            }),
    );
    Ok(rows)
}
fn listing(rule: &sunstack_rules::RulebookRule) -> RuleListing {
    RuleListing {
        rule: rule.name.clone(),
        marker: rule.sentence.marker.clone(),
        source: SourceLocation {
            file: rule.sentence.rulebook.cli_path(),
            line: u32::try_from(rule.sentence.line).unwrap_or(u32::MAX),
        },
        evidence: rule.kind.clone(),
        kind: if rule.questions.is_empty() {
            RuleKind::Mechanical
        } else {
            RuleKind::Asking
        },
        severity: rule.severity,
        standard: rule.sentence.standard.clone(),
    }
}
pub fn list_rules(
    _config: &RulesConfig,
    checked: &CheckResult,
    _file: Option<&Path>,
) -> Vec<RuleListing> {
    match checked {
        CheckResult::Valid { rules } => rules
            .iter()
            .filter_map(|row| match row {
                CheckListing::Rule(rule) => Some(rule.clone()),
                CheckListing::Kind(_) => None,
            })
            .collect(),
        CheckResult::Invalid { .. } => vec![],
    }
}
pub(crate) fn native_diagnostics(
    root: &RootDefinition,
    diagnostics: sunstack_rules::DiagnosticList,
) -> Vec<LintDiagnostic> {
    diagnostics
        .items
        .into_iter()
        .map(|item| {
            let (located, line, column, path, message) = match item {
                sunstack_rules::ReportDiagnostic::Rule(diagnostic) => {
                    let (located, line, column) = match diagnostic.location {
                        sunstack_rules::RuleDiagnosticLocation::Definition {
                            file,
                            line,
                            column,
                            definition,
                            ..
                        } => (
                            Some((
                                file.clone(),
                                definition.unwrap_or_else(|| DefinitionOrigin::file(file)),
                            )),
                            u32::try_from(line).ok(),
                            u32::try_from(column).ok(),
                        ),
                        _ => (None, None, None),
                    };
                    (located, line, column, None, diagnostic.guidance)
                }
                sunstack_rules::ReportDiagnostic::Evidence(diagnostic) => (
                    None,
                    None,
                    None,
                    diagnostic.path,
                    format!("Evidence admission refused: {:?}.", diagnostic.code),
                ),
                sunstack_rules::ReportDiagnostic::Decision(diagnostic) => (
                    None,
                    None,
                    None,
                    None,
                    format!("Decision admission refused: {:?}.", diagnostic.code),
                ),
            };
            let (file, definition) = match located {
                Some((file, definition)) => (Some(file), definition),
                None => (
                    root.text
                        .as_ref()
                        .and(root.origin.file_path().map(Path::to_path_buf)),
                    root.origin.clone(),
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
                file,
                definition: Some(definition),
                json_path: vec![],
                path,
                line: line.or(Some(1)),
                column: column.or(Some(1)),
                code,
                message,
            }
        })
        .collect()
}
