use crate::RuleId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_evidence::{MarkdownByteRange, SourceRef};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleEvaluationKey {
    pub rule_set: PathBuf,
    pub group: usize,
    pub rule_id: RuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleOccurrenceTarget {
    Legacy,
    MarkdownUnit {
        source: SourceRef,
        range: MarkdownByteRange,
    },
    CallSite {
        source: SourceRef,
        unit_range: MarkdownByteRange,
        match_range: MarkdownByteRange,
        visible_range: MarkdownByteRange,
    },
    FrontmatterField {
        source: SourceRef,
        field: String,
    },
}

impl crate::RuleOccurrenceKey {
    pub(crate) fn evidence_label(&self) -> String {
        if matches!(self.target.as_ref(), RuleOccurrenceTarget::Legacy) {
            format!("rule:{}", self.rule_id.as_str())
        } else {
            // Each request already owns one rule-set/binding-group scope.
            // Target labels need its logical source/ranges, not the host's
            // absolute definition path. Settlement uses the full typed key.
            format!("rule:{}:{:?}", self.rule_id.as_str(), self.target)
        }
    }
}
