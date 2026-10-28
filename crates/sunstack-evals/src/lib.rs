//! Freeze label-blind replay material and score recorded candidate bundles without model work.
mod case_identity;
mod case_store;
mod case_validation;
mod eval_reports;
mod frozen_decoding;
mod labeled_cases;
mod score_cases;
pub use case_identity::*;
pub use case_store::*;
pub use eval_reports::*;
pub use labeled_cases::*;
pub use score_cases::score;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("invalid eval field: {field}")]
    Invalid { field: String },
    #[error("eval file unavailable: {path}")]
    File { path: std::path::PathBuf },
    #[error("frozen content changed: {path}")]
    Integrity { path: std::path::PathBuf },
    #[error("split leakage: {field}")]
    SplitLeakage { field: String },
}
