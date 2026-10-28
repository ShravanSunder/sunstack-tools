//! Replay frozen case sources through rules and score bundles against separately held labels.
mod case_identity;
mod case_store;
mod case_validation;
mod definition_snapshot;
mod eval_reports;
mod eval_run;
mod eval_run_contracts;
mod eval_run_diagnostics;
mod frozen_decoding;
pub use eval_run::{decode_eval_run, run_eval};
pub use eval_run_contracts::*;
mod labeled_cases;
mod payload_snapshot;
mod score_cases;
pub use case_identity::*;
pub use case_store::*;
pub use eval_reports::*;
pub use labeled_cases::*;
pub use score_cases::score;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("score_eval forbids caller field {field}")]
    ForbiddenField { field: String },
    #[error(
        "bundle rule is not from this case set's frozen definitions: {rule:?}; frozen home {home:?}"
    )]
    WrongDefinitions {
        rule: sunstack_rules::RuleRef,
        home: std::path::PathBuf,
    },
    #[error("frozen rule definitions are invalid")]
    Definitions {
        diagnostics: sunstack_rules::DiagnosticList,
    },
    #[error("invalid eval field: {field}")]
    Invalid { field: String },
    #[error("eval file unavailable: {path}")]
    File { path: std::path::PathBuf },
    #[error("frozen content changed: {path}")]
    Integrity { path: std::path::PathBuf },
    #[error("split leakage: {field}")]
    SplitLeakage { field: String },
}
