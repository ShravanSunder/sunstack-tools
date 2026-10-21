use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
};
use sunstack_decisions::ModelName;
use sunstack_rules::RuleId;

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct InvalidConfigValue(pub(crate) &'static str);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntryName(String);
impl EntryName {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidConfigValue> {
        let value = value.into();
        if value.is_empty() || value.contains('/') {
            return Err(InvalidConfigValue(
                "entry names must be nonempty and contain no slash",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for EntryName {
    type Error = InvalidConfigValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<EntryName> for String {
    fn from(value: EntryName) -> Self {
        value.0
    }
}
impl JsonSchema for EntryName {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "EntryName".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","minLength":1,"pattern":"^[^/]+$"})
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum RuleSelector {
    Exact { entry: EntryName, rule: RuleId },
    AllInEntry { entry: EntryName },
}
impl RuleSelector {
    pub fn entry(&self) -> &EntryName {
        match self {
            Self::Exact { entry, .. } | Self::AllInEntry { entry } => entry,
        }
    }
    pub fn written(&self) -> String {
        match self {
            Self::Exact { entry, rule } => format!("{}/{}", entry.as_str(), rule.as_str()),
            Self::AllInEntry { entry } => format!("{}/*", entry.as_str()),
        }
    }
}
impl TryFrom<String> for RuleSelector {
    type Error = InvalidConfigValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (entry, rule) = value.split_once('/').ok_or(InvalidConfigValue(
            "invalid rule selector: use <entry>/<rule> or <entry>/*",
        ))?;
        let entry = EntryName::new(entry)?;
        if rule == "*" {
            Ok(Self::AllInEntry { entry })
        } else {
            Ok(Self::Exact {
                entry,
                rule: RuleId::new(rule).map_err(|_| {
                    InvalidConfigValue("invalid rule selector: use an authored rule ID")
                })?,
            })
        }
    }
}
impl From<RuleSelector> for String {
    fn from(value: RuleSelector) -> Self {
        value.written()
    }
}
impl JsonSchema for RuleSelector {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "RuleSelector".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","pattern":"^[^/]+/(\\*|[a-z0-9][a-z0-9._-]{0,127})$"})
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeveritySetting {
    Error,
    Warn,
    Off,
}
impl<'de> Deserialize<'de> for SeveritySetting {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "off" => Ok(Self::Off),
            "block" => Err(serde::de::Error::custom(
                "old severity word: use error instead of block",
            )),
            "nudge" => Err(serde::de::Error::custom(
                "old severity word: use warn instead of nudge",
            )),
            _ => Err(serde::de::Error::custom(
                "use error, warn or off for a severity setting",
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleSetEntry {
    #[schemars(length(min = 1))]
    pub path: PathBuf,
    #[schemars(length(min = 1))]
    pub files: Vec<GlobPattern>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverrideBlock {
    #[schemars(length(min = 1))]
    pub files: Vec<GlobPattern>,
    #[schemars(extend("minProperties"=1))]
    pub rules: BTreeMap<RuleSelector, SeveritySetting>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackendConfig {
    JevOpenrouter {
        model: ModelName,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        endpoint: Option<String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintLimits {
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
pub struct LintConfig {
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[schemars(extend("minProperties"=1))]
    pub rule_sets: BTreeMap<EntryName, RuleSetEntry>,
    #[serde(default)]
    pub rules: BTreeMap<RuleSelector, SeveritySetting>,
    #[serde(default)]
    pub overrides: Vec<OverrideBlock>,
    #[serde(default)]
    pub ignore_patterns: Vec<GlobPattern>,
    #[serde(default)]
    pub backend: Option<BackendConfig>,
    #[serde(default)]
    pub limits: Option<LintLimits>,
    // Diagnostic source is invocation-local metadata, never part of the config wire/schema.
    #[serde(skip)]
    pub(crate) source: Option<ConfigSource>,
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
