//! Capture and select caller-bound evidence without provider dependencies.
pub mod conversation_contracts;
pub mod evidence_contracts;
pub mod evidence_diagnostics;
pub mod structured_json;

pub use conversation_contracts::*;
pub use evidence_contracts::*;
pub use evidence_diagnostics::*;
pub use structured_json::*;
pub mod document_selection;
pub mod payload_inputs;
pub mod source_capture;
pub use document_selection::*;
pub use payload_inputs::*;
pub use source_capture::*;
pub mod transcript_formats;
mod transcript_normalization;
pub use transcript_formats::*;
pub mod conversation_capture;
pub mod conversation_selection;
pub use conversation_capture::*;
pub use conversation_selection::*;
pub mod evidence_preparation;
pub use evidence_preparation::*;

mod evidence_admission;
pub mod preparation_contracts;
pub use preparation_contracts::*;

pub mod evidence_session;
pub use evidence_session::*;

mod acquisition_progress;

mod evidence_plan_decoding;
pub use evidence_plan_decoding::decode_evidence_plan_input;
