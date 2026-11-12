use crate::{
    EffectiveLimits, IdentifiedResult, LintInvalid, LintInvalidCode, RulebookFile, SavedRequest,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{NamedAnswerMap, SharedEvidenceState};
use sunstack_evidence::{
    EvidenceKindName, EvidencePiece, FieldName, MarkerId, PieceLocation, SourceSpan,
};
use sunstack_rules::{Anchor, Basis, RuleId, RulebookSkippedRule, Severity, StepName};
#[cfg_attr(feature = "typescript", derive(specta::Type))]
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
    #[serde(default)]
    pub root: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub scope: ScopeChoice,
    pub max_warnings: Option<u32>,
    #[serde(default)]
    pub evidence_listing: Option<Option<sunstack_evidence::EvidenceKindName>>,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum WidenedReason {
    ConfigChanged,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScopeReport {
    #[serde(flatten)]
    pub kind: ScopeChoice,
    pub widened: Option<WidenedReason>,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum LintStatus {
    Prepared,
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
            Self::Prepared => 6,
        }
    }
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum SettingOrigin {
    Authored,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Finding {
    pub id: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator_omitted: Option<sunstack_rules::LocatorOmission>,
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
    pub related: Vec<RelatedPlace>,
    pub anchor: Anchor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RelatedPlace {
    pub label: String,
    /// A checked file, or the CLI path of the standard's rulebook definition.
    pub file: PathBuf,
    pub first_line: usize,
    pub last_line: usize,
    /// Set when this place is a definition (the standard); never CLI-serialized.
    #[serde(skip)]
    #[schemars(skip)]
    pub definition: Option<sunstack_evidence::DefinitionOrigin>,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct JudgmentSummary {
    pub answers: NamedAnswerMap,
    pub value: Option<f64>,
    pub path_taken: Vec<StepName>,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UncertainResult {
    pub id: usize,
    pub rule: RuleId,
    pub marker: MarkerId,
    #[serde(rename = "where")]
    pub location: PieceLocation,
    pub step: StepName,
    pub state: SharedEvidenceState,
    pub answers: NamedAnswerMap,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SeverityCounts {
    pub error: u32,
    pub warn: u32,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExclusionCounts {
    pub off: u32,
    pub ignored: u32,
    pub not_applicable: u32,
    pub outside_change_scope: u32,
    pub zero_matches: u32,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[derive(schemars::JsonSchema)]
pub enum IncompleteReason {
    ChoiceOptionsExceeded {
        options: usize,
        limit: usize,
    },
    Filter {
        filter: sunstack_rules::FilterName,
        reason: sunstack_rules::FilterIncomplete,
    },
    NoFilterFolder {
        filter: sunstack_rules::FilterName,
    },
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
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IncompletePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldName>,
    pub rule: Option<RuleId>,
    pub file: Option<PathBuf>,
    pub reason: IncompleteReason,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
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
    /// Source that stopped execution, distinct from its owning configuration file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub code: LintInvalidCode,
    pub message: String,
    /// The definition this diagnostic belongs to; never part of CLI serialization.
    #[serde(skip)]
    #[schemars(skip)]
    pub definition: Option<sunstack_evidence::DefinitionOrigin>,
    /// The configuration field path, when known; never part of CLI serialization.
    #[serde(skip)]
    #[schemars(skip)]
    pub json_path: Vec<String>,
}
impl From<LintInvalid> for LintDiagnostic {
    fn from(error: LintInvalid) -> Self {
        Self {
            file: Some(error.file),
            path: None,
            line: error.line,
            column: error.column,
            code: error.code,
            message: error.message,
            definition: error.definition.map(|definition| *definition),
            json_path: error.json_path,
        }
    }
}
impl LintDiagnostic {
    pub(crate) fn location_path(&self) -> Option<&std::path::Path> {
        self.path.as_deref().or(self.file.as_deref())
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
    #[serde(default, flatten)]
    pub preparation: Option<PreparedReport>,
    pub piece_evidence: BTreeMap<usize, crate::SavedPieceEvidence>,
    pub requests: Vec<SavedRequest>,
    pub rulebook_files: Vec<RulebookFile>,
    pub filter_answers: Vec<IdentifiedResult<sunstack_rules::FilterAnswer>>,
    pub filter_skips: Vec<IdentifiedResult<sunstack_rules::FilterSkip>>,
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
            preparation: None,
            piece_evidence: BTreeMap::new(),
            requests: vec![],
            rulebook_files: vec![],
            filter_answers: vec![],
            filter_skips: vec![],
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
pub struct PreparedReport {
    pub mode: PrepareMode,
    pub needs_answer: Vec<NeedsSelectionAnswer>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepareMode {
    Prepare,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NeedsSelectionAnswer {
    pub kind: EvidenceKindName,
    pub file: PathBuf,
    pub waits_on: SelectionDependency,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SelectionDependency {
    pub selection_kind: EvidenceKindName,
    pub request_id: usize,
}
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PieceOutcome {
    Pass,
    Finding,
    NotApplicable,
    Uncertain,
    Incomplete,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RulePieceResult {
    pub text_sha256: Option<String>,
    pub id: usize,
    pub request_ids: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<sunstack_rules::RulebookIncompleteReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator_omitted: Option<sunstack_rules::LocatorOmission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<sunstack_rules::NotApplicableReason>,
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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub cut: Vec<FieldName>,
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

#[cfg_attr(feature = "typescript", derive(specta::Type))]
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
