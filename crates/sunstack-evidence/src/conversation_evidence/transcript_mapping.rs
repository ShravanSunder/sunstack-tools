use super::transcript_normalization::{
    NormalizationState, category, normalize_records, tool_record_id,
};
use crate::source_acquisition::structured_json::valid_pointer;
use crate::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, btree_map::Entry},
    sync::Arc,
};

/// Declarative extraction for one JSONL transcript format.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptAdapterMapping {
    pub format: TranscriptFormatName,
    pub discriminator: String,
    pub records: BTreeMap<String, TranscriptRecordMapping>,
    #[cfg_attr(feature = "typescript", specta(type = TranscriptRecordSchema))]
    #[serde(
        default,
        deserialize_with = "read_schema",
        skip_serializing_if = "Option::is_none"
    )]
    pub schema: Option<TranscriptRecordSchema>,
}

fn read_schema<'de, TDeserializer: Deserializer<'de>>(
    deserializer: TDeserializer,
) -> Result<Option<TranscriptRecordSchema>, TDeserializer::Error> {
    TranscriptRecordSchema::deserialize(deserializer).map(Some)
}

/// An offline JSON Schema Draft 2020-12 document for selected records.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TranscriptRecordSchema {
    Boolean(bool),
    Object(BTreeMap<String, StructuredJson>),
}

/// JSON Pointers for the fields consumed from a selected record.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TranscriptRecordMapping {
    UserMessage {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
    },
    AssistantMessage {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
    },
    ToolCall {
        call_id: String,
        tool_name: String,
        input: String,
    },
    ToolResult {
        call_id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_error: Option<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MappedRecordIdentity {
    NativeMessage,
    OffsetMessage,
    ToolCall,
}

#[derive(Default)]
struct MappedRecordOwners(BTreeMap<RecordId, MappedRecordIdentity>);

impl MappedRecordOwners {
    fn accept(
        &mut self,
        record_id: RecordId,
        identity: MappedRecordIdentity,
        normalize: impl FnOnce(&RecordId) -> EvidenceResult<()>,
    ) -> EvidenceResult<()> {
        match self.0.entry(record_id) {
            Entry::Occupied(entry) if *entry.get() != identity => {
                Err(EvidenceCode::AdapterOutputInvalid.into())
            }
            Entry::Occupied(entry) => normalize(entry.key()),
            Entry::Vacant(entry) => {
                normalize(entry.key())?;
                entry.insert(identity);
                Ok(())
            }
        }
    }
}

impl TranscriptRecordMapping {
    fn pointers_are_valid(&self) -> bool {
        match self {
            Self::UserMessage { text, message_id }
            | Self::AssistantMessage { text, message_id } => {
                valid_pointer(text) && message_id.as_deref().is_none_or(valid_pointer)
            }
            Self::ToolCall {
                call_id,
                tool_name,
                input,
            } => [call_id, tool_name, input]
                .into_iter()
                .all(|pointer| valid_pointer(pointer)),
            Self::ToolResult {
                call_id,
                text,
                is_error,
            } => {
                valid_pointer(call_id)
                    && valid_pointer(text)
                    && is_error.as_deref().is_none_or(valid_pointer)
            }
        }
    }

    fn normalize(
        &self,
        state: &mut NormalizationState,
        owners: &mut MappedRecordOwners,
        record: &Value,
        position: u64,
    ) -> EvidenceResult<()> {
        match self {
            Self::UserMessage { text, message_id }
            | Self::AssistantMessage { text, message_id } => {
                let (record_id, identity) = match message_id
                    .as_deref()
                    .and_then(|pointer| record.pointer(pointer))
                {
                    Some(value) => (
                        RecordId::new(value.as_str().ok_or(EvidenceCode::AdapterOutputInvalid)?)?,
                        MappedRecordIdentity::NativeMessage,
                    ),
                    None => (
                        RecordId::new(format!("offset:{position}"))?,
                        MappedRecordIdentity::OffsetMessage,
                    ),
                };
                let assistant = matches!(self, Self::AssistantMessage { .. });
                let text = extract_text(record, text)?.into();
                let injection = if assistant {
                    InjectionOrigin::NotInjected
                } else {
                    InjectionOrigin::Unknown
                };
                owners.accept(record_id, identity, |record_id| {
                    state.message(
                        record_id.clone(),
                        position,
                        assistant,
                        vec![MessageBlock::Text { text }],
                        injection,
                    )
                })
            }
            Self::ToolCall {
                call_id,
                tool_name,
                input,
            } => {
                let call_id = CallId::new(extract_text(record, call_id)?)?;
                let tool_name = extract_text(record, tool_name)?;
                if tool_name.is_empty() {
                    return Err(EvidenceCode::AdapterOutputInvalid.into());
                }
                let input = record
                    .pointer(input)
                    .ok_or(EvidenceCode::AdapterOutputInvalid)?;
                let input = serde_json::from_value(input.clone())
                    .map_err(|_| EvidenceCode::AdapterOutputInvalid)?;
                owners.accept(
                    tool_record_id(&call_id)?,
                    MappedRecordIdentity::ToolCall,
                    |_| {
                        state.call(
                            call_id,
                            tool_name.into(),
                            input,
                            position,
                            category(tool_name, false),
                        )
                    },
                )
            }
            Self::ToolResult {
                call_id,
                text,
                is_error,
            } => {
                let call_id = CallId::new(extract_text(record, call_id)?)?;
                let text = extract_text(record, text)?.into();
                let is_error = is_error
                    .as_deref()
                    .and_then(|pointer| record.pointer(pointer));
                let outcome = match is_error {
                    Some(Value::Bool(true)) => ToolOutcome::Failed,
                    Some(Value::Bool(false)) => ToolOutcome::Succeeded,
                    Some(_) => return Err(EvidenceCode::AdapterOutputInvalid.into()),
                    None => ToolOutcome::ResultPresentStatusUnknown,
                };
                state.result(
                    call_id,
                    ToolResult::FullObserved { text },
                    outcome,
                    position,
                );
                Ok(())
            }
        }
    }
}

fn extract_text<'record>(record: &'record Value, pointer: &str) -> EvidenceResult<&'record str> {
    record
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| EvidenceCode::AdapterOutputInvalid.into())
}

struct MappedTranscriptAdapter {
    mapping: TranscriptAdapterMapping,
    validator: Option<jsonschema::Validator>,
    identity: String,
}

impl MappedTranscriptAdapter {
    fn admit(mapping: TranscriptAdapterMapping) -> EvidenceResult<Self> {
        if mapping.records.is_empty() {
            return Err(EvidenceCode::InvalidRequest.into());
        }
        if !valid_pointer(&mapping.discriminator)
            || !mapping
                .records
                .values()
                .all(TranscriptRecordMapping::pointers_are_valid)
        {
            return Err(EvidenceCode::InvalidPointer.into());
        }
        let validator = mapping
            .schema
            .as_ref()
            .map(|schema| {
                let schema =
                    serde_json::to_value(schema).map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
                jsonschema::options()
                    .with_draft(jsonschema::Draft::Draft202012)
                    .offline()
                    .build(&schema)
                    .map_err(|_| EvidenceCode::PayloadSchemaInvalid)
            })
            .transpose()?;
        // Typed fields and ordered maps make the hash independent of input key order.
        let canonical = serde_json::to_vec(&mapping).map_err(|_| EvidenceCode::InvalidRequest)?;
        let identity = format!("mapping:v1:{}", content_digest(&canonical));
        Ok(Self {
            mapping,
            validator,
            identity,
        })
    }
}

impl TranscriptAdapter for MappedTranscriptAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.mapping.format
    }

    fn cursor_identity(&self) -> Option<&str> {
        Some(&self.identity)
    }

    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        let mut owners = MappedRecordOwners::default();
        normalize_records(capture.bytes, |state, record, position| {
            let event = extract_text(&record, &self.mapping.discriminator)?;
            let Some(mapping) = self.mapping.records.get(event) else {
                state.unsupported_record(position);
                return Ok(());
            };
            if self
                .validator
                .as_ref()
                .is_some_and(|validator| !validator.is_valid(&record))
            {
                return Err(EvidenceCode::AdapterOutputInvalid.into());
            }
            mapping.normalize(state, &mut owners, &record, position)
        })
    }
}

impl AdapterRegistry {
    pub fn with_mapping(self, mapping: TranscriptAdapterMapping) -> EvidenceResult<Self> {
        if self.adapter(&mapping.format).is_ok() {
            return Err(EvidenceCode::UnknownTranscriptFormat.into());
        }
        self.with_adapter(Arc::new(MappedTranscriptAdapter::admit(mapping)?))
    }
}

#[cfg(test)]
#[path = "transcript_mapping_test.rs"]
mod tests;
