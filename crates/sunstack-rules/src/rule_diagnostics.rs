use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleDiagnosticCode {
    MalformedJson,
    UnknownField,
    DuplicateField,
    UnsupportedSchemaVersion,
    DuplicateRuleId,
    DuplicateRuleSource,
    DuplicateRuleSet,
    EmptyRuleFile,
    ForbiddenMarkerHost,
    SpacedHeadingMarker,
    DuplicateMarker,
    MissingRuleEntry,
    OrphanedRuleEntry,
    InvalidRuleSourceKey,
    QuestionTooLarge,
    InvalidThreshold,
    InvalidBand,
    InvalidComposite,
    InvalidMechanicalCheck,
    InvalidTarget,
    UndeclaredInput,
    InputKindMismatch,
    InvalidRoute,
    InvalidWiring,
    UnboundInput,
    DuplicateBinding,
    ExtraBinding,
    BindingKindMismatch,
    ConditionUnresolved,
    DecisionFailed,
    RouterFailed,
    InvalidConfiguration,
    RuleExceedsBudget,
    RequestLimit,
    UnexpectedExecutionError,
    CallerAborted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleDiagnosticOrigin {
    RuleSet,
    RuleFile,
    Router,
    InputBinding,
    Applicability,
    Request,
    Execution,
    Invocation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleDiagnosticLocation {
    None,
    Definition {
        file: PathBuf,
        line: usize,
        column: usize,
        field_path: Vec<String>,
    },
    Field {
        field_path: Vec<String>,
    },
    Source {
        source: sunstack_evidence::SourceRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<sunstack_evidence::SourceSpan>,
    },
    Record {
        source: sunstack_evidence::SourceRef,
        record_id: sunstack_evidence::RecordId,
    },
    Answer {
        request_index: usize,
        question_name: sunstack_decisions::QuestionName,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{code:?}: {guidance}")]
#[serde(deny_unknown_fields)]
pub struct RuleDiagnostic {
    pub origin: RuleDiagnosticOrigin,
    pub code: RuleDiagnosticCode,
    pub location: RuleDiagnosticLocation,
    pub guidance: String,
    pub retry_advice: sunstack_decisions::RetryAdvice,
}
impl RuleDiagnostic {
    pub fn new(code: RuleDiagnosticCode) -> Self {
        use RuleDiagnosticCode::*;
        let (origin, guidance) = match code {
            ForbiddenMarkerHost => (
                RuleDiagnosticOrigin::RuleFile,
                "Move the marker to a heading of level 2–6, a paragraph or a list item, outside block quotes, tables and HTML.",
            ),
            SpacedHeadingMarker => (
                RuleDiagnosticOrigin::RuleFile,
                "Remove the space before the heading marker.",
            ),
            DuplicateMarker => (
                RuleDiagnosticOrigin::RuleFile,
                "Use each marker ID at most once in a Markdown source.",
            ),
            QuestionTooLarge => (
                RuleDiagnosticOrigin::RuleFile,
                "Mark a narrower host, such as a subsection, a paragraph or a list item.",
            ),
            MissingRuleEntry => (
                RuleDiagnosticOrigin::RuleFile,
                "Add a rules entry for this marker ID.",
            ),
            OrphanedRuleEntry => (
                RuleDiagnosticOrigin::RuleFile,
                "Add the corresponding marker or remove the orphaned rules entry.",
            ),
            ConditionUnresolved => (
                RuleDiagnosticOrigin::Applicability,
                "Supply the missing field or complete the declared bounded scan.",
            ),
            DecisionFailed | RouterFailed => (
                RuleDiagnosticOrigin::Applicability,
                "Inspect the referenced request failure; no applicability answer was fabricated.",
            ),
            UnboundInput | DuplicateBinding | ExtraBinding | BindingKindMismatch => (
                RuleDiagnosticOrigin::InputBinding,
                "Bind each declared input exactly once with its declared kind.",
            ),
            InvalidThreshold => (
                RuleDiagnosticOrigin::RuleFile,
                "Use a finite threshold above zero and at most one.",
            ),
            InvalidBand => (
                RuleDiagnosticOrigin::RuleFile,
                "Use a band below the threshold and only on an error rule.",
            ),
            InvalidComposite => (
                RuleDiagnosticOrigin::RuleFile,
                "Use this rule's noul or multi-level score questions with positive weights summing to one.",
            ),
            InvalidMechanicalCheck => (
                RuleDiagnosticOrigin::RuleFile,
                "Use exactly one supported expect predicate or a valid Markdown ast-grep rule.",
            ),
            RuleExceedsBudget | RequestLimit => (
                RuleDiagnosticOrigin::Request,
                "Keep required rule evidence intact; reduce authored scope or raise the finite invocation budget.",
            ),
            CallerAborted => (
                RuleDiagnosticOrigin::Execution,
                "The caller cancelled; remote inference may already have happened.",
            ),
            UnexpectedExecutionError => (
                RuleDiagnosticOrigin::Execution,
                "An owned execution task failed before producing its outcome.",
            ),
            InvalidConfiguration => (
                RuleDiagnosticOrigin::Invocation,
                "Repair finite execution settings before acquiring evidence.",
            ),
            _ => (
                RuleDiagnosticOrigin::RuleSet,
                "Repair the declared definition or reference at the indicated safe location.",
            ),
        };
        Self {
            origin,
            code,
            location: RuleDiagnosticLocation::None,
            guidance: guidance.into(),
            retry_advice: sunstack_decisions::RetryAdvice::RepairRequest,
        }
    }
    pub fn definition(
        mut self,
        file: &std::path::Path,
        line: usize,
        column: usize,
        field_path: Vec<String>,
    ) -> Self {
        self.location = RuleDiagnosticLocation::Definition {
            file: file.into(),
            line,
            column,
            field_path: field_path.into_iter().take(32).collect(),
        };
        self
    }
}
/// Wrapping preserves the boundary owner's closed diagnostic and safe message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ReportDiagnostic {
    Rule(Box<RuleDiagnostic>),
    Evidence(Box<sunstack_evidence::Diagnostic>),
    Decision(Box<sunstack_decisions::DecisionDiagnostic>),
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticList {
    pub items: Vec<ReportDiagnostic>,
    pub omitted_count: usize,
}
impl DiagnosticList {
    pub fn push(&mut self, diagnostic: ReportDiagnostic) {
        if self.items.len() < 100 {
            self.items.push(diagnostic)
        } else {
            self.omitted_count += 1
        }
    }
    pub fn rule(code: RuleDiagnosticCode) -> Self {
        RuleDiagnostic::new(code).into()
    }
    pub fn extend(&mut self, other: Self) {
        for item in other.items {
            self.push(item)
        }
        self.omitted_count += other.omitted_count;
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.omitted_count == 0
    }
}
impl From<RuleDiagnostic> for DiagnosticList {
    fn from(value: RuleDiagnostic) -> Self {
        Self {
            items: vec![ReportDiagnostic::Rule(Box::new(value))],
            omitted_count: 0,
        }
    }
}
impl From<sunstack_evidence::DiagnosticList> for DiagnosticList {
    fn from(value: sunstack_evidence::DiagnosticList) -> Self {
        Self {
            items: value
                .items
                .into_iter()
                .map(|diagnostic| ReportDiagnostic::Evidence(Box::new(diagnostic)))
                .collect(),
            omitted_count: value.omitted_count,
        }
    }
}
impl From<sunstack_decisions::DiagnosticList> for DiagnosticList {
    fn from(value: sunstack_decisions::DiagnosticList) -> Self {
        Self {
            items: value
                .items
                .into_iter()
                .map(|diagnostic| ReportDiagnostic::Decision(Box::new(diagnostic)))
                .collect(),
            omitted_count: value.omitted_count,
        }
    }
}
