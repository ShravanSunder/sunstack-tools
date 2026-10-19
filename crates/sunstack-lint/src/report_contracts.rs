use crate::{
    EffectiveLimits, EntryName, LintInvalid, LintInvalidCode, RuleSelector, SeveritySetting,
    policy::{EffectiveSetting, SettingOrigin},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{JudgmentAnswer, QuestionName, SharedEvidenceState};
use sunstack_evidence::SourceSpan;
use sunstack_rules::{Severity, VerdictBasis, VerdictResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WidenedReason {
    ConfigChanged,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeReport {
    #[serde(flatten)]
    pub kind: ScopeChoice,
    pub widened: Option<WidenedReason>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LintStatus {
    Clean,
    NothingSelected,
    Findings,
    Incomplete,
    Invalid,
    Cancelled,
}
impl LintStatus {
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Clean | Self::NothingSelected => 0,
            Self::Findings => 1,
            Self::Invalid => 2,
            Self::Incomplete => 3,
            Self::Cancelled => 4,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule: RuleSelector,
    pub file: PathBuf,
    pub span: Option<SourceSpan>,
    pub severity: Severity,
    pub severity_origin: SettingOrigin,
    pub message: String,
    pub judgment: Option<JudgmentSummary>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub error: u32,
    pub warn: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExclusionCounts {
    pub off: u32,
    pub ignored: u32,
    pub not_applicable: u32,
    pub outside_change_scope: u32,
    pub zero_matches: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IncompleteReason {
    SelectionFailed,
    EvaluationFailed,
    NotStarted,
    PreparationFailed,
    CoveragePartial,
    CoverageNone,
    ProviderFailure,
    RequestLimitReached,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompletePart {
    pub rule: Option<RuleSelector>,
    pub file: Option<PathBuf>,
    pub reason: IncompleteReason,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageSummary {
    pub model_requests: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub cost_reported_requests: u32,
    pub cost_missing_requests: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentSummary {
    pub request_index: usize,
    pub basis: VerdictBasis,
    pub result: VerdictResult,
    pub value: f64,
    pub threshold: f64,
    pub answers: Vec<ResolvedAnswer>,
}
/// Resolved values are mandatory; a reference alone cannot become a report answer.
///
/// ```compile_fail
/// use sunstack_lint::ResolvedAnswer;
/// use sunstack_decisions::QuestionName;
/// use sunstack_rules::JudgmentReference;
/// let question = QuestionName::new("q000001").unwrap();
/// let reference = JudgmentReference { request_index: 0, question_name: question.clone() };
/// let answer = ResolvedAnswer { question, answer: reference };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAnswer {
    pub question: QuestionName,
    pub answer: JudgmentAnswer,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintReport {
    pub version: String,
    pub config: PathBuf,
    pub scope: ScopeReport,
    pub status: LintStatus,
    pub findings: Vec<Finding>,
    pub counts: SeverityCounts,
    pub exclusions: ExclusionCounts,
    pub incomplete: Vec<IncompletePart>,
    pub usage: UsageSummary,
    pub evidence_seen: BTreeMap<usize, SharedEvidenceState>,
    pub limits: EffectiveLimits,
    pub diagnostics: Vec<LintDiagnostic>,
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
            counts: SeverityCounts::default(),
            exclusions: ExclusionCounts::default(),
            incomplete: vec![],
            usage: UsageSummary::default(),
            evidence_seen: BTreeMap::new(),
            limits,
            diagnostics: vec![],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    Mechanical,
    Judged,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleListing {
    pub rule: RuleSelector,
    pub source: SourceLocation,
    pub kind: RuleKind,
    pub authored: Severity,
    pub configured: SeveritySetting,
    pub for_file: Option<EffectiveSetting>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckResult {
    Valid { rules: Vec<RuleListing> },
    Invalid { diagnostics: Vec<LintDiagnostic> },
}
#[derive(Debug, Clone)]
pub(crate) struct CheckedEntry {
    pub name: EntryName,
    pub path: PathBuf,
    pub input: sunstack_evidence::SourceName,
    pub rules: Vec<sunstack_rules::PrintedRule>,
}
