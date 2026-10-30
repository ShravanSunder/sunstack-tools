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

#[derive(Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct AstGrepRule(pub SerializableRule);

impl<'de> Deserialize<'de> for AstGrepRule {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = serde_json::Value::deserialize(deserializer)?;
        let rule = SerializableRule::deserialize(&input).map_err(serde::de::Error::custom)?;
        let output = serde_json::to_value(&rule).map_err(serde::de::Error::custom)?;
        if missing_input_key(&input, &output).is_some() {
            return Err(serde::de::Error::custom("unknown field in ast_grep rule"));
        }
        Ok(Self(rule))
    }
}

// The vendor owns accepted fields, including flattened relations. Compare only
// key paths: serialization may normalize values or add defaults legitimately.
fn missing_input_key(input: &serde_json::Value, output: &serde_json::Value) -> Option<Vec<String>> {
    match input {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                let Some(retained) = output.get(key) else {
                    return Some(vec![key.clone()]);
                };
                if let Some(mut path) = missing_input_key(value, retained) {
                    path.insert(0, key.clone());
                    return Some(path);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if let Some(mut path) = missing_input_key(value, &output[index]) {
                    path.insert(0, index.to_string());
                    return Some(path);
                }
            }
        }
        _ => {}
    }
    None
}

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
