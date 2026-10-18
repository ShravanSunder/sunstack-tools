use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sunstack_evidence::{SourceName, SourceRef};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleEvaluationResult {
    Expanded {
        key: RuleEvaluationKey,
        occurrences: Vec<RuleOccurrenceKey>,
    },
    ZeroMatches {
        key: RuleEvaluationKey,
        input: SourceName,
        source: SourceRef,
    },
    NotApplicable {
        key: RuleEvaluationKey,
        stopped_at: EntryKey,
        basis: EntryBasis,
    },
    OutsideChangeScope {
        key: RuleEvaluationKey,
        reason: OutsideChangeReason,
    },
    SelectionFailed {
        key: RuleEvaluationKey,
        diagnostics: DiagnosticList,
    },
    EvaluationFailed {
        key: RuleEvaluationKey,
        cause: Cause,
    },
    Cancelled {
        key: RuleEvaluationKey,
    },
}

impl RuleEvaluationResult {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Expanded { occurrences, .. } => !occurrences.is_empty(),
            Self::ZeroMatches { .. }
            | Self::NotApplicable { .. }
            | Self::OutsideChangeScope { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedRuleEvaluation {
    WaitingForExpansion {
        key: RuleEvaluationKey,
        on: Vec<EntryKey>,
    },
    Expanded {
        key: RuleEvaluationKey,
        occurrences: Vec<RuleOccurrenceKey>,
    },
    ZeroMatches {
        key: RuleEvaluationKey,
        input: SourceName,
        source: SourceRef,
    },
    NotApplicable {
        key: RuleEvaluationKey,
        stopped_at: EntryKey,
        basis: EntryBasis,
    },
    OutsideChangeScope {
        key: RuleEvaluationKey,
        reason: OutsideChangeReason,
    },
    SelectionFailed {
        key: RuleEvaluationKey,
        diagnostics: DiagnosticList,
    },
    EvaluationFailed {
        key: RuleEvaluationKey,
        cause: Cause,
    },
    Cancelled {
        key: RuleEvaluationKey,
    },
}

impl From<RuleEvaluationResult> for PreparedRuleEvaluation {
    fn from(result: RuleEvaluationResult) -> Self {
        match result {
            RuleEvaluationResult::Expanded { key, occurrences } => {
                Self::Expanded { key, occurrences }
            }
            RuleEvaluationResult::ZeroMatches { key, input, source } => {
                Self::ZeroMatches { key, input, source }
            }
            RuleEvaluationResult::NotApplicable {
                key,
                stopped_at,
                basis,
            } => Self::NotApplicable {
                key,
                stopped_at,
                basis,
            },
            RuleEvaluationResult::OutsideChangeScope { key, reason } => {
                Self::OutsideChangeScope { key, reason }
            }
            RuleEvaluationResult::SelectionFailed { key, diagnostics } => {
                Self::SelectionFailed { key, diagnostics }
            }
            RuleEvaluationResult::EvaluationFailed { key, cause } => {
                Self::EvaluationFailed { key, cause }
            }
            RuleEvaluationResult::Cancelled { key } => Self::Cancelled { key },
        }
    }
}
