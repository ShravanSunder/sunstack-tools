use crate::{
    config_decoding::{config_error, validate_config_shape},
    *,
};
use std::path::{Path, PathBuf};
use sunstack_evidence::{EvidenceProject, load_evidence_project};
use sunstack_rules::{RulebookAssembly, RulebookPair, admit_evidence_kinds, assemble_rulebook};
use tokio_util::sync::CancellationToken;

pub(crate) struct CheckedRulebook {
    pub project: EvidenceProject,
    pub assembly: RulebookAssembly,
    pub inputs: Vec<PathBuf>,
    pub selections: Vec<sunstack_rules::AdmittedSelection>,
}
pub(crate) fn config_directory(config: &RulesConfig) -> PathBuf {
    config
        .source
        .as_ref()
        .map(|source| source.location.directory.clone())
        .unwrap_or_else(|| PathBuf::from("."))
}
pub(crate) fn load_checked_rulebook(
    config: &RulesConfig,
) -> Result<CheckedRulebook, Vec<LintDiagnostic>> {
    validate_config_shape(config).map_err(|error| vec![error.into()])?;
    let base = config_directory(config);
    let project =
        load_evidence_project(&base.join(&config.evidence.ast_grep)).map_err(|error| {
            let mut diagnostic =
                LintInvalid::at(error.path, LintInvalidCode::Syntax, error.message);
            diagnostic.line = u32::try_from(error.line).ok();
            diagnostic.column = error.column.and_then(|column| u32::try_from(column).ok());
            vec![diagnostic.into()]
        })?;
    let limits = config
        .limits
        .as_ref()
        .map(EffectiveLimits::from)
        .unwrap_or_default();
    let errors = admit_evidence_kinds(
        &config.evidence.kinds,
        &project,
        limits.evidence_max_chars as usize,
    );
    if !errors.is_empty() {
        return Err(errors
            .into_iter()
            .map(|error| {
                config_error(
                    config,
                    &error.field_path,
                    LintInvalidCode::Syntax,
                    error.message,
                )
                .into()
            })
            .collect());
    }
    let pairs = config
        .rulebook
        .iter()
        .map(|pair| RulebookPair {
            markdown: base.join(&pair.markdown),
            rules: base.join(&pair.rules),
        })
        .collect::<Vec<_>>();
    let assembly = assemble_rulebook(
        &pairs,
        &config.marker_prefix,
        config.max_question_words.get() as usize,
        &config.evidence.kinds,
        &project,
    )
    .map_err(|errors| native_diagnostics(config, errors))?;
    let mut inputs = project.inputs.clone();
    for pair in &pairs {
        inputs.push(pair.markdown.clone());
        inputs.push(pair.rules.clone());
    }
    if let Some(source) = &config.source {
        inputs.push(source.location.file.clone());
    }
    let selections =
        sunstack_rules::admit_selections(&config.evidence.kinds).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    config_error(
                        config,
                        &error.field_path,
                        LintInvalidCode::Syntax,
                        error.message,
                    )
                    .into()
                })
                .collect::<Vec<LintDiagnostic>>()
        })?;
    Ok(CheckedRulebook {
        selections,
        project,
        assembly,
        inputs,
    })
}
pub async fn check(config: &RulesConfig, cancellation: CancellationToken) -> CheckResult {
    let config = config.clone();
    if cancellation.is_cancelled() {
        return CheckResult::Invalid {
            diagnostics: vec![],
        };
    }
    let result = tokio::task::spawn_blocking(move || {
        let checked = load_checked_rulebook(&config)?;
        let mut diagnostics = Vec::new();
        for skipped in &checked.assembly.skipped {
            diagnostics.extend(native_diagnostics(&config, skipped.diagnostics.clone()));
        }
        if diagnostics.is_empty() {
            let mut rows = checked
                .assembly
                .rules
                .iter()
                .map(|rule| CheckListing::Rule(listing(rule)))
                .collect::<Vec<_>>();
            rows.extend(
                config
                    .evidence
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
        } else {
            Err(diagnostics)
        }
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
fn listing(rule: &sunstack_rules::RulebookRule) -> RuleListing {
    RuleListing {
        rule: rule.name.clone(),
        marker: rule.sentence.marker.clone(),
        source: SourceLocation {
            file: rule.sentence.rulebook.clone(),
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
    config: &RulesConfig,
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
