use crate::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct EventReadingState {
    events: Vec<TranscriptEvent>,
    messages: BTreeMap<RecordId, RecordAuthor>,
    calls: BTreeSet<CallId>,
    parents: BTreeMap<String, usize>,
}
fn field<'value>(value: &'value Value, key: &str) -> EvidenceResult<&'value str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| EvidenceCode::AdapterOutputInvalid.into())
}
fn category(name: &str, namespaced: bool) -> ToolCategory {
    if namespaced || name.starts_with("mcp__") {
        return ToolCategory::Mcp;
    }
    match name {
        "Read" => ToolCategory::ReadFile,
        "Edit" | "Write" => ToolCategory::EditFile,
        "Bash" | "exec_command" | "shell_command" => ToolCategory::Shell,
        "Search" | "Grep" | "Glob" => ToolCategory::Search,
        "WebSearch" | "WebFetch" => ToolCategory::Web,
        _ => ToolCategory::Unknown,
    }
}
impl EventReadingState {
    fn gap(&mut self, gap: CoverageGap) {
        self.events.push(TranscriptEvent::Gap { gap });
    }
    fn message(
        &mut self,
        message_id: RecordId,
        position: SourcePosition,
        assistant: bool,
        blocks: Vec<MessageBlock>,
        injection: InjectionOrigin,
    ) -> EvidenceResult<()> {
        let author = if assistant {
            RecordAuthor::Assistant
        } else {
            RecordAuthor::User
        };
        if self
            .messages
            .get(&message_id)
            .is_some_and(|previous| *previous != author)
        {
            return Err(EvidenceCode::AdapterOutputInvalid.into());
        }
        self.messages.insert(message_id.clone(), author);
        self.events.push(TranscriptEvent::MessagePart {
            message_id,
            author,
            position,
            blocks,
            injection,
            private_kind: None,
        });
        Ok(())
    }
    fn call(
        &mut self,
        call_id: CallId,
        name: String,
        input: StructuredJson,
        position: SourcePosition,
        namespaced: bool,
    ) -> EvidenceResult<()> {
        if !self.calls.insert(call_id.clone()) {
            return Err(EvidenceCode::AdapterOutputInvalid.into());
        }
        let category = category(&name, namespaced);
        self.events.push(TranscriptEvent::ToolCall {
            call_id,
            tool: ToolName::new(name)?,
            category,
            input,
            position,
        });
        Ok(())
    }
    fn result(
        &mut self,
        call_id: CallId,
        result: ToolResult,
        outcome: ToolOutcome,
        position: SourcePosition,
    ) {
        let orphan = !self.calls.contains(&call_id);
        let orphan_id = call_id.clone();
        self.events.push(TranscriptEvent::ToolResult {
            call_id,
            result,
            outcome,
            position,
        });
        if orphan {
            self.gap(CoverageGap::OrphanResult {
                call_id: orphan_id,
                position: position.byte_offset,
            });
        }
    }
    fn claude(&mut self, value: Value, position: SourcePosition) -> EvidenceResult<()> {
        let kind = field(&value, "type")?;
        if kind != "user" && kind != "assistant" {
            self.gap(CoverageGap::UnsupportedRecord {
                position: position.byte_offset,
            });
            return Ok(());
        }
        let assistant = kind == "assistant";
        let message = value
            .get("message")
            .ok_or(EvidenceCode::AdapterOutputInvalid)?;
        let id = message
            .get("id")
            .or_else(|| value.get("uuid"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("message:{}", position.byte_offset));
        let id = RecordId::new(id)?;
        let injection = if assistant {
            InjectionOrigin::NotInjected
        } else {
            match value
                .get("origin")
                .and_then(|origin| origin.get("kind"))
                .and_then(Value::as_str)
            {
                Some("hook") => InjectionOrigin::KnownInjected,
                Some("user") => InjectionOrigin::NotInjected,
                _ => InjectionOrigin::Unknown,
            }
        };
        let content = message
            .get("content")
            .ok_or(EvidenceCode::AdapterOutputInvalid)?;
        let mut blocks = Vec::new();
        let mut deferred = Vec::new();
        if let Some(text) = content.as_str() {
            blocks.push(MessageBlock::Text { text: text.into() });
        } else if let Some(items) = content.as_array() {
            for item in items {
                match field(item, "type")? {
                    "text" => blocks.push(MessageBlock::Text {
                        text: item
                            .get("text")
                            .and_then(Value::as_str)
                            .ok_or(EvidenceCode::AdapterOutputInvalid)?
                            .into(),
                    }),
                    "thinking" => blocks.push(MessageBlock::Thinking {
                        text: field(item, "thinking")?.into(),
                    }),
                    "tool_use" => {
                        if assistant {
                            blocks.push(MessageBlock::ToolCall {
                                call_id: CallId::new(field(item, "id")?)?,
                            });
                        }
                        deferred.push(item);
                    }
                    "tool_result" => deferred.push(item),
                    block_type => {
                        blocks.push(MessageBlock::Unsupported {
                            block_type: block_type.into(),
                            content: serde_json::from_value(item.clone())
                                .map_err(|_| EvidenceCode::AdapterOutputInvalid)?,
                        });
                        self.gap(CoverageGap::UnsupportedRecord {
                            position: position.byte_offset,
                        });
                    }
                }
            }
        } else {
            return Err(EvidenceCode::AdapterOutputInvalid.into());
        }
        if !blocks.is_empty() {
            self.message(id, position, assistant, blocks, injection)?;
        }
        for item in deferred {
            match field(item, "type")? {
                "tool_use" => self.call(
                    CallId::new(field(item, "id")?)?,
                    field(item, "name")?.into(),
                    serde_json::from_value(
                        item.get("input")
                            .ok_or(EvidenceCode::AdapterOutputInvalid)?
                            .clone(),
                    )
                    .map_err(|_| EvidenceCode::AdapterOutputInvalid)?,
                    position,
                    false,
                )?,
                "tool_result" => {
                    let content = item
                        .get("content")
                        .ok_or(EvidenceCode::AdapterOutputInvalid)?;
                    let text = observed_text(content)?;
                    let result = if item.get("is_truncated").and_then(Value::as_bool) == Some(true)
                    {
                        ToolResult::Clipped {
                            text,
                            original_bytes: item
                                .get("original_bytes")
                                .and_then(Value::as_u64)
                                .and_then(|bytes| usize::try_from(bytes).ok())
                                .map_or(OriginalBytes::Unknown, |bytes| OriginalBytes::Known {
                                    bytes,
                                }),
                        }
                    } else {
                        ToolResult::FullObserved { text }
                    };
                    let outcome = match item.get("is_error") {
                        Some(Value::Bool(true)) => ToolOutcome::Failed,
                        Some(Value::Bool(false)) | None => ToolOutcome::Succeeded,
                        _ => return Err(EvidenceCode::AdapterOutputInvalid.into()),
                    };
                    self.result(
                        CallId::new(field(item, "tool_use_id")?)?,
                        result,
                        outcome,
                        position,
                    );
                }
                _ => {}
            }
        }
        if let Some(parent) = value.get("parentUuid").and_then(Value::as_str) {
            let count = self.parents.entry(parent.into()).or_default();
            *count += 1;
            if *count > 1 {
                self.gap(CoverageGap::AmbiguousTurn {
                    position: position.byte_offset,
                    reason: AmbiguousReason::UnresolvedBranch,
                });
            }
        }
        Ok(())
    }
    fn codex(&mut self, value: Value, position: SourcePosition) -> EvidenceResult<()> {
        if field(&value, "type")? != "response_item" {
            self.gap(CoverageGap::UnsupportedRecord {
                position: position.byte_offset,
            });
            return Ok(());
        }
        let payload = value
            .get("payload")
            .ok_or(EvidenceCode::AdapterOutputInvalid)?;
        match field(payload, "type")? {
            "message" => {
                let role = field(payload, "role")?;
                if role != "assistant" && role != "user" {
                    self.gap(CoverageGap::UnsupportedRecord {
                        position: position.byte_offset,
                    });
                    return Ok(());
                }
                let content = payload
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or(EvidenceCode::AdapterOutputInvalid)?;
                let mut blocks = Vec::new();
                for block in content {
                    match field(block, "type")? {
                        "input_text" | "output_text" => blocks.push(MessageBlock::Text {
                            text: block
                                .get("text")
                                .and_then(Value::as_str)
                                .ok_or(EvidenceCode::AdapterOutputInvalid)?
                                .into(),
                        }),
                        block_type => {
                            blocks.push(MessageBlock::Unsupported {
                                block_type: block_type.into(),
                                content: serde_json::from_value(block.clone())
                                    .map_err(|_| EvidenceCode::AdapterOutputInvalid)?,
                            });
                            self.gap(CoverageGap::UnsupportedRecord {
                                position: position.byte_offset,
                            });
                        }
                    }
                }
                self.message(
                    RecordId::new(format!("message:{}", position.byte_offset))?,
                    position,
                    role == "assistant",
                    blocks,
                    if role == "assistant" {
                        InjectionOrigin::NotInjected
                    } else {
                        InjectionOrigin::Unknown
                    },
                )?;
                if payload.get("channel").and_then(Value::as_str) == Some("analysis")
                    && let Some(TranscriptEvent::MessagePart { private_kind, .. }) =
                        self.events.last_mut()
                {
                    *private_kind = Some(PrivateKind::AnalysisChannel);
                }
            }
            "reasoning" => {
                let text = payload
                    .get("summary")
                    .and_then(Value::as_array)
                    .map(|summary| {
                        summary
                            .iter()
                            .filter_map(|part| part.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .filter(|text| !text.is_empty());
                self.events.push(TranscriptEvent::PrivateRecord {
                    record_id: RecordId::new(format!("reasoning:{}", position.byte_offset))?,
                    author: RecordAuthor::Assistant,
                    position,
                    private_kind: PrivateKind::EncryptedReasoning,
                    text,
                });
            }
            "function_call" => {
                let arguments = payload
                    .get("arguments")
                    .ok_or(EvidenceCode::AdapterOutputInvalid)?;
                let input = if let Some(text) = arguments.as_str() {
                    serde_json::from_str(text)
                        .unwrap_or_else(|_| StructuredJson::String(text.into()))
                } else {
                    serde_json::from_value(arguments.clone())
                        .map_err(|_| EvidenceCode::AdapterOutputInvalid)?
                };
                let name = field(payload, "name")?;
                let namespace = payload.get("namespace").and_then(Value::as_str);
                self.call(
                    CallId::new(field(payload, "call_id")?)?,
                    namespace
                        .map_or_else(|| name.into(), |namespace| format!("{namespace}.{name}")),
                    input,
                    position,
                    namespace.is_some(),
                )?;
            }
            "function_call_output" => self.result(
                CallId::new(field(payload, "call_id")?)?,
                ToolResult::FullObserved {
                    text: observed_text(
                        payload
                            .get("output")
                            .ok_or(EvidenceCode::AdapterOutputInvalid)?,
                    )?,
                },
                ToolOutcome::ResultPresentStatusUnknown,
                position,
            ),
            _ => self.gap(CoverageGap::UnsupportedRecord {
                position: position.byte_offset,
            }),
        }
        Ok(())
    }
}
fn observed_text(value: &Value) -> EvidenceResult<String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        _ => serde_json::to_string(value).map_err(|_| EvidenceCode::AdapterOutputInvalid.into()),
    }
}
pub(crate) fn read_builtin_events(format: &str, bytes: &[u8]) -> Vec<TranscriptEvent> {
    let mut state = EventReadingState::default();
    let mut byte_offset = 0_u64;
    for (line_index, line) in bytes.split_inclusive(|byte| *byte == b'\n').enumerate() {
        let position = SourcePosition {
            byte_offset,
            line: line_index as u64 + 1,
        };
        let parsed = serde_json::from_slice(line);
        let result = match parsed {
            Ok(value) if format == "claude_code_jsonl" => state.claude(value, position),
            Ok(value) => state.codex(value, position),
            Err(_) => Err(EvidenceCode::AdapterOutputInvalid.into()),
        };
        if result.is_err() {
            state.gap(CoverageGap::MalformedRecord {
                position: byte_offset,
            });
        }
        byte_offset += line.len() as u64;
    }
    state.events
}
