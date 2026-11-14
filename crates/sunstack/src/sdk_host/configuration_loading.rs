//! Load an existing CLI configuration into editable captured values.
//!
//! Discovery and decoding are the CLI's own. Every referenced definition is read
//! once; the returned values are complete, so later calls never reread them.
use super::{
    definition_capture::{CallFiles, CaptureFailure, project_refusal, refusal, unavailable},
    matcher_transport::{AstMatcher, AstUtility},
    normalized_values::source_id,
    sdk_contracts::{
        CapturedEvidence, CapturedKind, CapturedRulebook, CapturedSource, CapturedValue,
        DefinitionSourceFormat, Diagnostic, DiagnosticCode, LoadOptions, LoadedConfiguration,
        OriginalConfiguration, ResolvedConfiguration, WholeSourceFormat,
    },
};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{
    CapturedFile, DefinitionOrigin, DefinitionSnapshot, ProjectReadFailure, SnapshotAcquisition,
    SnapshotRepresentation, SourceLocator,
};
use sunstack_lint::{LintInvalid, LintInvalidCode};

pub(crate) fn lint_refusal(error: LintInvalid) -> Diagnostic {
    Diagnostic {
        code: error.code.into(),
        origin: error.definition.clone().map(|definition| *definition),
        path: error.definition.is_none().then(|| error.file.clone()),
        line: error.line,
        column: error.column,
        json_path: error.json_path,
        message: error.message,
    }
}

struct LoadReceipt {
    snapshots: Vec<DefinitionSnapshot>,
    seen: BTreeSet<std::path::PathBuf>,
}

impl LoadReceipt {
    fn read(&mut self, file: &CapturedFile) {
        if self.seen.insert(file.path.clone()) {
            self.snapshots.push(DefinitionSnapshot::new(
                DefinitionOrigin::file(&file.path),
                file.text.clone(),
                SnapshotRepresentation::OriginalText,
                SnapshotAcquisition::Read,
            ));
        }
    }
}

fn file_locator(file: &CapturedFile) -> SourceLocator {
    SourceLocator::File {
        path: file.path.clone(),
    }
}

/// Discover, read and capture a configuration and every definition it references.
pub fn load_configuration(options: &LoadOptions) -> Result<LoadedConfiguration, CaptureFailure> {
    let invalid = |diagnostic: Diagnostic| CaptureFailure::Invalid(vec![diagnostic]);
    if !options.directory.is_absolute() {
        return Err(invalid(refusal(
            DiagnosticCode::UnsupportedInputs,
            None,
            vec!["directory".into()],
            "load directory must be an absolute path",
        )));
    }
    let location =
        sunstack_lint::discover_config(&options.directory, options.config_path.as_deref())
            .map_err(|error| match error.code {
                LintInvalidCode::NoConfigFound => {
                    CaptureFailure::Unavailable(vec![lint_refusal(error)])
                }
                _ => CaptureFailure::Invalid(vec![lint_refusal(error)]),
            })?;
    let mut files = CallFiles::default();
    let config_file = files.read(&location.file)?;
    let config = sunstack_lint::parse_config(&config_file.text, &location)
        .map_err(|error| invalid(lint_refusal(error)))?;
    let mut receipt = LoadReceipt {
        snapshots: vec![],
        seen: BTreeSet::new(),
    };
    receipt.read(&config_file);
    let configuration_id = location.file.display().to_string();

    let project = sunstack_evidence::read_evidence_project(
        &location.directory.join(&config.evidence.ast_grep),
    )
    .map_err(|failure| match failure {
        ProjectReadFailure::Unavailable(failure) => {
            CaptureFailure::Unavailable(vec![unavailable(&failure.source)])
        }
        ProjectReadFailure::Refused(detail) => invalid(project_refusal(&detail)),
    })?;
    receipt.read(&project.project);
    let mut utilities = BTreeMap::new();
    for acquired in &project.utility_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| CaptureFailure::Unavailable(vec![unavailable(&failure.source)]))?;
        receipt.read(file);
        for utility in sunstack_evidence::decode_utility_file(file)
            .map_err(|detail| invalid(project_refusal(&detail)))?
        {
            let id = utility.id.clone();
            let value = AstUtility::from_upstream(utility).map_err(|detail| {
                invalid(refusal(
                    DiagnosticCode::UnsupportedInputs,
                    Some(&DefinitionOrigin::file(&file.path)),
                    detail.json_path,
                    detail.message,
                ))
            })?;
            let captured = CapturedValue {
                value,
                original: CapturedSource::Definition {
                    format: DefinitionSourceFormat::UtilityYaml,
                    origin: file_locator(file),
                    content: file.text.clone(),
                    definition_id: id.clone(),
                },
            };
            if utilities.insert(id.clone(), captured).is_some() {
                return Err(invalid(refusal(
                    DiagnosticCode::DuplicateEntry,
                    Some(&DefinitionOrigin::file(&file.path)),
                    vec![],
                    format!("duplicate utility ID: {id}"),
                )));
            }
        }
    }
    let mut matchers = BTreeMap::new();
    for acquired in &project.rule_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| CaptureFailure::Unavailable(vec![unavailable(&failure.source)]))?;
        receipt.read(file);
        for matcher in sunstack_evidence::decode_matcher_file(file)
            .map_err(|detail| invalid(project_refusal(&detail)))?
        {
            let id = matcher.id.clone();
            let value = AstMatcher::from_upstream(matcher).map_err(|detail| {
                invalid(refusal(
                    DiagnosticCode::UnsupportedInputs,
                    Some(&DefinitionOrigin::file(&file.path)),
                    detail.json_path,
                    detail.message,
                ))
            })?;
            let captured = CapturedValue {
                value,
                original: CapturedSource::Definition {
                    format: DefinitionSourceFormat::MatcherYaml,
                    origin: file_locator(file),
                    content: file.text.clone(),
                    definition_id: id.clone(),
                },
            };
            if matchers.insert(id.clone(), captured).is_some() {
                return Err(invalid(refusal(
                    DiagnosticCode::DuplicateEntry,
                    Some(&DefinitionOrigin::file(&file.path)),
                    vec![],
                    format!("duplicate evidence rule ID: {id}"),
                )));
            }
        }
    }

    let mut rulebooks = Vec::new();
    for (index, pair) in config.rulebook.iter().enumerate() {
        let markdown = files.read(&location.directory.join(&pair.markdown))?;
        let entries = files.read(&location.directory.join(&pair.rules))?;
        receipt.read(&markdown);
        receipt.read(&entries);
        let entries_origin = DefinitionOrigin::file(&entries.path);
        let decoded = sunstack_rules::decode_rule_entries(entries.text.as_bytes(), &entries_origin)
            .map_err(|diagnostics| CaptureFailure::Invalid(rule_refusals(diagnostics)))?;
        let mut values = super::sdk_contracts::RuleEntries::new();
        for (key, entry) in decoded {
            let entry =
                entry.map_err(|diagnostics| CaptureFailure::Invalid(rule_refusals(diagnostics)))?;
            if values.insert(key.clone(), entry).is_some() {
                return Err(invalid(refusal(
                    DiagnosticCode::DuplicateKey,
                    Some(&entries_origin),
                    vec![key],
                    "marker has duplicate rule entries",
                )));
            }
        }
        let index_text = index.to_string();
        rulebooks.push(CapturedRulebook {
            kind: CapturedKind::Captured,
            id: source_id(&configuration_id, &["rulebooks", &index_text]),
            markdown: CapturedValue {
                value: markdown.text.clone(),
                original: CapturedSource::Whole {
                    format: WholeSourceFormat::Markdown,
                    origin: file_locator(&markdown),
                    content: markdown.text.clone(),
                },
            },
            entries: CapturedValue {
                value: values,
                original: CapturedSource::Whole {
                    format: WholeSourceFormat::RuleEntriesJson,
                    origin: file_locator(&entries),
                    content: entries.text.clone(),
                },
            },
        });
    }

    Ok(LoadedConfiguration {
        configuration: ResolvedConfiguration {
            id: configuration_id,
            label: None,
            base_directory: location.directory.clone(),
            original_config: Some(OriginalConfiguration {
                origin: file_locator(&config_file),
                content: config_file.text.clone(),
            }),
            schema: config.schema.clone(),
            marker_prefix: config.marker_prefix.clone(),
            max_question_words: Some(config.max_question_words),
            check: config.check.clone(),
            decision_model: config.decision_model.clone(),
            limits: config.limits.clone(),
            filters: config.filters.clone(),
            rulebooks,
            evidence: CapturedEvidence {
                kind: CapturedKind::Captured,
                matchers,
                utilities,
                kinds: config.evidence.kinds.clone(),
                html: config.evidence.html.clone(),
                markdown: config.evidence.markdown.clone(),
            },
        },
        sources: receipt.snapshots,
    })
}

/// Project rule-definition refusals into SDK diagnostics.
pub(crate) fn rule_refusals(diagnostics: sunstack_rules::DiagnosticList) -> Vec<Diagnostic> {
    diagnostics
        .items
        .into_iter()
        .map(|item| match item {
            sunstack_rules::ReportDiagnostic::Rule(diagnostic) => {
                let (origin, line, column, json_path) = match &diagnostic.location {
                    sunstack_rules::RuleDiagnosticLocation::Definition {
                        file,
                        line,
                        column,
                        field_path,
                        definition,
                    } => (
                        Some(
                            definition
                                .clone()
                                .unwrap_or_else(|| DefinitionOrigin::file(file)),
                        ),
                        u32::try_from(*line).ok(),
                        u32::try_from(*column).ok(),
                        field_path.clone(),
                    ),
                    _ => (None, None, None, vec![]),
                };
                Diagnostic {
                    code: DiagnosticCode::Syntax,
                    origin,
                    path: None,
                    line,
                    column,
                    json_path,
                    message: diagnostic.guidance.clone(),
                }
            }
            sunstack_rules::ReportDiagnostic::Evidence(diagnostic) => Diagnostic {
                code: DiagnosticCode::Syntax,
                origin: None,
                path: diagnostic.path.clone(),
                line: None,
                column: None,
                json_path: vec![],
                message: format!("Evidence admission refused: {:?}.", diagnostic.code),
            },
            sunstack_rules::ReportDiagnostic::Decision(diagnostic) => Diagnostic {
                code: DiagnosticCode::Syntax,
                origin: None,
                path: None,
                line: None,
                column: None,
                json_path: vec![],
                message: format!("Decision admission refused: {:?}.", diagnostic.code),
            },
        })
        .collect()
}
