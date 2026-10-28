use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MappedFieldPrivacy {
    pub private_kind: Option<PrivateKind>,
    pub readable: bool,
}
pub(crate) struct MappedTranscriptReader {
    name: TranscriptFormatName,
    base_format: TranscriptFormatName,
    mapping: RecordMapping,
}
impl MappedTranscriptReader {
    pub(crate) fn new(
        format: &TranscriptFormatName,
        mapping: &RecordMapping,
    ) -> EvidenceResult<Self> {
        if !matches!(
            format.as_str(),
            "claude_code_jsonl" | "codex_rollout_jsonl" | "cursor_export_jsonl"
        ) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        if mapping
            .fields
            .values()
            .any(|pointer| !crate::custom_sources::valid_pointer(pointer))
        {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        let bytes =
            serde_json::to_vec(&(format, mapping)).map_err(|_| EvidenceCode::InvalidSelector)?;
        Ok(Self {
            name: TranscriptFormatName::new(format!("mapped_{}", content_digest(&bytes)))?,
            base_format: format.clone(),
            mapping: mapping.clone(),
        })
    }
}
pub(crate) fn map_line(
    mut line: SourceLine,
    raw: &serde_json::Value,
    mapping: &RecordMapping,
) -> SourceLine {
    let mapped = (|| {
        let regions = private_regions(&line.events, raw);
        let mut fields = BTreeMap::new();
        let mut field_privacy = BTreeMap::new();
        for (name, pointer) in &mapping.fields {
            let value = raw
                .pointer(pointer)
                .ok_or(EvidenceCode::AdapterOutputInvalid)?;
            let classification = classify_pointer(pointer, &regions);
            // Opaque bytes remain only in the frozen raw capture, never in mapped events.
            if classification.readable {
                fields.insert(
                    name.clone(),
                    serde_json::from_value(value.clone())
                        .map_err(|_| EvidenceCode::AdapterOutputInvalid)?,
                );
            }
            field_privacy.insert(name.clone(), classification);
        }
        Ok::<_, EvidenceError>((fields, field_privacy))
    })();
    let (author, injection) = line
        .events
        .iter()
        .find_map(|event| match event {
            TranscriptEvent::MessagePart {
                author, injection, ..
            } => Some((*author, *injection)),
            TranscriptEvent::PrivateRecord { author, .. } => {
                Some((*author, InjectionOrigin::NotInjected))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            if line
                .events
                .iter()
                .any(|event| matches!(event, TranscriptEvent::ToolCall { .. }))
            {
                (RecordAuthor::Assistant, InjectionOrigin::NotInjected)
            } else if line
                .events
                .iter()
                .any(|event| matches!(event, TranscriptEvent::ToolResult { .. }))
            {
                (RecordAuthor::Tool, InjectionOrigin::NotInjected)
            } else {
                (RecordAuthor::Custom, InjectionOrigin::Unknown)
            }
        });
    let mut events = line
        .events
        .iter()
        .filter(|event| matches!(event, TranscriptEvent::Gap { .. }))
        .cloned()
        .collect::<Vec<_>>();
    match mapped {
        Ok((fields, field_privacy)) => events.insert(
            0,
            TranscriptEvent::CustomRecord {
                author,
                injection,
                fields,
                field_privacy,
                position: line.position,
            },
        ),
        Err(_) => events.push(TranscriptEvent::Gap {
            gap: CoverageGap::MalformedRecord {
                position: line.position.byte_offset,
            },
        }),
    }
    line.events = events;
    // New mapped formats have no historical merge disposition to replay.
    line.legacy = LegacyDisposition::Folded;
    line
}
impl TranscriptEventReader for MappedTranscriptReader {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn lines(&self, capture: &CapturedTranscript<'_>) -> Vec<SourceLine> {
        crate::transcript_event_reading::read_mapped_lines(
            self.base_format.as_str(),
            capture.bytes,
            &self.mapping,
        )
    }
}
impl TranscriptAdapter for MappedTranscriptReader {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        crate::transcript_normalization::fold_transcript_lines(self.lines(capture))
    }
}
fn private_regions(
    events: &[TranscriptEvent],
    raw: &serde_json::Value,
) -> Vec<(String, MappedFieldPrivacy)> {
    let mut regions = Vec::new();
    for event in events {
        match event {
            TranscriptEvent::PrivateRecord {
                private_kind, text, ..
            } => {
                regions.push((
                    "/payload".into(),
                    MappedFieldPrivacy {
                        private_kind: Some(*private_kind),
                        readable: false,
                    },
                ));
                if text.is_some() {
                    regions.push((
                        "/payload/summary".into(),
                        MappedFieldPrivacy {
                            private_kind: Some(*private_kind),
                            readable: true,
                        },
                    ));
                }
            }
            TranscriptEvent::MessagePart {
                private_kind: Some(PrivateKind::AnalysisChannel),
                ..
            } => regions.push((
                "/payload/content".into(),
                MappedFieldPrivacy {
                    private_kind: Some(PrivateKind::AnalysisChannel),
                    readable: true,
                },
            )),
            TranscriptEvent::MessagePart {
                blocks,
                private_kind,
                ..
            } if *private_kind == Some(PrivateKind::Thinking)
                || blocks
                    .iter()
                    .any(|block| matches!(block, MessageBlock::Thinking { .. })) =>
            {
                if let Some(content) = raw
                    .pointer("/message/content")
                    .and_then(serde_json::Value::as_array)
                {
                    for (index, block) in content.iter().enumerate() {
                        if block.get("type").and_then(serde_json::Value::as_str) == Some("thinking")
                        {
                            regions.push((
                                format!("/message/content/{index}"),
                                MappedFieldPrivacy {
                                    private_kind: Some(PrivateKind::Thinking),
                                    readable: true,
                                },
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    regions
}
fn within(pointer: &str, prefix: &str) -> bool {
    pointer == prefix
        || pointer
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}
fn classify_pointer(pointer: &str, regions: &[(String, MappedFieldPrivacy)]) -> MappedFieldPrivacy {
    if let Some((_, classification)) = regions
        .iter()
        .filter(|(prefix, _)| within(pointer, prefix))
        .max_by_key(|(prefix, _)| prefix.len())
    {
        return *classification;
    }
    let contained = regions
        .iter()
        .filter(|(prefix, _)| pointer.is_empty() || within(prefix, pointer))
        .map(|(_, classification)| *classification)
        .collect::<Vec<_>>();
    contained
        .iter()
        .find(|classification| !classification.readable)
        .or_else(|| contained.first())
        .copied()
        .unwrap_or(MappedFieldPrivacy {
            private_kind: None,
            readable: true,
        })
}
