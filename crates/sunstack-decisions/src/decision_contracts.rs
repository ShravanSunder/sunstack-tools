use crate::decision_diagnostics::{DecisionDiagnostic, DecisionDiagnosticCode, DiagnosticList};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, num::NonZeroU64};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("A nonempty decision identifier is required.")]
pub struct InvalidIdentifier;

macro_rules! named_string {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
                let value = value.into();
                if value.is_empty() {
                    Err(InvalidIdentifier)
                } else {
                    Ok(Self(value))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidIdentifier;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
named_string!(QuestionName);
named_string!(ChoiceLabel);
named_string!(ModelName);

// Author content is JSON itself, so its variants intentionally have no wire tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StructuredJson {
    Null,
    Boolean(bool),
    Number(serde_json::Number),
    Text(String),
    Array(Vec<StructuredJson>),
    Object(#[serde(deserialize_with = "unique_map")] BTreeMap<String, StructuredJson>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StructuredEntry {
    Null,
    Text(String),
    Array(Vec<StructuredJson>),
    Object(#[serde(deserialize_with = "unique_map")] BTreeMap<String, StructuredJson>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SharedEvidenceState {
    Text(String),
    Array(Vec<StructuredJson>),
    Object(#[serde(deserialize_with = "unique_map")] BTreeMap<String, StructuredJson>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoulDescriptions {
    #[serde(rename = "true")]
    pub true_description: StructuredEntry,
    #[serde(rename = "false")]
    pub false_description: StructuredEntry,
}
fn specified_criteria<'de, TDeserializer: Deserializer<'de>>(
    deserializer: TDeserializer,
) -> Result<Option<NoulDescriptions>, TDeserializer::Error> {
    NoulDescriptions::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum JudgmentQuestion {
    Noul {
        instructions: StructuredEntry,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "specified_criteria"
        )]
        criteria: Option<NoulDescriptions>,
    },
    Choice {
        instructions: StructuredEntry,
        #[serde(deserialize_with = "unique_map")]
        criteria: BTreeMap<ChoiceLabel, StructuredEntry>,
    },
    Score {
        instructions: StructuredEntry,
        criteria: Vec<StructuredEntry>,
    },
}

pub type NamedQuestionMap = BTreeMap<QuestionName, JudgmentQuestion>;
pub type NamedAnswerMap = BTreeMap<QuestionName, JudgmentAnswer>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJudgmentRequest {
    #[serde(deserialize_with = "unique_map")]
    pub questions: NamedQuestionMap,
    pub state: SharedEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum JudgmentAnswer {
    Noul {
        yes_probability: f64,
    },
    Choice {
        selected_label: ChoiceLabel,
        confidence: f64,
        #[serde(deserialize_with = "unique_map")]
        probabilities: BTreeMap<ChoiceLabel, f64>,
    },
    Score {
        score: f64,
        confidence: f64,
        #[serde(deserialize_with = "unique_map")]
        legend: BTreeMap<String, StructuredEntry>,
        #[serde(deserialize_with = "unique_map")]
        probabilities: BTreeMap<String, f64>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub resolved_model: ModelName,
    pub usage: ExecutionUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackendSelection {
    JevOpenrouter {
        endpoint: String,
        model: ModelName,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "specified_context_ceiling"
        )]
        context_ceiling_tokens: Option<NonZeroU64>,
    },
}
fn specified_context_ceiling<'de, TDeserializer: Deserializer<'de>>(
    deserializer: TDeserializer,
) -> Result<Option<NonZeroU64>, TDeserializer::Error> {
    NonZeroU64::deserialize(deserializer).map(Some)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptSettings {
    pub attempt_timeout_ms: u64,
    pub retry_count: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub max_response_bytes: usize,
}
impl Default for AttemptSettings {
    fn default() -> Self {
        Self {
            attempt_timeout_ms: 10_000,
            retry_count: 0,
            initial_backoff_ms: 100,
            max_backoff_ms: 1_000,
            max_response_bytes: 1_048_576,
        }
    }
}
impl AttemptSettings {
    pub fn validate(&self) -> Result<(), DecisionDiagnostic> {
        if self.attempt_timeout_ms == 0
            || self.initial_backoff_ms == 0
            || self.max_backoff_ms < self.initial_backoff_ms
            || self.max_response_bytes == 0
            || self.retry_count == u32::MAX
        {
            Err(DecisionDiagnostic::request(
                DecisionDiagnosticCode::InvalidConfiguration,
            ))
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountingPolicy {
    RequireExact,
    AllowEstimated,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestBudgetSettings {
    pub max_input_tokens: u64,
    pub max_request_bytes: usize,
    pub max_questions: usize,
    pub max_requests: usize,
    pub safety_margin_tokens: u64,
    pub accounting_policy: AccountingPolicy,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunJudgmentsInput {
    pub request: StructuredJudgmentRequest,
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureClass {
    ProviderRejectedRequest,
    Authentication,
    Authorization,
    InsufficientCredits,
    ProviderResourceUnavailable,
    RequestTooLarge,
    RateLimit,
    ProviderUnavailable,
    Timeout,
    Connection,
    HttpError,
    UnexpectedProviderError,
}
/// An attempt can end through cancellation or invalid response without the
/// whole run being a provider failure. Kept separate by the adopted contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptFailureClass {
    ProviderRejectedRequest,
    Authentication,
    Authorization,
    InsufficientCredits,
    ProviderResourceUnavailable,
    RequestTooLarge,
    RateLimit,
    ProviderUnavailable,
    Timeout,
    Connection,
    HttpError,
    UnexpectedProviderError,
    InvalidResponse,
    CallerAborted,
}
impl From<ProviderFailureClass> for AttemptFailureClass {
    fn from(value: ProviderFailureClass) -> Self {
        match value {
            ProviderFailureClass::ProviderRejectedRequest => Self::ProviderRejectedRequest,
            ProviderFailureClass::Authentication => Self::Authentication,
            ProviderFailureClass::Authorization => Self::Authorization,
            ProviderFailureClass::InsufficientCredits => Self::InsufficientCredits,
            ProviderFailureClass::ProviderResourceUnavailable => Self::ProviderResourceUnavailable,
            ProviderFailureClass::RequestTooLarge => Self::RequestTooLarge,
            ProviderFailureClass::RateLimit => Self::RateLimit,
            ProviderFailureClass::ProviderUnavailable => Self::ProviderUnavailable,
            ProviderFailureClass::Timeout => Self::Timeout,
            ProviderFailureClass::Connection => Self::Connection,
            ProviderFailureClass::HttpError => Self::HttpError,
            ProviderFailureClass::UnexpectedProviderError => Self::UnexpectedProviderError,
        }
    }
}
impl AttemptFailureClass {
    pub fn provider_class(self) -> Option<ProviderFailureClass> {
        match self {
            Self::ProviderRejectedRequest => Some(ProviderFailureClass::ProviderRejectedRequest),
            Self::Authentication => Some(ProviderFailureClass::Authentication),
            Self::Authorization => Some(ProviderFailureClass::Authorization),
            Self::InsufficientCredits => Some(ProviderFailureClass::InsufficientCredits),
            Self::ProviderResourceUnavailable => {
                Some(ProviderFailureClass::ProviderResourceUnavailable)
            }
            Self::RequestTooLarge => Some(ProviderFailureClass::RequestTooLarge),
            Self::RateLimit => Some(ProviderFailureClass::RateLimit),
            Self::ProviderUnavailable => Some(ProviderFailureClass::ProviderUnavailable),
            Self::Timeout => Some(ProviderFailureClass::Timeout),
            Self::Connection => Some(ProviderFailureClass::Connection),
            Self::HttpError => Some(ProviderFailureClass::HttpError),
            Self::UnexpectedProviderError => Some(ProviderFailureClass::UnexpectedProviderError),
            Self::InvalidResponse | Self::CallerAborted => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttemptEvidence {
    NotDispatched {
        failure_class: AttemptFailureClass,
    },
    Completed {
        http_status: u16,
    },
    RemoteOutcomeUnknown {
        failure_class: AttemptFailureClass,
        #[serde(skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceAttemptRecord {
    pub ordinal: u32,
    pub evidence: AttemptEvidence,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionRunOutcome {
    Completed {
        answers: NamedAnswerMap,
        receipt: ExecutionReceipt,
        attempts: Vec<InferenceAttemptRecord>,
    },
    InvalidRequest {
        diagnostics: DiagnosticList,
    },
    ProviderFailure {
        failure_class: ProviderFailureClass,
        #[serde(skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
        diagnostics: DiagnosticList,
        attempts: Vec<InferenceAttemptRecord>,
    },
    InvalidResponse {
        diagnostics: DiagnosticList,
        attempts: Vec<InferenceAttemptRecord>,
    },
    Cancelled {
        diagnostics: DiagnosticList,
        attempts: Vec<InferenceAttemptRecord>,
    },
}
impl DecisionRunOutcome {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Completed { .. } => 0,
            Self::InvalidRequest { .. } => 2,
            Self::ProviderFailure { .. } | Self::InvalidResponse { .. } => 3,
            Self::Cancelled { .. } => 4,
        }
    }
}

pub(crate) fn unique_map<'de, TDeserializer, TKey, TValue>(
    deserializer: TDeserializer,
) -> Result<BTreeMap<TKey, TValue>, TDeserializer::Error>
where
    TDeserializer: Deserializer<'de>,
    TKey: Deserialize<'de> + Ord,
    TValue: Deserialize<'de>,
{
    struct UniqueMapVisitor<TKey, TValue>(std::marker::PhantomData<(TKey, TValue)>);
    impl<'de, TKey: Deserialize<'de> + Ord, TValue: Deserialize<'de>> serde::de::Visitor<'de>
        for UniqueMapVisitor<TKey, TValue>
    {
        type Value = BTreeMap<TKey, TValue>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map with unique keys")
        }
        fn visit_map<TAccess: serde::de::MapAccess<'de>>(
            self,
            mut access: TAccess,
        ) -> Result<Self::Value, TAccess::Error> {
            let mut entries = BTreeMap::new();
            while let Some((key, value)) = access.next_entry()? {
                if entries.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate field"));
                }
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(UniqueMapVisitor(std::marker::PhantomData))
}
