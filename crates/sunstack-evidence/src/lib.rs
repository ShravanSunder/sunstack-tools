//! Capture and select caller-bound evidence without provider dependencies.
pub mod conversation_contracts;
pub mod evidence_contracts;
pub mod evidence_diagnostics;
pub mod evidence_project_loading;
pub mod standard_markdown_utils;
pub mod structured_json;
pub use evidence_project_loading::*;
pub mod evidence_kind_contracts;
pub use evidence_kind_contracts::*;
pub mod checked_file_source;
pub use checked_file_source::*;
pub mod evidence_model_view;
pub use evidence_model_view::*;
pub mod evidence_piece_building;
pub use evidence_piece_building::*;
pub mod evidence_piece_picking;
pub use evidence_piece_picking::*;

pub use conversation_contracts::*;
pub use evidence_contracts::*;
pub use evidence_diagnostics::*;
pub use structured_json::*;
pub mod document_selection;
pub mod markdown_contracts;
pub mod markdown_document;
pub mod markdown_selection_contracts;
pub use markdown_selection_contracts::*;
pub mod frontmatter_contracts;
pub use frontmatter_contracts::*;
mod markdown_frontmatter;
mod markdown_mapped_text;
mod markdown_offset_projection;
mod markdown_projection;
pub use markdown_contracts::*;
pub use markdown_document::*;
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
pub use evidence_admission::admit_evidence_declaration;
pub mod preparation_contracts;
pub use preparation_contracts::*;

pub mod evidence_session;
pub use evidence_session::*;
pub mod change_contracts;
pub mod connected_contracts;
pub use change_contracts::*;
pub use connected_contracts::*;
pub mod change_selection;
pub use change_selection::*;
mod repository_capture;
pub mod source_units;
pub use source_units::*;
pub mod relationship_discovery;
mod relationship_resolution;
mod relationship_syntax;
pub use relationship_discovery::*;
pub mod semantic_chunks;
pub use semantic_chunks::*;
pub mod repository_context;
pub use repository_context::*;
mod frozen_queries;
pub use frozen_queries::{
    ChangeSetSelection, SourceUnitSelection, select_change_set, select_source_units,
};

pub mod change_reporting;
mod repository_catalogue;
pub use change_reporting::*;
mod acquisition_progress;
mod connected_admission;

pub mod change_comparison_contracts;
pub use change_comparison_contracts::*;

mod evidence_plan_decoding;
pub use evidence_plan_decoding::decode_evidence_plan_input;

pub mod evidence_kind_plan;
pub use evidence_kind_plan::*;
