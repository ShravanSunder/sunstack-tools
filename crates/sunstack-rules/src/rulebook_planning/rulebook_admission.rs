use crate::{
    BinaryRoutes, DecisionGraph, DecisionGraphWire, DecisionMechanicalTool, DecisionRoute,
    DecisionStep, DecisionStepWire, RuleEntryWire,
};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_decisions::{
    JudgmentQuestion, NamedQuestionMap, QuestionName, StructuredEntry, StructuredJson,
};
use sunstack_evidence::{
    ConfigPath, EvidenceKindDefinition, EvidenceKindName, EvidenceProject, FieldName, FieldShape,
    FieldSource, MatchSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    pub field_path: ConfigPath,
    pub message: String,
}
fn refusal(path: impl Into<ConfigPath>, message: impl Into<String>) -> AdmissionRefusal {
    AdmissionRefusal {
        field_path: path.into(),
        message: message.into(),
    }
}

mod evidence_kind_admission;
pub use evidence_kind_admission::admit_evidence_kinds;
pub(crate) const RESERVED_STATE_KEYS: [&str; 5] = [
    "standard",
    "where",
    "cut-fields",
    "unavailable-fields",
    "reference",
];

mod decision_graph_admission;
mod rule_entry_admission;
mod selection_admission;

pub(crate) use decision_graph_admission::admit_graph;
use decision_graph_admission::validate_thresholds;
pub use rule_entry_admission::admit_rule_entry;
use rule_entry_admission::field_references_valid;
pub(crate) use rule_entry_admission::{
    admit_questions, question_field_references, question_state_paths,
};
pub use selection_admission::{admit_selections, normalized_answer};

#[cfg(test)]
#[path = "rulebook_admission_test.rs"]
mod tests;
