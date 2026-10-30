//! Native decisions, request admission, and bounded Jev execution.
//! The caller owns the Tokio runtime and cancellation token.
#[path = "backend_transport/decision_backends.rs"]
pub mod decision_backends;
#[path = "decision_protocol/decision_contracts.rs"]
pub mod decision_contracts;
#[path = "decision_protocol/decision_diagnostics.rs"]
pub mod decision_diagnostics;
#[path = "judgment_processing/judgment_execution.rs"]
pub mod judgment_execution;
#[path = "judgment_processing/judgment_validation.rs"]
pub mod judgment_validation;
#[path = "judgment_processing/request_budgeting.rs"]
pub mod request_budgeting;

pub use decision_backends::*;
pub use decision_contracts::*;
pub use decision_diagnostics::*;
pub use judgment_execution::*;
pub use request_budgeting::*;
pub use tokio_util::sync::CancellationToken;

#[cfg(test)]
extern crate self as sunstack_decisions;
