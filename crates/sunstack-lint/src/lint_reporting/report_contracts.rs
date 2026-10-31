use crate::{EffectiveLimits, LintInvalid, LintInvalidCode};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{NamedAnswerMap, SharedEvidenceState};
use sunstack_evidence::{
    EvidenceKindName, EvidencePiece, FieldName, MarkerId, PieceLocation, SourceSpan,
};
use sunstack_rules::{Basis, RuleId, RulebookSkippedRule, Severity, StepName};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum ScopeChoice {
    #[default]
    Default,
    All,
    Since {
        rev: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintRequest {
    pub config_path: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub scope: ScopeChoice,
    pub max_warnings: Option<u32>,
    #[serde(default)]
    pub evidence_listing: Option<Option<sunstack_evidence::EvidenceKindName>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum WidenedReason {
    ConfigChanged,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScopeReport {
    #[serde(flatten)]
    pub kind: ScopeChoice,
    pub widened: Option<WidenedReason>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum LintStatus {
    Clean,
    NothingSelected,
    Findings,
    Incomplete,
    Invalid,
    Cancelled,
    Uncertain,
}
impl LintStatus {
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Clean | Self::NothingSelected => 0,
            Self::Findings => 1,
            Self::Invalid => 2,
            Self::Incomplete => 3,
            Self::Cancelled => 4,
            Self::Uncertain => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum SettingOrigin {
    Authored,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Finding {
    pub rule: RuleId,
    pub marker: MarkerId,
    pub file: PathBuf,
    pub line: usize,
    pub field: Option<FieldName>,
    pub span: Option<SourceSpan>,
    pub severity: Severity,
    pub severity_origin: SettingOrigin,
    pub message: String,
    pub judgment: Option<JudgmentSummary>,
    pub basis: Basis,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct JudgmentSummary {
    pub answers: NamedAnswerMap,
    pub value: Option<f64>,
    pub path_taken: Vec<StepName>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UncertainResult {
    pub rule: RuleId,
    pub marker: MarkerId,
    #[serde(rename = "where")]
    pub location: PieceLocation,
    pub step: StepName,
    pub state: SharedEvidenceState,
    pub answers: NamedAnswerMap,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SeverityCounts {
    pub error: u32,
    pub warn: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExclusionCounts {
    pub off: u32,
    pub ignored: u32,
    pub not_applicable: u32,
    pub outside_change_scope: u32,
    pub zero_matches: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum IncompleteReason {
    SelectionFailed,
    SelectionIncomplete,
    EvaluationFailed,
    NotStarted,
    PreparationFailed,
    CoveragePartial,
    CoverageNone,
    ProviderFailure {
        failure_class: sunstack_decisions::ProviderFailureClass,
        #[serde(skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
    },
    InvalidRequest,
    InvalidResponse,
    RequestLimitReached,
    EvidenceUnavailable,
    EvidenceTooLarge,
    RequestTooLarge,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IncompletePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldName>,
    pub rule: Option<RuleId>,
    pub file: Option<PathBuf>,
    pub reason: IncompleteReason,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UsageSummary {
    /// Distinct logical checked/target content acquisitions; Git snapshot verification rereads are excluded.
    pub checked_files_acquired: usize,
    pub model_requests: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub cost_reported_requests: u32,
    pub cost_missing_requests: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LintDiagnostic {
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub code: LintInvalidCode,
    pub message: String,
}
impl From<LintInvalid> for LintDiagnostic {
    fn from(error: LintInvalid) -> Self {
        Self {
            file: Some(error.file),
            line: error.line,
            column: error.column,
            code: error.code,
            message: error.message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EvidenceListing {
    pub version: String,
    pub config: PathBuf,
    pub scope: ScopeReport,
    pub pieces: Vec<EvidencePiece>,
    pub selections: Vec<SelectionEntry>,
    pub file_counts: Vec<sunstack_evidence::FileCount>,
    pub usage: UsageSummary,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LintReport {
    pub version: String,
    pub config: PathBuf,
    pub scope: ScopeReport,
    pub status: LintStatus,
    pub findings: Vec<Finding>,
    pub results: Vec<RulePieceResult>,
    pub uncertain: Vec<UncertainResult>,
    pub selections: Vec<SelectionEntry>,
    pub file_counts: Vec<sunstack_evidence::FileCount>,
    pub skipped_rules: Vec<RulebookSkippedRule>,
    pub counts: SeverityCounts,
    pub exclusions: ExclusionCounts,
    pub incomplete: Vec<IncompletePart>,
    pub usage: UsageSummary,
    pub evidence_seen: BTreeMap<usize, SharedEvidenceState>,
    pub limits: EffectiveLimits,
    pub diagnostics: Vec<LintDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_listing: Option<EvidenceListing>,
}
impl LintReport {
    pub(crate) fn empty(config: PathBuf, scope: ScopeChoice, limits: EffectiveLimits) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            config,
            scope: ScopeReport {
                kind: scope,
                widened: None,
            },
            status: LintStatus::NothingSelected,
            findings: vec![],
            results: vec![],
            uncertain: vec![],
            selections: vec![],
            file_counts: vec![],
            skipped_rules: vec![],
            counts: SeverityCounts::default(),
            exclusions: ExclusionCounts::default(),
            incomplete: vec![],
            usage: UsageSummary::default(),
            evidence_seen: BTreeMap::new(),
            limits,
            diagnostics: vec![],
            evidence_listing: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PieceOutcome {
    Pass,
    Finding,
    NotApplicable,
    Uncertain,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RulePieceResult {
    pub name: RuleId,
    pub marker: MarkerId,
    pub file: PathBuf,
    pub first_line: usize,
    pub last_line: usize,
    pub outcome: PieceOutcome,
    pub step: Option<StepName>,
    pub path: Vec<StepName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis: Option<Basis>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub answers: NamedAnswerMap,
}

pub fn report_schema() -> schemars::Schema {
    schemars::schema_for!(LintReport)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    Mechanical,
    Asking,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleListing {
    pub rule: RuleId,
    pub marker: MarkerId,
    pub source: SourceLocation,
    pub evidence: EvidenceKindName,
    pub kind: RuleKind,
    pub severity: Severity,
    pub standard: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckResult {
    Valid { rules: Vec<CheckListing> },
    Invalid { diagnostics: Vec<LintDiagnostic> },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SelectionEntry {
    pub kind: EvidenceKindName,
    #[serde(rename = "where")]
    pub location: PieceLocation,
    pub first_line: String,
    pub outcome: sunstack_rules::SelectionOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckListing {
    Rule(RuleListing),
    Kind(KindListing),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindListing {
    pub kind: EvidenceKindName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<sunstack_evidence::SelectionWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<sunstack_evidence::ContainingKinds>,
}
