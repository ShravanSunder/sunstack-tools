//! Capture and select caller-bound evidence without provider dependencies.
pub mod conversation_evidence;
pub mod document_evidence;
pub mod evidence_execution;
pub mod evidence_piece_building;
pub mod evidence_pieces;
pub mod evidence_planning;
pub mod repository_evidence;
pub mod source_acquisition;

pub use conversation_evidence::conversation_capture::*;
pub use conversation_evidence::conversation_contracts::*;
pub use conversation_evidence::conversation_selection::*;
pub use conversation_evidence::transcript_formats::*;
pub use document_evidence::document_selection::*;
pub use document_evidence::frontmatter_contracts::*;
pub use document_evidence::markdown_contracts::*;
pub use document_evidence::markdown_document::*;
pub use document_evidence::markdown_selection_contracts::*;
pub use evidence_execution::evidence_preparation::*;
pub use evidence_execution::evidence_session::*;
pub use evidence_execution::preparation_contracts::*;
pub use evidence_piece_building::*;
pub use evidence_pieces::evidence_model_view::*;
pub use evidence_pieces::evidence_piece_picking::*;
pub use evidence_planning::evidence_admission::admit_evidence_declaration;
pub use evidence_planning::evidence_contracts::*;
pub use evidence_planning::evidence_diagnostics::*;
pub use evidence_planning::evidence_kind_contracts::*;
pub use evidence_planning::evidence_kind_plan::*;
pub use evidence_planning::evidence_plan_decoding::decode_evidence_plan_input;
pub use evidence_planning::evidence_project_loading::*;
pub use repository_evidence::change_comparison_contracts::*;
pub use repository_evidence::change_contracts::*;
pub use repository_evidence::change_reporting::*;
pub use repository_evidence::change_selection::*;
pub use repository_evidence::connected_contracts::*;
pub use repository_evidence::frozen_queries::{
    ChangeSetSelection, SourceUnitSelection, select_change_set, select_source_units,
};
pub use repository_evidence::relationship_discovery::*;
pub use repository_evidence::repository_context::*;
pub use repository_evidence::semantic_chunks::*;
pub use repository_evidence::source_units::*;
pub use source_acquisition::checked_file_source::*;
pub use source_acquisition::payload_inputs::*;
pub use source_acquisition::source_capture::*;
pub use source_acquisition::structured_json::*;
