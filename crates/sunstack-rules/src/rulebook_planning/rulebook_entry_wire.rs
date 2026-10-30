use crate::{DecisionGraph, DecisionGraphWire, RuleId, Severity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_decisions::NamedQuestionMap;
use sunstack_evidence::{EvidenceKindName, MarkerId};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleEntryWire {
    pub name: RuleId,
    pub evidence: EvidenceKindName,
    #[serde(default)]
    pub questions: NamedQuestionMap,
    pub decision: DecisionGraphWire,
    #[serde(default)]
    pub severity: Severity,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RulebookPair {
    pub markdown: PathBuf,
    pub rules: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulebookSentence {
    pub marker: MarkerId,
    pub rulebook: PathBuf,
    pub line: usize,
    pub standard: String,
}
#[derive(Debug, Clone)]
pub struct RulebookRule {
    pub name: RuleId,
    pub sentence: RulebookSentence,
    pub kind: EvidenceKindName,
    pub questions: NamedQuestionMap,
    pub decision: DecisionGraph,
    pub severity: Severity,
}
