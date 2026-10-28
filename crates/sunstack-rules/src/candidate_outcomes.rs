use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sunstack_evidence::DeclarationId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AnswerId(pub DeclarationId);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuestionRef {
    Router {},
    RuleSetGate { position: usize },
    RuleGate { position: usize },
    Verdict {},
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateOutcome {
    Finding {
        path: Vec<AnswerId>,
    },
    Dropped {
        at: QuestionRef,
        answer: AnswerId,
    },
    Uncertain {
        at: QuestionRef,
        answers: Vec<AnswerId>,
    },
    Unresolved {
        at: QuestionRef,
        missing: UnresolvedCause,
    },
    NotProcessed {
        reason: NotProcessedReason,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnresolvedCause {
    EvidenceUnavailable {
        evidence: sunstack_evidence::SourceName,
    },
    EvidenceNotCaptured {
        evidence: sunstack_evidence::SourceName,
    },
    PageMissing {
        pages: Vec<u32>,
    },
    NeedsWholeContext {
        pages: u32,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotProcessedReason {
    Refused,
    LeftOutForPrivacy,
    OverBudget,
    RunStopped,
    EvaluationFailed,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSettlement {
    Passed,
    DidNotPass,
    Uncertain,
    Unresolved,
    NotSent,
}
