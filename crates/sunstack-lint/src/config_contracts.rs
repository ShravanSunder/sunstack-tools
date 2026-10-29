use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
};
use sunstack_decisions::ModelName;
use sunstack_evidence::{EvidenceSettings, MarkerPrefix};
use sunstack_rules::RulebookPair;
#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct InvalidConfigValue(pub(crate) &'static str);
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GlobPattern(String);
impl GlobPattern {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidConfigValue> {
        let value = value.into();
        if value.is_empty() || globset::Glob::new(&value).is_err() {
            return Err(InvalidConfigValue("invalid glob pattern"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for GlobPattern {
    type Error = InvalidConfigValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<GlobPattern> for String {
    fn from(value: GlobPattern) -> Self {
        value.0
    }
}
impl JsonSchema for GlobPattern {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "GlobPattern".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","minLength":1})
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintLimits {
    #[schemars(range(min = 1000))]
    pub evidence_max_chars: Option<NonZeroU32>,
    pub request_max_chars: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub max_files: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u64::MAX))]
    pub max_file_bytes: Option<NonZeroU64>,
    #[schemars(extend("maximum" = u64::MAX))]
    pub max_total_bytes: Option<NonZeroU64>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub max_matches: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub max_model_requests: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub max_input_tokens_per_request: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub attempt_timeout_seconds: Option<NonZeroU32>,
    #[schemars(extend("maximum" = u32::MAX))]
    pub max_concurrent_requests: Option<NonZeroU32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckedFiles {
    #[schemars(length(min = 1))]
    pub files: Vec<GlobPattern>,
    #[serde(default)]
    pub ignore: Vec<GlobPattern>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionModelSettings {
    pub model: ModelName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RulesConfig {
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[schemars(length(min = 1))]
    pub rulebook: Vec<RulebookPair>,
    pub marker_prefix: MarkerPrefix,
    #[serde(default = "default_question_words")]
    pub max_question_words: NonZeroU32,
    pub check: CheckedFiles,
    pub evidence: EvidenceSettings,
    pub decision_model: DecisionModelSettings,
    #[serde(default)]
    pub limits: Option<LintLimits>,
    #[serde(skip)]
    pub(crate) source: Option<ConfigSource>,
}
fn default_question_words() -> NonZeroU32 {
    NonZeroU32::new(400).unwrap_or(NonZeroU32::MIN)
}
#[derive(Debug, Clone)]
pub(crate) struct ConfigSource {
    pub location: ConfigLocation,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct ConfigLocation {
    pub file: PathBuf,
    pub directory: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LintInvalidCode {
    NoConfigFound,
    Syntax,
    UnknownField,
    DuplicateKey,
    DuplicateEntry,
    SameRuleSetTwice,
    UnknownSelector,
    BadGlob,
    MissingRuleSet,
    UnsupportedInputs,
    OldSeverityWord,
    PathNotFound,
    NoGitForDefaultScope,
    BackendMissing,
    CredentialMissing,
}
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct LintInvalid {
    pub file: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub json_path: Vec<String>,
    pub code: LintInvalidCode,
    pub message: String,
}
impl LintInvalid {
    pub fn at(file: impl Into<PathBuf>, code: LintInvalidCode, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: Some(1),
            column: Some(1),
            json_path: vec![],
            code,
            message: message.into(),
        }
    }
}
