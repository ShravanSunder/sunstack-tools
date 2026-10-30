//! Admit authored definitions, plan evidence-backed requests, and reduce native answers in authored order.
pub mod decision_graph_contracts;
pub mod definition_admission;
pub mod definition_contracts;
pub mod invocation_contracts;
pub mod rule_diagnostics;
pub use decision_graph_contracts::*;
pub mod rulebook_entry_wire;
pub use rulebook_entry_wire::*;
pub mod rulebook_admission;
pub use rulebook_admission::*;
pub mod decision_walking;
pub use decision_walking::*;
pub mod rulebook_assembly;
pub use rulebook_assembly::*;
pub mod rulebook_run;
pub use rulebook_run::*;
#[cfg(test)]
mod rulebook_test_support;
mod strict_decoding;
pub use definition_admission::*;
pub use definition_contracts::*;
pub use invocation_contracts::*;
pub use rule_diagnostics::*;
pub mod rule_check_contracts;
pub use rule_check_contracts::*;
pub mod rule_checking;
pub use rule_checking::{check_rules, decode_check_rules};
pub use strict_decoding::decode_rule_invocation;

mod definition_loading;
pub mod input_binding;
mod invocation_admission;
pub mod report_contracts;
pub mod rule_identity;
pub use definition_loading::{LocatedRuleDefinition, ValidatedRuleFile};
pub use input_binding::{AdmittedSource, SourceId};
pub use report_contracts::*;
pub use rule_identity::*;
pub mod rule_evaluation_contracts;
pub use rule_evaluation_contracts::*;
pub mod mechanical_contracts;
pub use mechanical_contracts::*;
mod local_check_evaluation;
pub mod rule_backend_factory;
pub use rule_backend_factory::RuleBackendFactory;
pub mod verdict_evaluation;
pub use verdict_evaluation::*;

mod request_state;
pub mod rule_execution;
pub use rule_execution::*;
mod rule_policy;
pub use rule_policy::*;
mod applicability_evaluation;
mod request_planning;

pub mod preparation_contracts;
pub use preparation_contracts::*;
pub mod rule_preparation;
pub use rule_preparation::*;

pub mod selection_contracts;
pub use selection_contracts::*;

pub mod model_request_dispatch;
mod rule_coverage_reporting;
pub use model_request_dispatch::*;
pub mod selection_run;
pub use selection_run::*;
