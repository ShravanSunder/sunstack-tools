use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sunstack_evidence::{
    CoverageGap, PrivateKind, SourceInputId, SourceOccurrence, UnlocatableReason,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateRunReport {
    pub completeness: RunCompleteness,
    pub candidates: Vec<CandidateEntry>,
    pub dropped_samples: Vec<DroppedSample>,
    pub inputs: Vec<InputReport>,
    pub privacy: PrivacyReport,
    pub usage: RunUsage,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateEntry {
    pub candidate_id: CandidateId,
    pub rule: RuleRef,
    pub occurrence: SourceOccurrence,
    pub outcome: CandidateOutcome,
    pub answers: Vec<AnswerRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceUse>,
    pub privacy_view: PrivacyView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceUse {
    pub input: sunstack_evidence::SourceName,
    pub binding: EvidenceBinding,
    pub content_identity: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnswerRecord {
    pub answer_id: AnswerId,
    pub question: QuestionRef,
    pub page: Option<u32>,
    pub answer: Option<sunstack_decisions::JudgmentAnswer>,
    pub settlement: QuestionSettlement,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DroppedSample {
    pub rule: RuleRef,
    pub pass: SamplePass,
    pub occurrence: SourceOccurrence,
    pub excerpt: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SamplePass {
    Mechanical,
    Narrowing,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputReport {
    pub input_id: SourceInputId,
    pub location: InputLocation,
    pub records: usize,
    pub unfit: Vec<CoverageGap>,
    pub capture: InputCaptureStatus,
    pub discovery: Vec<RuleDiscovery>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InputCaptureStatus {
    Complete,
    Incomplete,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleDiscovery {
    pub rule: RuleRef,
    pub status: RuleDiscoveryStatus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleDiscoveryStatus {
    Completed,
    NotStarted,
    Stopped,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputLocation {
    Located {
        path: std::path::PathBuf,
        format: sunstack_evidence::TranscriptFormatName,
    },
    Unlocatable {
        reason: UnlocatableReason,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRoute {
    Records,
    Samples,
    OutsideEvidence,
    Code,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrivacyCount {
    pub route: PrivacyRoute,
    pub private_kind: PrivateKind,
    pub count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrivacyGrant {
    pub rule: RuleRef,
    pub private_kind: PrivateKind,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrivacyReport {
    pub left_out: Vec<PrivacyCount>,
    pub granted: Vec<PrivacyGrant>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrivacyView {
    pub grants: Vec<PrivateKind>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunUsage {
    pub requests: usize,
    pub attempts: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub wall_time_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunCompleteness {
    NothingFound {},
    Findings {
        count: usize,
        inconclusive: usize,
    },
    Inconclusive {
        uncertain: usize,
        unresolved: usize,
        not_processed: usize,
    },
    Partial {
        stopped_by: CandidateStopReason,
        findings: usize,
        inconclusive: usize,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStopReason {
    UnlocatableInput,
    RequestLimit,
    Cancelled,
    IncompleteInput,
    DiscoveryStopped,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateRunOutcome {
    CandidateReport { report: CandidateRunReport },
    Invalid { diagnostics: DiagnosticList },
    Incomplete { report: CandidateRunReport },
    Cancelled { report: CandidateRunReport },
}
impl CandidateRunOutcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::CandidateReport { .. } => 0,
            Self::Invalid { .. } => 2,
            Self::Incomplete { .. } => 3,
            Self::Cancelled { .. } => 4,
        }
    }
}

pub(crate) fn completeness(
    candidates: &[CandidateEntry],
    inputs: &[InputReport],
    cancelled: bool,
) -> RunCompleteness {
    let mut findings = 0;
    let mut uncertain = 0;
    let mut unresolved = 0;
    let mut not_processed = 0;
    let mut limited = false;
    for entry in candidates {
        match entry.outcome {
            CandidateOutcome::Finding { .. } => findings += 1,
            CandidateOutcome::Dropped { .. } => {}
            CandidateOutcome::Uncertain { .. } => uncertain += 1,
            CandidateOutcome::Unresolved { .. } => unresolved += 1,
            CandidateOutcome::NotProcessed { reason } => {
                not_processed += 1;
                limited |= matches!(reason, NotProcessedReason::RunStopped);
            }
        }
    }
    let inconclusive = uncertain + unresolved + not_processed;
    if cancelled {
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::Cancelled,
            findings,
            inconclusive,
        }
    } else if inputs
        .iter()
        .any(|input| matches!(input.location, InputLocation::Unlocatable { .. }))
    {
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::UnlocatableInput,
            findings,
            inconclusive,
        }
    } else if inputs
        .iter()
        .any(|input| input.capture == InputCaptureStatus::Incomplete)
    {
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::IncompleteInput,
            findings,
            inconclusive,
        }
    } else if inputs.iter().any(|input| {
        input
            .discovery
            .iter()
            .any(|discovery| discovery.status != RuleDiscoveryStatus::Completed)
    }) {
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::DiscoveryStopped,
            findings,
            inconclusive,
        }
    } else if limited {
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::RequestLimit,
            findings,
            inconclusive,
        }
    } else if findings > 0 {
        RunCompleteness::Findings {
            count: findings,
            inconclusive,
        }
    } else if inconclusive > 0 {
        RunCompleteness::Inconclusive {
            uncertain,
            unresolved,
            not_processed,
        }
    } else {
        RunCompleteness::NothingFound {}
    }
}
