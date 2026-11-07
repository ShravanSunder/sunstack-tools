use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_decisions::{DecisionRunOutcome, ModelName, StructuredJudgmentRequest};
use sunstack_evidence::PieceLocation;
use sunstack_rules::{FilterName, RuleId};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IdentifiedResult<T> {
    pub id: usize,
    pub request_ids: Vec<usize>,
    #[serde(flatten)]
    pub value: T,
}
impl<T> std::ops::Deref for IdentifiedResult<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestOwner {
    Rule { rule: RuleId, piece: PieceLocation },
    Filter { filter: FilterName, folder: PathBuf },
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestNotSentReason {
    Rule {
        reason: sunstack_rules::RulebookIncompleteReason,
    },
    Filter {
        reason: sunstack_rules::FilterIncomplete,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SavedRequestOutcome {
    Run { outcome: DecisionRunOutcome },
    NotSent { reason: RequestNotSentReason },
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SavedRequest {
    pub id: usize,
    pub owner: RequestOwner,
    pub requested_model: ModelName,
    pub resolved_model: Option<ModelName>,
    pub request: StructuredJudgmentRequest,
    #[serde(flatten)]
    pub outcome: SavedRequestOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RulebookFile {
    pub path: PathBuf,
    pub sha256: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SavedPieceEvidence {
    pub piece: sunstack_evidence::EvidencePiece,
    pub matches: std::collections::BTreeMap<
        sunstack_evidence::EvidenceRuleId,
        Vec<sunstack_evidence::MatchRange>,
    >,
}
