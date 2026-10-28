use crate::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct NormalizationState {
    output: AdapterOutput,
    messages: BTreeMap<RecordId, usize>,
    calls: BTreeMap<CallId, usize>,
    parents: BTreeMap<String, usize>,
}
impl NormalizationState {
    fn add_record(&mut self, record: ConversationRecord) {
        if matches!(record.body, ConversationBody::UserMessage { .. }) {
            let consecutive = self.output.records.last().is_some_and(|previous| {
                matches!(previous.body, ConversationBody::UserMessage { .. })
            });
            let boundary = if consecutive {
                self.output.gaps.push(CoverageGap::AmbiguousTurn {
                    position: record.position,
                    reason: AmbiguousReason::ConsecutiveUserMessages,
                });
                TurnBoundary::Ambiguous {
                    reason: AmbiguousReason::ConsecutiveUserMessages,
                }
            } else {
                TurnBoundary::Clear
            };
            self.output.turns.push(ConversationTurn {
                opening: TurnOpening::UserMessage {
                    record_id: record.record_id.clone(),
                },
                records: Vec::new(),
                boundary,
            });
        }
        if self.output.turns.is_empty() {
            self.output.turns.push(ConversationTurn {
                opening: TurnOpening::CaptureStart,
                records: Vec::new(),
                boundary: TurnBoundary::Clear,
            });
        }
        if let Some(turn) = self.output.turns.last_mut() {
            turn.records.push(record.record_id.clone());
        }
        if record.injection == InjectionOrigin::Unknown {
            self.output.gaps.push(CoverageGap::UnknownInjection {
                record_id: record.record_id.clone(),
            });
        }
        self.output.records.push(record);
    }
    fn message(
        &mut self,
        id: RecordId,
        position: u64,
        assistant: bool,
        blocks: Vec<MessageBlock>,
        injection: InjectionOrigin,
    ) -> EvidenceResult<()> {
        if let Some(index) = self.messages.get(&id).copied() {
            let record = &mut self.output.records[index];
            let old_blocks = match (&mut record.body, assistant) {
                (ConversationBody::AssistantMessage { blocks }, true)
                | (ConversationBody::UserMessage { blocks }, false) => blocks,
                _ => return Err(EvidenceCode::AdapterOutputInvalid.into()),
            };
            old_blocks.extend(blocks);
            record.position = position;
        } else {
            self.messages.insert(id.clone(), self.output.records.len());
            let body = if assistant {
                ConversationBody::AssistantMessage { blocks }
            } else {
                ConversationBody::UserMessage { blocks }
            };
            self.add_record(ConversationRecord {
                record_id: id,
                position,
                body,
                provenance: RecordProvenance::Transcript,
                injection,
            });
        }
        Ok(())
    }
    fn result(&mut self, id: CallId, result: ToolResult, outcome: ToolOutcome, position: u64) {
        if let Some(index) = self.calls.get(&id).copied() {
            let record = &mut self.output.records[index];
            if let ConversationBody::ToolUse(tool) = &mut record.body {
                tool.result = result;
                tool.outcome = outcome;
                record.position = position;
            }
        } else {
            self.output.gaps.push(CoverageGap::OrphanResult {
                call_id: id,
                position,
            });
        }
    }
}
pub(crate) fn fold_transcript_lines(lines: Vec<SourceLine>) -> AdapterOutput {
    let mut state = NormalizationState::default();
    'lines: for line in lines {
        if let LegacyDisposition::RejectedWhole {
            reason,
            preceding_gaps,
        } = line.legacy
        {
            state.output.gaps.extend(preceding_gaps);
            state.output.gaps.push(reason);
            continue;
        }
        for event in line.events {
            match event {
                TranscriptEvent::MessagePart {
                    message_id,
                    author,
                    position,
                    blocks,
                    injection,
                    ..
                } => {
                    // Each consumer owns message identity; rejected legacy lines never reach it.
                    if state
                        .message(
                            message_id,
                            position.byte_offset,
                            author == RecordAuthor::Assistant,
                            blocks,
                            injection,
                        )
                        .is_err()
                    {
                        state.output.gaps.push(CoverageGap::MalformedRecord {
                            position: position.byte_offset,
                        });
                        continue 'lines;
                    }
                }
                TranscriptEvent::ToolCall {
                    call_id,
                    tool,
                    category,
                    input,
                    position,
                } => {
                    if state.calls.contains_key(&call_id) {
                        state.output.gaps.push(CoverageGap::MalformedRecord {
                            position: position.byte_offset,
                        });
                        continue 'lines;
                    }
                    let record_id = RecordId::new(format!("tool:{}", call_id.as_str()));
                    if let Ok(record_id) = record_id {
                        state
                            .calls
                            .insert(call_id.clone(), state.output.records.len());
                        state.add_record(ConversationRecord {
                            record_id,
                            position: position.byte_offset,
                            body: ConversationBody::ToolUse(ToolUse {
                                call_id,
                                tool_name: tool.as_str().into(),
                                category,
                                input,
                                outcome: ToolOutcome::ResultMissing,
                                result: ToolResult::Absent,
                            }),
                            provenance: RecordProvenance::Transcript,
                            injection: InjectionOrigin::NotInjected,
                        });
                    }
                }
                TranscriptEvent::ToolResult {
                    call_id,
                    outcome,
                    result,
                    position,
                } => state.result(call_id, result, outcome, position.byte_offset),
                TranscriptEvent::PrivateRecord { position, .. }
                | TranscriptEvent::CustomRecord { position, .. } => {
                    state.output.gaps.push(CoverageGap::UnsupportedRecord {
                        position: position.byte_offset,
                    });
                }
                TranscriptEvent::Gap { gap } => {
                    if matches!(
                        gap,
                        CoverageGap::AmbiguousTurn {
                            reason: AmbiguousReason::UnresolvedBranch,
                            ..
                        }
                    ) && let Some(turn) = state.output.turns.last_mut()
                    {
                        turn.boundary = TurnBoundary::Ambiguous {
                            reason: AmbiguousReason::UnresolvedBranch,
                        };
                    }
                    let malformed = matches!(gap, CoverageGap::MalformedRecord { .. });
                    state.output.gaps.push(gap);
                    if malformed {
                        continue 'lines;
                    }
                }
            }
        }
        if let Some(parent) = line.parent {
            let count = state.parents.entry(parent).or_default();
            *count += 1;
            if *count > 1 {
                state.output.gaps.push(CoverageGap::AmbiguousTurn {
                    position: line.position.byte_offset,
                    reason: AmbiguousReason::UnresolvedBranch,
                });
                if let Some(turn) = state.output.turns.last_mut() {
                    turn.boundary = TurnBoundary::Ambiguous {
                        reason: AmbiguousReason::UnresolvedBranch,
                    };
                }
            }
        }
    }
    state.output
}

pub(crate) fn normalize_builtin(format: &str, bytes: &[u8]) -> AdapterOutput {
    fold_transcript_lines(crate::transcript_event_reading::read_builtin_lines(
        format, bytes,
    ))
}
