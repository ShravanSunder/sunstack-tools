use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf};
use sunstack_decisions::{
    ChoiceLabel, JudgmentQuestion, NamedQuestionMap, QuestionName, StructuredEntry,
};
use sunstack_evidence::{
    DeclarationId, EvidenceDeclaration, SchemaName, SourceName, StructuredJson,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("A rule identifier must match the authored rule-id contract.")]
pub struct InvalidRuleId;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RuleId(String);
impl JsonSchema for RuleId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "RuleId".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({"type":"string","pattern":"^[a-z0-9][a-z0-9._-]{0,127}$"})
    }
}
impl RuleId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidRuleId> {
        let value = value.into();
        if !value.is_empty()
            && value.len() <= 128
            && (value.as_bytes()[0].is_ascii_lowercase() || value.as_bytes()[0].is_ascii_digit())
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            })
        {
            Ok(Self(value))
        } else {
            Err(InvalidRuleId)
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for RuleId {
    type Error = InvalidRuleId;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<RuleId> for String {
    fn from(value: RuleId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputDeclaration {
    Document {},
    Conversation {},
    ChangeSet {},
    Payload { schema: SchemaName },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleSetDefinition {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub description: String,
    #[schemars(extend("minProperties"=1))]
    pub inputs: BTreeMap<SourceName, InputDeclaration>,
    pub applicability: Vec<ApplicabilityEntry>,
    pub common_ruleset_instructions: StructuredEntry,
    #[schemars(length(min = 1))]
    pub rule_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    #[default]
    Warn,
}
impl<'de> Deserialize<'de> for Severity {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "block" => Err(serde::de::Error::custom(
                "severity `block` was renamed to `error`",
            )),
            "nudge" => Err(serde::de::Error::custom(
                "severity `nudge` was renamed to `warn`",
            )),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["error", "warn"],
            )),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub rule_id: RuleId,
    #[schemars(length(min = 1))]
    pub evidence: Vec<EvidenceDeclaration>,
    pub target: DeclarationId,
    pub applicability: Vec<ApplicabilityEntry>,
    pub questions: NamedQuestionMap,
    pub verdict: VerdictDeclaration,
    #[serde(default)]
    pub severity: Severity,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VerdictDeclaration {
    Mechanical {
        explanation: String,
        check: crate::MechanicalCheck,
    },
    Noul {
        #[schemars(schema_with = "noul_schema")]
        question: JudgmentQuestion,
        #[schemars(extend("exclusiveMinimum" = 0, "maximum" = 1))]
        threshold: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("minimum" = 0, "exclusiveMaximum" = 1))]
        error_at_or_below: Option<f64>,
    },
    Composite {
        #[schemars(length(min = 1))]
        dimensions: Vec<CompositeDimension>,
        #[schemars(extend("exclusiveMinimum" = 0, "maximum" = 1))]
        threshold: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("minimum" = 0, "exclusiveMaximum" = 1))]
        error_at_or_below: Option<f64>,
    },
}
impl VerdictDeclaration {
    pub fn threshold(&self) -> Option<f64> {
        match self {
            Self::Noul { threshold, .. } | Self::Composite { threshold, .. } => Some(*threshold),
            Self::Mechanical { .. } => None,
        }
    }
    pub fn band(&self) -> Option<f64> {
        match self {
            Self::Mechanical { .. } => None,
            Self::Noul {
                error_at_or_below, ..
            }
            | Self::Composite {
                error_at_or_below, ..
            } => *error_at_or_below,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositeDimension {
    pub question: QuestionName,
    #[schemars(extend("exclusiveMinimum" = 0, "maximum" = 1))]
    pub weight: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApplicabilityEntry {
    Condition {
        test: ConditionTest,
    },
    Decision {
        #[schemars(schema_with = "noul_schema")]
        question: JudgmentQuestion,
        #[schemars(length(min = 1))]
        evidence: Vec<EvidenceDeclaration>,
        #[schemars(extend("exclusiveMinimum" = 0, "maximum" = 1))]
        threshold: f64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionTest {
    PayloadFieldEquals {
        input: SourceName,
        pointer: String,
        value: StructuredJson,
    },
    PayloadFieldIn {
        input: SourceName,
        pointer: String,
        #[schemars(length(min = 1))]
        values: Vec<StructuredJson>,
    },
    PayloadFieldPresent {
        input: SourceName,
        pointer: String,
    },
    OriginFieldEquals {
        field: OriginField,
        #[schemars(length(min = 1))]
        value: String,
    },
    PatternPresent {
        input: SourceName,
        pattern: PresencePattern,
        max_scan_bytes: NonZeroUsize,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OriginField {
    Host,
    Event,
    AgentKind,
    AgentType,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresencePattern {
    Literal {
        #[schemars(length(min = 1))]
        text: String,
    },
    Regex {
        #[schemars(length(min = 1))]
        pattern: String,
    },
    Ast {
        #[schemars(length(min = 1))]
        language: String,
        #[schemars(length(min = 1))]
        pattern: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouterDefinition {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub description: String,
    #[schemars(extend("minProperties"=1))]
    pub inputs: BTreeMap<SourceName, InputDeclaration>,
    #[schemars(length(min = 1))]
    pub evidence: Vec<EvidenceDeclaration>,
    #[schemars(schema_with = "choice_schema")]
    pub question: JudgmentQuestion,
    #[schemars(extend("minProperties"=1))]
    pub routes: BTreeMap<ChoiceLabel, Vec<RouteTarget>>,
    #[schemars(extend("exclusiveMinimum" = 0, "maximum" = 1))]
    pub confidence_floor: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteTarget {
    pub rule_set: PathBuf,
    #[schemars(extend("minProperties"=1))]
    pub inputs: BTreeMap<SourceName, SourceName>,
}
// Refine the owning decision schema by its native tag; no question fields are restated here.
fn primitive_schema(generator: &mut schemars::SchemaGenerator, kind: &str) -> schemars::Schema {
    let owned = generator.subschema_for::<JudgmentQuestion>();
    schemars::json_schema!({"allOf":[owned,{"properties":{"type":{"const":kind}}}]})
}
fn noul_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    primitive_schema(generator, "noul")
}
fn choice_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    primitive_schema(generator, "choice")
}
