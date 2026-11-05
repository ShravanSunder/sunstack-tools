//! Admit authored definitions, plan evidence-backed requests, and reduce native answers in authored order.
pub mod evidence_selection;
pub mod execution_preparation;
mod request_planning;
pub mod rule_definitions;
pub mod rule_evaluation;
pub mod rulebook_planning;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use evidence_selection::selection_contracts::*;
pub use evidence_selection::selection_run::*;
pub use execution_preparation::preparation_contracts::*;
pub use execution_preparation::rule_preparation::*;
pub use rule_definitions::definition_admission::*;
pub use rule_definitions::definition_contracts::*;
pub use rule_definitions::definition_loading::{LocatedRuleDefinition, ValidatedRuleFile};
pub use rule_definitions::input_binding::{AdmittedSource, SourceId};
pub use rule_definitions::invocation_contracts::*;
pub use rule_definitions::rule_diagnostics::*;
pub use rule_definitions::rule_identity::*;
pub use rule_definitions::strict_decoding::decode_rule_invocation;
pub use rule_evaluation::mechanical_contracts::*;
pub use rule_evaluation::model_request_dispatch::*;
pub use rule_evaluation::report_contracts::*;
pub use rule_evaluation::rule_backend_factory::RuleBackendFactory;
pub use rule_evaluation::rule_evaluation_contracts::*;
pub use rule_evaluation::rule_execution::*;
pub use rule_evaluation::rule_policy::*;
pub use rule_evaluation::verdict_evaluation::*;
pub use rulebook_planning::decision_graph_contracts::*;
pub use rulebook_planning::decision_walking::*;
pub use rulebook_planning::rule_check_contracts::*;
pub use rulebook_planning::rule_checking::{check_rules, decode_check_rules};
pub use rulebook_planning::rulebook_admission::*;
pub use rulebook_planning::rulebook_assembly::*;
pub use rulebook_planning::rulebook_entry_wire::*;
pub use rulebook_planning::rulebook_run::*;
