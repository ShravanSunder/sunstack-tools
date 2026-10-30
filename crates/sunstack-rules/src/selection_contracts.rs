use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SelectedReason {
    FullScope,
    TargetChanged,
    ContextChanged,
    RuleChanged,
    NewRule,
    ApplicabilityChanged,
    BaselineUnavailable,
    BeforeAnalysisUnknown,
}

/// A selection always explains itself with unique reasons in the contract's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "Vec<SelectedReason>", into = "Vec<SelectedReason>")]
#[schemars(with = "Vec<SelectedReason>")]
pub struct SelectedReasons(Box<[SelectedReason]>);
impl SelectedReasons {
    pub fn full_scope() -> Self {
        Self(vec![SelectedReason::FullScope].into_boxed_slice())
    }
    pub fn as_slice(&self) -> &[SelectedReason] {
        &self.0
    }
}
impl TryFrom<Vec<SelectedReason>> for SelectedReasons {
    type Error = SelectionReasonError;
    fn try_from(reasons: Vec<SelectedReason>) -> Result<Self, Self::Error> {
        if reasons.is_empty() {
            return Err(SelectionReasonError::Empty);
        }
        if reasons.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(SelectionReasonError::InvalidOrder);
        }
        Ok(Self(reasons.into_boxed_slice()))
    }
}
impl From<SelectedReasons> for Vec<SelectedReason> {
    fn from(reasons: SelectedReasons) -> Self {
        reasons.0.into_vec()
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SelectionReasonError {
    #[error("a selected occurrence requires a reason")]
    Empty,
    #[error("selection reasons must be unique and canonically ordered")]
    InvalidOrder,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutsideChangeReason {
    NoDeclaredChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OccurrenceSelection {
    Selected { selected_by: SelectedReasons },
    ComparisonPending {},
}
impl OccurrenceSelection {
    pub fn full_scope() -> Self {
        Self::Selected {
            selected_by: SelectedReasons::full_scope(),
        }
    }
    pub fn selected_reasons(&self) -> Option<&SelectedReasons> {
        match self {
            Self::Selected { selected_by } => Some(selected_by),
            Self::ComparisonPending {} => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdmittedSelection {
    pub kind: sunstack_evidence::EvidenceKindName,
    pub question: sunstack_decisions::JudgmentQuestion,
    pub yes_at_or_above: f64,
    pub no_at_or_below: f64,
    pub shared: Vec<sunstack_evidence::FieldName>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestLimit {
    RequestMaxChars,
    QuestionsPerRequest,
    RequestBytes,
    InputTokens,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectionIncompleteReason {
    EvidenceUnavailable { field: sunstack_evidence::FieldName },
    EvidenceTooLarge,
    RequestTooLarge { limit: RequestLimit },
    InvalidRequest,
    ProviderFailure,
    MalformedAnswer,
    RequestLimitReached,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectionOutcome {
    Selected { yes: f64 },
    NotSelected { yes: f64 },
    Uncertain { yes: f64 },
    Incomplete { reason: SelectionIncompleteReason },
}
#[derive(Debug, Clone)]
pub struct SelectionInput {
    pub selection: AdmittedSelection,
    pub candidates: Vec<sunstack_evidence::SelectionCandidate>,
    pub shared: Vec<sunstack_evidence::SharedView>,
    pub files: std::collections::BTreeSet<std::path::PathBuf>,
}
