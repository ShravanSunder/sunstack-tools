use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomEntrySchema {
    pub format: CustomRecordFormat,
    pub record_schema: StructuredJson,
    pub fields: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomRecordFormat {
    Json,
    Jsonl,
}

pub(crate) struct CustomRecordReader {
    name: TranscriptFormatName,
    schema_name: SchemaName,
    schema: CustomEntrySchema,
    catalog: PayloadSchemaCatalog,
}
impl CustomRecordReader {
    pub(crate) fn new(schema: &CustomEntrySchema) -> EvidenceResult<Self> {
        if schema
            .fields
            .values()
            .any(|pointer| !valid_pointer(pointer))
        {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        let schema_name = SchemaName::new("urn:sunstack:custom-record")?;
        let value = serde_json::to_value(&schema.record_schema)
            .map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
        let catalog = PayloadSchemaCatalog::compile([(schema_name.clone(), value)].into())?;
        let bytes = serde_json::to_vec(schema).map_err(|_| EvidenceCode::InvalidSelector)?;
        Ok(Self {
            name: TranscriptFormatName::new(format!("custom_{}", content_digest(&bytes)))?,
            schema_name,
            schema: schema.clone(),
            catalog,
        })
    }
    fn mapped_line(&self, raw: serde_json::Value, position: SourcePosition) -> SourceLine {
        let mapped = (|| {
            let raw: StructuredJson =
                serde_json::from_value(raw).map_err(|_| EvidenceCode::InvalidSelector)?;
            self.catalog.validate(&self.schema_name, raw.clone())?;
            self.schema
                .fields
                .iter()
                .map(|(name, pointer)| {
                    raw.pointer(pointer)
                        .cloned()
                        .map(|value| (name.clone(), value))
                        .ok_or_else(|| EvidenceCode::PayloadSchemaMismatch.into())
                })
                .collect::<EvidenceResult<BTreeMap<_, _>>>()
        })();
        let events = match mapped {
            Ok(fields) => vec![TranscriptEvent::CustomRecord {
                author: RecordAuthor::Custom,
                injection: InjectionOrigin::NotInjected,
                field_privacy: BTreeMap::new(),
                fields,
                position,
            }],
            Err(_) => vec![TranscriptEvent::Gap {
                gap: CoverageGap::MalformedRecord {
                    position: position.byte_offset,
                },
            }],
        };
        SourceLine {
            position,
            events,
            legacy: LegacyDisposition::Folded,
            parent: None,
        }
    }
}
impl TranscriptEventReader for CustomRecordReader {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn framing(&self) -> TranscriptFraming {
        match self.schema.format {
            CustomRecordFormat::Json => TranscriptFraming::JsonDocument,
            CustomRecordFormat::Jsonl => TranscriptFraming::JsonLines,
        }
    }
    fn lines(&self, capture: &CapturedTranscript<'_>) -> Vec<SourceLine> {
        match self.schema.format {
            CustomRecordFormat::Json => document_records(capture.bytes)
                .unwrap_or_default()
                .into_iter()
                .map(|(offset, raw)| self.mapped_line(raw, source_position(capture.bytes, offset)))
                .collect(),
            CustomRecordFormat::Jsonl => {
                let mut offset = 0;
                capture
                    .bytes
                    .split_inclusive(|byte| *byte == b'\n')
                    .enumerate()
                    .map(|(line, bytes)| {
                        let position = SourcePosition {
                            byte_offset: offset as u64,
                            line: line as u64 + 1,
                        };
                        offset += bytes.len();
                        match serde_json::from_slice(bytes) {
                            Ok(raw) => self.mapped_line(raw, position),
                            Err(_) => SourceLine {
                                position,
                                legacy: LegacyDisposition::Folded,
                                parent: None,
                                events: vec![TranscriptEvent::Gap {
                                    gap: CoverageGap::MalformedRecord {
                                        position: position.byte_offset,
                                    },
                                }],
                            },
                        }
                    })
                    .collect()
            }
        }
    }
}
impl TranscriptAdapter for CustomRecordReader {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        crate::transcript_normalization::fold_transcript_lines(self.lines(capture))
    }
}
pub(crate) fn valid_pointer(pointer: &str) -> bool {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return false;
    }
    let mut characters = pointer.chars();
    while let Some(character) = characters.next() {
        if character == '~' && !matches!(characters.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}
fn source_position(bytes: &[u8], offset: usize) -> SourcePosition {
    SourcePosition {
        byte_offset: offset as u64,
        line: bytes[..offset]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count() as u64
            + 1,
    }
}
fn document_records(bytes: &[u8]) -> Option<Vec<(usize, serde_json::Value)>> {
    let mut offset = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    if bytes[offset] != b'[' {
        return serde_json::from_slice(&bytes[offset..])
            .ok()
            .map(|raw| vec![(offset, raw)]);
    }
    offset += 1;
    let mut records = Vec::new();
    loop {
        while bytes.get(offset).is_some_and(u8::is_ascii_whitespace) {
            offset += 1;
        }
        if bytes.get(offset) == Some(&b']') {
            return bytes[offset + 1..]
                .iter()
                .all(u8::is_ascii_whitespace)
                .then_some(records);
        }
        let mut stream = serde_json::Deserializer::from_slice(bytes.get(offset..)?)
            .into_iter::<serde_json::Value>();
        let value = stream.next()?.ok()?;
        let length = stream.byte_offset();
        records.push((offset, value));
        offset += length;
        while bytes.get(offset).is_some_and(u8::is_ascii_whitespace) {
            offset += 1;
        }
        match bytes.get(offset)? {
            b',' => {
                offset += 1;
                if bytes
                    .get(offset..)?
                    .iter()
                    .find(|byte| !byte.is_ascii_whitespace())
                    == Some(&b']')
                {
                    return None;
                }
            }
            b']' => {}
            _ => return None,
        }
    }
}
