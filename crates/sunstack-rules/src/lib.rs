//! Admit authored definitions, plan evidence-backed requests, and reduce native answers in authored order.
#[path = "rulebook_planning/decision_graph_contracts.rs"]
pub mod decision_graph_contracts;
#[path = "rule_definitions/definition_admission.rs"]
pub mod definition_admission;
#[path = "rule_definitions/definition_contracts.rs"]
pub mod definition_contracts;
#[path = "rule_definitions/invocation_contracts.rs"]
pub mod invocation_contracts;
#[path = "rule_definitions/rule_diagnostics.rs"]
pub mod rule_diagnostics;
pub use decision_graph_contracts::*;
#[path = "rulebook_planning/rulebook_entry_wire.rs"]
pub mod rulebook_entry_wire;
pub use rulebook_entry_wire::*;
#[path = "rulebook_planning/rulebook_admission.rs"]
pub mod rulebook_admission;
pub use rulebook_admission::*;
#[path = "rulebook_planning/decision_walking.rs"]
pub mod decision_walking;
pub use decision_walking::*;
#[path = "rulebook_planning/rulebook_assembly.rs"]
pub mod rulebook_assembly;
pub use rulebook_assembly::*;
#[path = "rulebook_planning/rulebook_run.rs"]
pub mod rulebook_run;
pub use rulebook_run::*;
#[cfg(test)]
#[path = "rulebook_planning/rulebook_test_support.rs"]
mod rulebook_test_support;
#[path = "rule_definitions/strict_decoding.rs"]
mod strict_decoding;
pub use definition_admission::*;
pub use definition_contracts::*;
pub use invocation_contracts::*;
pub use rule_diagnostics::*;
#[path = "rulebook_planning/rule_check_contracts.rs"]
pub mod rule_check_contracts;
pub use rule_check_contracts::*;
#[path = "rulebook_planning/rule_checking.rs"]
pub mod rule_checking;
pub use rule_checking::{check_rules, decode_check_rules};
pub use strict_decoding::decode_rule_invocation;

#[path = "rule_definitions/definition_loading.rs"]
mod definition_loading;
#[path = "rule_definitions/input_binding.rs"]
pub mod input_binding;
#[path = "rule_definitions/invocation_admission.rs"]
mod invocation_admission;
#[path = "rule_evaluation/report_contracts.rs"]
pub mod report_contracts;
#[path = "rule_definitions/rule_identity.rs"]
pub mod rule_identity;
pub use definition_loading::{LocatedRuleDefinition, ValidatedRuleFile};
pub use input_binding::{AdmittedSource, SourceId};
pub use report_contracts::*;
pub use rule_identity::*;
#[path = "rule_evaluation/rule_evaluation_contracts.rs"]
pub mod rule_evaluation_contracts;
pub use rule_evaluation_contracts::*;
#[path = "rule_evaluation/mechanical_contracts.rs"]
pub mod mechanical_contracts;
pub use mechanical_contracts::*;
#[path = "rule_evaluation/local_check_evaluation.rs"]
mod local_check_evaluation;
#[path = "rule_evaluation/rule_backend_factory.rs"]
pub mod rule_backend_factory;
pub use rule_backend_factory::RuleBackendFactory;
#[path = "rule_evaluation/verdict_evaluation.rs"]
pub mod verdict_evaluation;
pub use verdict_evaluation::*;

#[path = "rule_evaluation/request_state.rs"]
mod request_state;
#[path = "rule_evaluation/rule_execution.rs"]
pub mod rule_execution;
pub use rule_execution::*;
#[path = "rule_evaluation/rule_policy.rs"]
mod rule_policy;
pub use rule_policy::*;
#[path = "rule_evaluation/applicability_evaluation.rs"]
mod applicability_evaluation;
mod request_planning;

#[path = "execution_preparation/preparation_contracts.rs"]
pub mod preparation_contracts;
pub use preparation_contracts::*;
#[path = "execution_preparation/rule_preparation.rs"]
pub mod rule_preparation;
pub use rule_preparation::*;

#[path = "evidence_selection/selection_contracts.rs"]
pub mod selection_contracts;
pub use selection_contracts::*;

#[path = "rule_evaluation/model_request_dispatch.rs"]
pub mod model_request_dispatch;
#[path = "rule_evaluation/rule_coverage_reporting.rs"]
mod rule_coverage_reporting;
pub use model_request_dispatch::*;
#[path = "evidence_selection/selection_run.rs"]
pub mod selection_run;
pub use selection_run::*;
