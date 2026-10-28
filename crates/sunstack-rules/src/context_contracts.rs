use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sunstack_evidence::*;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateContextView {
    pub context: CandidateContext,
    pub labels: BTreeMap<PieceId, RelatednessLabel>,
    pub pages: Vec<ContextPage>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextPage {
    pub number: usize,
    pub of: usize,
    pub revision: String,
    pub holds: Vec<PieceId>,
    pub rendered_chars: usize,
    pub content_identity: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelatednessLabel {
    Related { basis: RelatedBasis },
    ProbablyRelated { fact: RelationshipFactRef },
    Uncertain { reasons: Vec<UncertaintyReason> },
    Unrelated { judged_by: AnswerId },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelatedBasis {
    InputUnit { unit: SourceUnitKey },
    Judged { answer: AnswerId },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelationshipFactRef {
    pub piece_id: PieceId,
    pub position: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UncertaintyReason {
    Graph { reason: RelationshipUncertainty },
    Contradicted { reason: RelationshipContradiction },
}
