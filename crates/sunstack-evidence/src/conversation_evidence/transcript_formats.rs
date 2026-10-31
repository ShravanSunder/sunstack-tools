use crate::conversation_evidence::transcript_normalization::normalize_builtin;
use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Normalize only the supplied complete bytes. Whole messages share host identity; tool results
/// amend their call. Record position is the last contributing entry's byte offset, so cursors
/// detect amendments. Coverage gaps retain unsupported, malformed and orphan evidence.
/// The implementation is trusted native code; this trait is not a sandbox.
pub trait TranscriptAdapter: Send + Sync {
    fn format_name(&self) -> &TranscriptFormatName;
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput;
}
#[derive(Clone)]
pub struct AdapterRegistry {
    adapters: BTreeMap<TranscriptFormatName, Arc<dyn TranscriptAdapter>>,
}
impl AdapterRegistry {
    pub fn builtins() -> EvidenceResult<Self> {
        let mut registry = Self {
            adapters: BTreeMap::new(),
        };
        for name in ["claude_code_jsonl", "codex_rollout_jsonl"] {
            registry = registry.with_adapter(Arc::new(BuiltinAdapter {
                name: TranscriptFormatName::new(name)?,
            }))?;
        }
        Ok(registry)
    }
    pub fn with_adapter(mut self, adapter: Arc<dyn TranscriptAdapter>) -> EvidenceResult<Self> {
        if self.adapters.contains_key(adapter.format_name()) {
            return Err(EvidenceCode::UnknownTranscriptFormat.into());
        }
        self.adapters.insert(adapter.format_name().clone(), adapter);
        Ok(self)
    }
    pub fn adapter(&self, name: &TranscriptFormatName) -> EvidenceResult<&dyn TranscriptAdapter> {
        self.adapters
            .get(name)
            .map(Arc::as_ref)
            .ok_or_else(|| EvidenceCode::UnknownTranscriptFormat.into())
    }
}
struct BuiltinAdapter {
    name: TranscriptFormatName,
}
impl TranscriptAdapter for BuiltinAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        normalize_builtin(self.name.as_str(), capture.bytes)
    }
}
/// Validation runs before hook supplements are added, for built-ins and external adapters alike.
pub fn validate_adapter_output(output: &AdapterOutput, committed_bytes: u64) -> EvidenceResult<()> {
    let invalid = || EvidenceError::from(EvidenceCode::AdapterOutputInvalid);
    let mut records = BTreeSet::new();
    let mut calls = BTreeSet::new();
    for record in &output.records {
        if record.position >= committed_bytes
            || record.provenance != RecordProvenance::Transcript
            || !records.insert(&record.record_id)
        {
            return Err(invalid());
        }
        if let ConversationBody::ToolUse(tool) = &record.body {
            if tool.tool_name.is_empty() || !calls.insert(&tool.call_id) {
                return Err(invalid());
            }
            let absent = matches!(tool.result, ToolResult::Absent);
            if absent != matches!(tool.outcome, ToolOutcome::ResultMissing) {
                return Err(invalid());
            }
            if let ToolResult::Clipped {
                text,
                original_bytes: OriginalBytes::Known { bytes },
            } = &tool.result
                && *bytes < text.len()
            {
                return Err(invalid());
            }
        }
    }
    let mut assigned = BTreeSet::new();
    for turn in &output.turns {
        for record in &turn.records {
            if !records.contains(record) || !assigned.insert(record) {
                return Err(invalid());
            }
        }
        if let TurnOpening::UserMessage { record_id } = &turn.opening
            && (!turn.records.contains(record_id)
                || !output.records.iter().any(|record| {
                    &record.record_id == record_id
                        && matches!(record.body, ConversationBody::UserMessage { .. })
                }))
        {
            return Err(invalid());
        }
    }
    if assigned != records {
        return Err(invalid());
    }
    for gap in &output.gaps {
        match gap {
            CoverageGap::UnsupportedRecord { position }
            | CoverageGap::MalformedRecord { position }
            | CoverageGap::OrphanResult { position, .. }
            | CoverageGap::AmbiguousTurn { position, .. }
                if *position >= committed_bytes =>
            {
                return Err(invalid());
            }
            CoverageGap::UnknownInjection { record_id } if !records.contains(record_id) => {
                return Err(invalid());
            }
            _ => {}
        }
    }
    Ok(())
}
