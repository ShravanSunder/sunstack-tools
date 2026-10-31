//! Native decisions, request admission, and bounded Jev execution.
//! The caller owns the Tokio runtime and cancellation token.
pub mod backend_transport;
pub mod decision_protocol;
pub mod judgment_processing;

pub use backend_transport::decision_backends::*;
pub use decision_protocol::decision_contracts::*;
pub use decision_protocol::decision_diagnostics::*;
pub use judgment_processing::judgment_execution::*;
pub use judgment_processing::request_budgeting::*;
pub use tokio_util::sync::CancellationToken;
#[cfg(test)]
extern crate self as sunstack_decisions;
