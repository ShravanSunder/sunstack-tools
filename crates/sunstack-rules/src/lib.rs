//! Admit authored definitions, plan evidence-backed requests, and reduce native answers in authored order.
pub mod definition_admission;
pub mod definition_contracts;
pub mod invocation_contracts;
pub mod rule_diagnostics;
mod strict_decoding;
pub use definition_admission::*;
pub use definition_contracts::*;
pub use invocation_contracts::*;
pub use rule_diagnostics::*;
pub use strict_decoding::decode_rule_invocation;

pub mod input_binding;
mod invocation_admission;
pub mod report_contracts;
pub use input_binding::{AdmittedSource, SourceId};
pub use invocation_admission::{LocatedRuleDefinition, ValidatedRuleFile};
pub use report_contracts::*;
pub mod verdict_evaluation;
pub use verdict_evaluation::*;

mod request_encoding;
mod request_state;
pub mod rule_execution;
pub use rule_execution::*;
mod applicability_evaluation;
mod request_planning;
