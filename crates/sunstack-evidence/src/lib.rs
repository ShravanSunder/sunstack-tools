//! Capture and select caller-bound evidence without provider dependencies.
#[path = "conversation_evidence/conversation_contracts.rs"]
pub mod conversation_contracts;
#[path = "evidence_planning/evidence_contracts.rs"]
pub mod evidence_contracts;
#[path = "evidence_planning/evidence_diagnostics.rs"]
pub mod evidence_diagnostics;
#[path = "evidence_planning/evidence_project_loading.rs"]
pub mod evidence_project_loading;
#[path = "document_evidence/standard_markdown_utils.rs"]
pub mod standard_markdown_utils;
#[path = "source_acquisition/structured_json.rs"]
pub mod structured_json;
pub use evidence_project_loading::*;
#[path = "evidence_planning/evidence_kind_contracts.rs"]
pub mod evidence_kind_contracts;
pub use evidence_kind_contracts::*;
#[path = "source_acquisition/checked_file_source.rs"]
pub mod checked_file_source;
pub use checked_file_source::*;
#[path = "evidence_pieces/evidence_model_view.rs"]
pub mod evidence_model_view;
pub use evidence_model_view::*;
pub mod evidence_piece_building;
pub use evidence_piece_building::*;
#[path = "evidence_pieces/evidence_piece_picking.rs"]
pub mod evidence_piece_picking;
pub use evidence_piece_picking::*;

pub use conversation_contracts::*;
pub use evidence_contracts::*;
pub use evidence_diagnostics::*;
pub use structured_json::*;
#[path = "document_evidence/document_selection.rs"]
pub mod document_selection;
#[path = "document_evidence/markdown_contracts.rs"]
pub mod markdown_contracts;
#[path = "document_evidence/markdown_document.rs"]
pub mod markdown_document;
#[path = "document_evidence/markdown_selection_contracts.rs"]
pub mod markdown_selection_contracts;
pub use markdown_selection_contracts::*;
#[path = "document_evidence/frontmatter_contracts.rs"]
pub mod frontmatter_contracts;
pub use frontmatter_contracts::*;
#[path = "document_evidence/markdown_frontmatter.rs"]
mod markdown_frontmatter;
#[path = "document_evidence/markdown_mapped_text.rs"]
mod markdown_mapped_text;
#[path = "document_evidence/markdown_offset_projection.rs"]
mod markdown_offset_projection;
#[path = "document_evidence/markdown_projection.rs"]
mod markdown_projection;
pub use markdown_contracts::*;
pub use markdown_document::*;
#[path = "source_acquisition/payload_inputs.rs"]
pub mod payload_inputs;
#[path = "source_acquisition/source_capture.rs"]
pub mod source_capture;
pub use document_selection::*;
pub use payload_inputs::*;
pub use source_capture::*;
#[path = "conversation_evidence/transcript_formats.rs"]
pub mod transcript_formats;
#[path = "conversation_evidence/transcript_normalization.rs"]
mod transcript_normalization;
pub use transcript_formats::*;
#[path = "conversation_evidence/conversation_capture.rs"]
pub mod conversation_capture;
#[path = "conversation_evidence/conversation_selection.rs"]
pub mod conversation_selection;
pub use conversation_capture::*;
pub use conversation_selection::*;
pub mod evidence_preparation;
pub use evidence_preparation::*;

#[path = "evidence_planning/evidence_admission.rs"]
mod evidence_admission;
pub use evidence_admission::admit_evidence_declaration;
#[path = "evidence_preparation/preparation_contracts.rs"]
pub mod preparation_contracts;
pub use preparation_contracts::*;

#[path = "evidence_preparation/evidence_session.rs"]
pub mod evidence_session;
pub use evidence_session::*;
#[path = "repository_evidence/change_contracts.rs"]
pub mod change_contracts;
#[path = "repository_evidence/connected_contracts.rs"]
pub mod connected_contracts;
pub use change_contracts::*;
pub use connected_contracts::*;
#[path = "repository_evidence/change_selection.rs"]
pub mod change_selection;
pub use change_selection::*;
#[path = "repository_evidence/repository_capture.rs"]
mod repository_capture;
#[path = "repository_evidence/source_units.rs"]
pub mod source_units;
pub use source_units::*;
#[path = "repository_evidence/relationship_discovery.rs"]
pub mod relationship_discovery;
#[path = "repository_evidence/relationship_resolution.rs"]
mod relationship_resolution;
#[path = "repository_evidence/relationship_syntax.rs"]
mod relationship_syntax;
pub use relationship_discovery::*;
#[path = "repository_evidence/semantic_chunks.rs"]
pub mod semantic_chunks;
pub use semantic_chunks::*;
#[path = "repository_evidence/repository_context.rs"]
pub mod repository_context;
pub use repository_context::*;
#[path = "repository_evidence/frozen_queries.rs"]
mod frozen_queries;
pub use frozen_queries::{
    ChangeSetSelection, SourceUnitSelection, select_change_set, select_source_units,
};

#[path = "repository_evidence/change_reporting.rs"]
pub mod change_reporting;
#[path = "repository_evidence/repository_catalogue.rs"]
mod repository_catalogue;
pub use change_reporting::*;
#[path = "source_acquisition/acquisition_progress.rs"]
mod acquisition_progress;
#[path = "repository_evidence/connected_admission.rs"]
mod connected_admission;

#[path = "repository_evidence/change_comparison_contracts.rs"]
pub mod change_comparison_contracts;
pub use change_comparison_contracts::*;

#[path = "evidence_planning/evidence_plan_decoding.rs"]
mod evidence_plan_decoding;
pub use evidence_plan_decoding::decode_evidence_plan_input;

#[path = "evidence_planning/evidence_kind_plan.rs"]
pub mod evidence_kind_plan;
pub use evidence_kind_plan::*;
