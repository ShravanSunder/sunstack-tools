use crate::{Diagnostic, DiagnosticLocation, EvidenceCode, EvidencePlanInput};
use serde_path_to_error::{Path, Segment};

pub fn decode_evidence_plan_input(bytes: &[u8]) -> Result<EvidencePlanInput, Box<Diagnostic>> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let input = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let message = error.inner().to_string();
        let code = if message.starts_with("unknown field") {
            EvidenceCode::UnknownField
        } else if message.starts_with("duplicate field") {
            EvidenceCode::DuplicateField
        } else {
            EvidenceCode::InvalidRequest
        };
        request_diagnostic(code, safe_request_path(error.path()))
    })?;
    deserializer
        .end()
        .map_err(|_| request_diagnostic(EvidenceCode::InvalidRequest, vec![]))?;
    Ok(input)
}

fn request_diagnostic(code: EvidenceCode, field_path: Vec<String>) -> Box<Diagnostic> {
    let mut diagnostic = Diagnostic::new(code);
    diagnostic.location = DiagnosticLocation::Field { field_path };
    Box::new(diagnostic)
}

fn safe_request_path(path: &Path) -> Vec<String> {
    let mut safe_path = Vec::new();
    for segment in path {
        match segment {
            Segment::Seq { .. } => continue,
            Segment::Map { key } if schema_owned_field(key) => {
                safe_path.push(key.clone());
                // The next segment in sources is an authored map key, even if its
                // spelling happens to match a schema field. Keep only the prefix.
                if key == "sources" || safe_path.len() == 32 {
                    break;
                }
            }
            _ => break,
        }
    }
    safe_path
}

fn schema_owned_field(field: &str) -> bool {
    matches!(
        field,
        "repositories"
            | "key"
            | "root"
            | "input_paths"
            | "change_scope"
            | "kind"
            | "path"
            | "pattern"
            | "sources"
            | "payload_schemas"
            | "name"
            | "cursors"
            | "transcript_path"
            | "format"
            | "cursor"
            | "declarations"
            | "id"
            | "input"
            | "requirement"
            | "select"
            | "target_id"
            | "text"
            | "language"
            | "turns"
            | "count"
            | "user_messages"
            | "assistant_messages"
            | "tool_use"
            | "tools"
            | "names"
            | "categories"
            | "results"
            | "hook_injected"
            | "pointer"
            | "max_items"
            | "max_bytes"
            | "native_queries"
            | "selector"
            | "repository_id"
            | "source_path"
            | "paths"
            | "package_bindings"
            | "importing_repository_id"
            | "module_prefix"
            | "target_repository_id"
            | "target_path_prefix"
            | "limits"
            | "max_files"
            | "max_file_bytes"
            | "max_total_bytes"
            | "max_matches"
            | "max_transcript_bytes"
            | "include_ignored"
            | "discovery"
            | "max_unit_visits"
            | "max_relationship_candidates"
            | "max_fan_out"
            | "max_history_commits"
            | "max_local_time_ms"
    )
}

#[cfg(test)]
#[path = "evidence_plan_decoding_test.rs"]
mod tests;
