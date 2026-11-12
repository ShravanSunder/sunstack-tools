use crate::{DecisionGraph, DecisionGraphWire, RuleId, Severity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::NamedQuestionMap;
use sunstack_evidence::{EvidenceKindName, FieldName, MarkerId};

pub type ReferenceMaterial = BTreeMap<String, sunstack_decisions::StructuredJson>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleEntryWire {
    #[serde(
        default,
        deserialize_with = "present_only_when",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "crate::OnlyWhenWire")]
    pub only_when: Option<crate::OnlyWhenWire>,
    pub name: RuleId,
    pub evidence: EvidenceKindName,
    #[serde(default)]
    pub questions: NamedQuestionMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reference: ReferenceMaterial,
    pub decision: DecisionGraphWire,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<FieldName>,
}

fn present_only_when<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::OnlyWhenWire>, D::Error> {
    crate::OnlyWhenWire::deserialize(deserializer).map(Some)
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
    #[serde(with = "sunstack_evidence::cli_path")]
    #[schemars(with = "PathBuf")]
    pub rulebook: sunstack_evidence::DefinitionOrigin,
    pub line: usize,
    pub standard: String,
}
#[derive(Debug, Clone)]
pub struct RulebookRule {
    pub reference: ReferenceMaterial,
    pub gate: Option<crate::OnlyWhenGate>,
    pub name: RuleId,
    pub sentence: RulebookSentence,
    pub kind: EvidenceKindName,
    pub questions: NamedQuestionMap,
    pub decision: DecisionGraph,
    pub severity: Severity,
    pub related: Vec<FieldName>,
}
