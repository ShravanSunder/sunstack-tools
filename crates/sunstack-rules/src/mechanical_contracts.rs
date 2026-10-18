use crate::{DiagnosticList, RuleDiagnosticCode};
use ast_grep_config::{DeserializeEnv, SerializableRule};
use ast_grep_language::SupportLang;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum ExpectPredicate {
    Matches { matches: String },
    NotMatches { not_matches: String },
    StartsWith { starts_with: String },
    Present { present: bool },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AstGrepRule(pub SerializableRule);

impl std::fmt::Debug for AstGrepRule {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AstGrepRule")
            .finish_non_exhaustive()
    }
}
impl AstGrepRule {
    pub(crate) fn compile(&self) -> Result<ast_grep_config::Rule, DiagnosticList> {
        DeserializeEnv::new(SupportLang::Markdown)
            .deserialize_rule(self.0.clone())
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidMechanicalCheck))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MechanicalCheck {
    Expect { expect: ExpectPredicate },
    AstGrep { ast_grep: Box<AstGrepRule> },
}
impl MechanicalCheck {
    pub(crate) fn validate(&self) -> Result<(), DiagnosticList> {
        let invalid = || DiagnosticList::rule(RuleDiagnosticCode::InvalidMechanicalCheck);
        match self {
            Self::Expect { expect } => match expect {
                ExpectPredicate::Matches { matches: pattern }
                | ExpectPredicate::NotMatches {
                    not_matches: pattern,
                } => {
                    regex::Regex::new(pattern).map_err(|_| invalid())?;
                }
                ExpectPredicate::Present { present: false } => return Err(invalid()),
                _ => {}
            },
            Self::AstGrep { ast_grep } => {
                ast_grep.compile()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MechanicalResult {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalObservation {
    AstMatch { span: sunstack_evidence::SourceSpan },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalSettlement {
    Completed {
        result: MechanicalResult,
        observations: Vec<LocalObservation>,
    },
    Unresolved {
        diagnostics: DiagnosticList,
    },
    Cancelled,
}

impl LocalSettlement {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }
}
