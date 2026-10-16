use crate::{definition_contracts::*, invocation_contracts::*, rule_diagnostics::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{ChoiceLabel, DecisionRunOutcome, QuestionName};
use sunstack_evidence::{CaptureCoverage, ReturnedCursor, SourceName, TargetLocation};
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvocationMeasurements {
    pub files_considered: usize,
    pub files_read: usize,
    pub files_parsed: usize,
    pub transcript_bytes_read: u64,
    pub transcript_bytes_parsed: u64,
    pub bytes_scanned: usize,
    pub bytes_included: usize,
    pub rules_selected: usize,
    pub rules_skipped: usize,
    pub rules_failed: usize,
    pub applicability_entries_evaluated: usize,
    pub local_elapsed_ms: u64,
    pub provider_elapsed_ms: u64,
    pub model_requests: usize,
    pub inference_attempts: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntryOwner {
    Router {
        group: usize,
    },
    RuleSet {
        path: PathBuf,
        group: usize,
    },
    Rule {
        path: PathBuf,
        group: usize,
        rule_id: RuleId,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntryKey {
    pub owner: EntryOwner,
    pub position: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleOccurrenceKey {
    pub rule_set: PathBuf,
    pub group: usize,
    pub rule_id: RuleId,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GroupBinding {
    Bound,
    Routed { router_group: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupResult {
    pub rule_set: PathBuf,
    pub position: usize,
    pub binding: GroupBinding,
    pub applicability: ApplicabilityOutcome,
    pub entries: Vec<EvaluatedEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotEvaluatedReason {
    Cancelled,
    InvocationRefused,
    EarlierEntryStopped,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApplicabilityOutcome {
    Applicable,
    NotApplicable { entry: EntryKey, basis: EntryBasis },
    ApplicabilityFailed { entry: EntryKey, cause: Cause },
    NotEvaluated { reason: NotEvaluatedReason },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionBasis {
    PayloadFieldEquals {
        input: SourceName,
        pointer: String,
    },
    PayloadFieldIn {
        input: SourceName,
        pointer: String,
    },
    PayloadFieldPresent {
        input: SourceName,
        pointer: String,
    },
    OriginFieldEquals {
        field: OriginField,
    },
    PatternPresent {
        input: SourceName,
        pattern_kind: PatternKind,
        max_scan_bytes: usize,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    Literal,
    Regex,
    Ast,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntryBasis {
    Condition {
        test: ConditionBasis,
    },
    Decision {
        threshold: f64,
        answer: JudgmentReference,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvaluatedEntry {
    pub key: EntryKey,
    pub result: EntryResult,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntryResult {
    Condition {
        outcome: ConditionOutcome,
        basis: ConditionBasis,
    },
    Decision {
        threshold: f64,
        outcome: DecisionEntryOutcome,
    },
    NotEvaluated {
        reason: NotEvaluatedReason,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionOutcome {
    Passed,
    DidNotPass,
    Unresolved { diagnostic: ReportDiagnostic },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionEntryOutcome {
    Passed { answer: JudgmentReference },
    DidNotPass { answer: JudgmentReference },
    Failed { cause: Cause },
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cause {
    Request {
        request_index: usize,
    },
    Evidence {
        diagnostics: DiagnosticList,
    },
    Budget {
        diagnostics: DiagnosticList,
    },
    Gate {
        entry: EntryKey,
    },
    Condition {
        entry: EntryKey,
        diagnostic: ReportDiagnostic,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JudgmentReference {
    pub request_index: usize,
    pub question_name: QuestionName,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleOccurrenceResult {
    Judged {
        key: RuleOccurrenceKey,
        judgment: RuleJudgment,
    },
    NotApplicable {
        key: RuleOccurrenceKey,
        stopped_at: EntryKey,
        basis: EntryBasis,
    },
    OutsideChangeScope {
        key: RuleOccurrenceKey,
        reason: String,
    },
    SelectionFailed {
        key: RuleOccurrenceKey,
        diagnostics: DiagnosticList,
    },
    EvaluationFailed {
        key: RuleOccurrenceKey,
        cause: Cause,
    },
    Cancelled {
        key: RuleOccurrenceKey,
    },
    NotStarted {
        key: RuleOccurrenceKey,
        reason: NotStartedReason,
    },
}
impl RuleOccurrenceResult {
    pub fn key(&self) -> &RuleOccurrenceKey {
        match self {
            Self::Judged { key, .. }
            | Self::NotApplicable { key, .. }
            | Self::OutsideChangeScope { key, .. }
            | Self::SelectionFailed { key, .. }
            | Self::EvaluationFailed { key, .. }
            | Self::Cancelled { key }
            | Self::NotStarted { key, .. } => key,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotStartedReason {
    CallerCancelled,
    RequestLimit,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleJudgment {
    pub key: RuleOccurrenceKey,
    pub verdict: VerdictOutcome,
    pub declared_severity: Severity,
    pub answers: BTreeMap<QuestionName, JudgmentReference>,
    pub target: TargetLocation,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerdictOutcome {
    pub basis: VerdictBasis,
    pub value: f64,
    pub threshold: f64,
    pub result: VerdictResult,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VerdictBasis {
    Noul { answer: JudgmentReference },
    Composite { dimensions: Vec<DimensionResult> },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DimensionResult {
    pub question: QuestionName,
    pub weight: f64,
    pub normalized_value: f64,
    pub answer: JudgmentReference,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VerdictResult {
    Pass,
    Fail { activation: Activation },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Activation {
    Nudge,
    Block,
    NudgeAboveBand { band: f64 },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouterOutcome {
    Routed {
        label: ChoiceLabel,
        confidence: f64,
        answer: JudgmentReference,
    },
    UncertainIntent {
        confidence: f64,
        floor: f64,
        distribution: BTreeMap<ChoiceLabel, f64>,
        answer: JudgmentReference,
    },
    NoRoute {
        label: ChoiceLabel,
        confidence: f64,
        answer: JudgmentReference,
    },
    RouterFailed {
        cause: Cause,
    },
    NotEvaluated {
        reason: RouterNotEvaluated,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouterNotEvaluated {
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouterGroupResult {
    pub position: usize,
    pub outcome: RouterOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RoutingReport {
    RuleSets,
    Router { groups: Vec<RouterGroupResult> },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestOwner {
    Router { group: usize },
    RuleSetGroup { rule_set: PathBuf, group: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestMode {
    Rules,
    Speculative,
    StagedGate,
    Split { part: usize, of: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuestionOwner {
    Router { group: usize },
    RuleSetDecision { entry: EntryKey },
    RuleDecision { rule_id: RuleId, entry: EntryKey },
    Verdict { rule_id: RuleId },
    Informational { rule_id: RuleId, name: QuestionName },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestionBinding {
    pub request_name: QuestionName,
    pub owner: QuestionOwner,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestStatus {
    Sent { outcome: DecisionRunOutcome },
    NotSent { reason: NotSentReason },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NotSentReason {
    GateStopped { entry: EntryKey },
    Cancelled,
    BudgetRefused { diagnostics: DiagnosticList },
    EvidenceRefused { diagnostics: DiagnosticList },
    RequestLimit,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscardedAnswer {
    pub question_name: QuestionName,
    pub reason: DiscardReason,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscardReason {
    OwnerNotApplicable { entry: EntryKey },
    NotRouted,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestRecord {
    pub index: usize,
    pub owner: RequestOwner,
    pub mode: RequestMode,
    pub questions: Vec<QuestionBinding>,
    pub prerequisites: Vec<EntryKey>,
    pub status: RequestStatus,
    pub discarded: Vec<DiscardedAnswer>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleInvocationReport {
    pub origin: ReportedOrigin,
    pub routing: RoutingReport,
    pub groups: Vec<GroupResult>,
    pub occurrences: Vec<RuleOccurrenceResult>,
    pub requests: Vec<RequestRecord>,
    pub captures: Vec<CaptureCoverage>,
    pub cursors: Vec<ReturnedCursor>,
    pub diagnostics: DiagnosticList,
    pub measurements: InvocationMeasurements,
}
impl RuleInvocationReport {
    pub fn is_complete(&self) -> bool {
        self.occurrences.iter().all(|result| {
            matches!(
                result,
                RuleOccurrenceResult::Judged { .. }
                    | RuleOccurrenceResult::NotApplicable { .. }
                    | RuleOccurrenceResult::OutsideChangeScope { .. }
            )
        }) && match &self.routing {
            RoutingReport::RuleSets => true,
            RoutingReport::Router { groups } => groups.iter().all(|group| {
                matches!(
                    group.outcome,
                    RouterOutcome::Routed { .. }
                        | RouterOutcome::UncertainIntent { .. }
                        | RouterOutcome::NoRoute { .. }
                )
            }),
        }
    }
    pub fn answer(
        &self,
        reference: &JudgmentReference,
    ) -> Option<&sunstack_decisions::JudgmentAnswer> {
        match &self.requests.get(reference.request_index)?.status {
            RequestStatus::Sent {
                outcome: DecisionRunOutcome::Completed { answers, .. },
            } => answers.get(&reference.question_name),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvocationCancellationProgress {
    BeforeReportAdmission,
    ReportAdmitted {
        partial_report: Box<RuleInvocationReport>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleInvocationOutcome {
    Invalid {
        diagnostics: DiagnosticList,
    },
    PreparationFailed {
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
    CompleteReport {
        report: RuleInvocationReport,
    },
    IncompleteReport {
        report: RuleInvocationReport,
    },
    Cancelled {
        progress: InvocationCancellationProgress,
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
}
impl RuleInvocationOutcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Invalid { .. } => 2,
            Self::PreparationFailed { .. } | Self::IncompleteReport { .. } => 3,
            Self::CompleteReport { .. } => 0,
            Self::Cancelled { .. } => 4,
        }
    }
}
