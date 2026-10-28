use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

validated_name!(PositionedRecordId, |value: &str| !value.is_empty());

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PositionedRecord {
    pub record_id: PositionedRecordId,
    pub author: RecordAuthor,
    pub positions: Vec<SourcePosition>,
    pub injection: InjectionOrigin,
    pub body: PositionedBody,
    #[serde(skip)]
    pub field_positions: BTreeMap<FieldPath, SourcePosition>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PositionedBody {
    Message {
        blocks: Vec<PositionedBlock>,
    },
    Private {
        private_kind: PrivateKind,
        text: Option<String>,
    },
    ToolCall {
        call_id: CallId,
        tool: ToolName,
        input: StructuredJson,
    },
    ToolResult {
        call_id: CallId,
        outcome: ToolOutcome,
        result: ToolResult,
    },
    Custom {
        fields: BTreeMap<String, StructuredJson>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PositionedBlock {
    pub block: MessageBlock,
    pub private_kind: Option<PrivateKind>,
}

#[derive(Clone, Debug, Default)]
pub struct PositionedRecords {
    pub records: Vec<PositionedRecord>,
    pub gaps: Vec<CoverageGap>,
}

pub fn project_positioned(
    source: &SourceInputId,
    lines: &[SourceLine],
) -> EvidenceResult<PositionedRecords> {
    let mut output = PositionedRecords::default();
    let mut messages = BTreeMap::<RecordId, usize>::new();
    let mut calls = BTreeSet::new();
    let mut parents = BTreeMap::<String, usize>::new();
    'lines: for line in lines {
        for event in &line.events {
            let (author, position, injection, body) = match event {
                TranscriptEvent::MessagePart {
                    message_id,
                    author,
                    position,
                    blocks,
                    injection,
                    private_kind,
                } => {
                    if let Some(index) = messages.get(message_id).copied() {
                        let record = &mut output.records[index];
                        if record.author != *author {
                            output.gaps.push(CoverageGap::MalformedRecord {
                                position: position.byte_offset,
                            });
                            continue 'lines;
                        }
                        if let PositionedBody::Message { blocks: previous } = &mut record.body {
                            let first_block = previous.len();
                            for block_index in 0..blocks.len() {
                                record.field_positions.insert(
                                    FieldPath::new(format!(
                                        "/body/blocks/{}",
                                        first_block + block_index
                                    ))?,
                                    *position,
                                );
                            }
                            previous.extend(positioned_blocks(blocks, *private_kind));
                            if record.positions.last() != Some(position) {
                                record.positions.push(*position);
                            }
                        }
                        continue;
                    }
                    messages.insert(message_id.clone(), output.records.len());
                    (
                        *author,
                        *position,
                        *injection,
                        PositionedBody::Message {
                            blocks: positioned_blocks(blocks, *private_kind),
                        },
                    )
                }
                TranscriptEvent::ToolCall {
                    call_id,
                    tool,
                    input,
                    position,
                    ..
                } => {
                    if !calls.insert(call_id.clone()) {
                        output.gaps.push(CoverageGap::MalformedRecord {
                            position: position.byte_offset,
                        });
                        continue 'lines;
                    }
                    (
                        RecordAuthor::Assistant,
                        *position,
                        InjectionOrigin::NotInjected,
                        PositionedBody::ToolCall {
                            call_id: call_id.clone(),
                            tool: tool.clone(),
                            input: input.clone(),
                        },
                    )
                }
                TranscriptEvent::ToolResult {
                    call_id,
                    outcome,
                    result,
                    position,
                } => {
                    if !calls.contains(call_id) {
                        output.gaps.push(CoverageGap::OrphanResult {
                            call_id: call_id.clone(),
                            position: position.byte_offset,
                        });
                    }
                    (
                        RecordAuthor::Tool,
                        *position,
                        InjectionOrigin::NotInjected,
                        PositionedBody::ToolResult {
                            call_id: call_id.clone(),
                            outcome: *outcome,
                            result: result.clone(),
                        },
                    )
                }
                TranscriptEvent::PrivateRecord {
                    author,
                    position,
                    private_kind,
                    text,
                    ..
                } => (
                    *author,
                    *position,
                    InjectionOrigin::NotInjected,
                    PositionedBody::Private {
                        private_kind: *private_kind,
                        text: text.clone(),
                    },
                ),
                TranscriptEvent::Gap { gap } => {
                    output.gaps.push(gap.clone());
                    if matches!(gap, CoverageGap::MalformedRecord { .. }) {
                        continue 'lines;
                    }
                    continue;
                }
            };
            // Several blocks may occupy the same source line; the event ordinal disambiguates them.
            let mut field_positions = BTreeMap::new();
            if let PositionedBody::Message { blocks } = &body {
                for block_index in 0..blocks.len() {
                    field_positions.insert(
                        FieldPath::new(format!("/body/blocks/{block_index}"))?,
                        position,
                    );
                }
            }
            output.records.push(PositionedRecord {
                record_id: PositionedRecordId::new(format!(
                    "{}:{}:{}",
                    source.as_str(),
                    position.byte_offset,
                    output.records.len()
                ))?,
                author,
                positions: vec![position],
                injection,
                body,
                field_positions,
            });
        }
        if let Some(parent) = &line.parent {
            let count = parents.entry(parent.clone()).or_default();
            *count += 1;
            if *count > 1 {
                output.gaps.push(CoverageGap::AmbiguousTurn {
                    position: line.position.byte_offset,
                    reason: AmbiguousReason::UnresolvedBranch,
                });
            }
        }
    }
    Ok(output)
}

fn positioned_blocks(
    blocks: &[MessageBlock],
    private_kind: Option<PrivateKind>,
) -> Vec<PositionedBlock> {
    blocks
        .iter()
        .map(|block| PositionedBlock {
            block: block.clone(),
            private_kind: private_kind.or(if matches!(block, MessageBlock::Thinking { .. }) {
                Some(PrivateKind::Thinking)
            } else {
                None
            }),
        })
        .collect()
}
