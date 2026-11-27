use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_decisions::{AccountingBasis, RequestBudgetCounts};
use sunstack_evidence::{CaptureCoverage, ReturnedCursor, SelectionResult};
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedRules {
    pub rule_evaluations: Vec<PreparedRuleEvaluation>,
    pub rule_coverage: RuleCoverage,
    pub skipped_rules: Vec<SkippedRule>,
    pub origin: ReportedOrigin,
    pub routing: PreparedRouting,
    pub groups: Vec<PreparedGroup>,
    pub occurrences: Vec<PreparedOccurrence>,
    pub planned_requests: Vec<PlannedRequestRecord>,
    pub captures: Vec<CaptureCoverage>,
    pub change_selections: Vec<sunstack_evidence::ResolvedChangeSelection>,
    pub cursors: Vec<ReturnedCursor>,
    pub measurements: InvocationMeasurements,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedRouting {
    RuleSets,
    Router {
        status: RouterPreparationStatus,
        candidate_rule_sets: Vec<PathBuf>,
        groups: Vec<usize>,
    },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouterPreparationStatus {
    NotEvaluated,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedGroup {
    pub rule_set: PathBuf,
    pub position: usize,
    pub binding: GroupBinding,
    pub leading_entries: Vec<EvaluatedEntry>,
    pub status: PreparedGroupStatus,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedGroupStatus {
    Planned,
    NotApplicable { entry: EntryKey, basis: EntryBasis },
    ApplicabilityFailed { entry: EntryKey, cause: Cause },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedOccurrence {
    pub key: RuleOccurrenceKey,
    pub status: PreparedOccurrenceStatus,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedOccurrenceStatus {
    Planned { selection: OccurrenceSelection },
    OutsideChangeScope { reason: OutsideChangeReason },
    NotApplicable { entry: EntryKey, basis: EntryBasis },
    SelectionFailed { diagnostics: DiagnosticList },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedRequestRecord {
    pub index: usize,
    pub owner: RequestOwner,
    pub mode: RequestMode,
    pub questions: Vec<QuestionBinding>,
    pub prerequisites: Vec<EntryKey>,
    pub readiness: RequestPreparationReadiness,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestPreparationReadiness {
    Prepared {
        budget: PreparedRequestBudget,
        evidence: Vec<SelectionResult>,
    },
    Waiting {
        on: Vec<EntryKey>,
    },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedRequestBudget {
    WithinConfiguredBudget {
        counts: RequestBudgetCounts,
        basis: AccountingBasis,
    },
    Exceeded {
        diagnostics: DiagnosticList,
    },
    Unverifiable {
        diagnostics: DiagnosticList,
    },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparationCancellationProgress {
    BeforePreparationAdmission,
    PreparationAdmitted { partial: Box<PreparedRules> },
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RulePreparationOutcome {
    Prepared {
        preparation: PreparedRules,
    },
    PreparedWithFailures {
        preparation: PreparedRules,
        diagnostics: DiagnosticList,
    },
    PreparationFailed {
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
    Invalid {
        diagnostics: DiagnosticList,
    },
    Cancelled {
        progress: PreparationCancellationProgress,
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
}
impl RulePreparationOutcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Prepared { .. } => 0,
            Self::Invalid { .. } => 2,
            Self::PreparedWithFailures { .. } | Self::PreparationFailed { .. } => 3,
            Self::Cancelled { .. } => 4,
        }
    }
}

/// The credential-free request encoded for this plan position.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRequestPreview {
    pub index: usize,
    pub backend: sunstack_decisions::BackendSelection,
    pub body: String,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RulePreviewOutcome {
    ReadySingle {
        request: ModelRequestPreview,
        preparation: Box<PreparedRules>,
    },
    NoModelWork {
        report: Box<RuleInvocationReport>,
    },
    PendingDependency {
        ready: Vec<ModelRequestPreview>,
        preparation: Box<PreparedRules>,
    },
    UnsupportedMultiple {
        ready: Vec<ModelRequestPreview>,
        preparation: Box<PreparedRules>,
    },
    Unavailable {
        outcome: Box<RulePreparationOutcome>,
    },
}
