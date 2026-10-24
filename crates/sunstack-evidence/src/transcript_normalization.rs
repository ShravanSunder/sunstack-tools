use crate::*;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
struct NormalizationState {
    output: AdapterOutput,
    messages: BTreeMap<RecordId, usize>,
    calls: BTreeMap<CallId, usize>,
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
    fn call(
        &mut self,
        id: CallId,
        name: String,
        input: StructuredJson,
        position: u64,
        tool_category: ToolCategory,
    ) -> EvidenceResult<()> {
        if self.calls.contains_key(&id) {
            return Err(EvidenceCode::AdapterOutputInvalid.into());
        }
        let record_id = RecordId::new(format!("tool:{}", id.as_str()))?;
        self.calls.insert(id.clone(), self.output.records.len());
        self.add_record(ConversationRecord {
            record_id,
            position,
            body: ConversationBody::ToolUse(ToolUse {
                call_id: id,
                category: tool_category,
                tool_name: name,
                input,
                outcome: ToolOutcome::ResultMissing,
                result: ToolResult::Absent,
            }),
            provenance: RecordProvenance::Transcript,
            injection: InjectionOrigin::NotInjected,
        });
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
    fn claude(&mut self, value: Value, position: u64) -> EvidenceResult<()> {
        let kind = field(&value, "type")?;
        if kind != "user" && kind != "assistant" {
            self.output
                .gaps
                .push(CoverageGap::UnsupportedRecord { position });
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
            .unwrap_or_else(|| format!("message:{position}"));
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
                        self.output
                            .gaps
                            .push(CoverageGap::UnsupportedRecord { position });
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
                    category(field(item, "name")?, false),
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
                self.output.gaps.push(CoverageGap::AmbiguousTurn {
                    position,
                    reason: AmbiguousReason::UnresolvedBranch,
                });
                if let Some(turn) = self.output.turns.last_mut() {
                    turn.boundary = TurnBoundary::Ambiguous {
                        reason: AmbiguousReason::UnresolvedBranch,
                    };
                }
            }
        }
        Ok(())
    }
    fn codex(&mut self, value: Value, position: u64) -> EvidenceResult<()> {
        if field(&value, "type")? != "response_item" {
            self.output
                .gaps
                .push(CoverageGap::UnsupportedRecord { position });
            return Ok(());
        }
        let payload = value
            .get("payload")
            .ok_or(EvidenceCode::AdapterOutputInvalid)?;
        match field(payload, "type")? {
            "message" => {
                let role = field(payload, "role")?;
                if role != "assistant" && role != "user" {
                    self.output
                        .gaps
                        .push(CoverageGap::UnsupportedRecord { position });
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
                            self.output
                                .gaps
                                .push(CoverageGap::UnsupportedRecord { position });
                        }
                    }
                }
                self.message(
                    RecordId::new(format!("message:{position}"))?,
                    position,
                    role == "assistant",
                    blocks,
                    if role == "assistant" {
                        InjectionOrigin::NotInjected
                    } else {
                        InjectionOrigin::Unknown
                    },
                )?;
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
                    category(name, namespace.is_some()),
                )?;
            }
            "custom_tool_call" => {
                let name = field(payload, "name")?;
                let tool_category = match name {
                    "apply_patch" => ToolCategory::EditFile,
                    "exec" | "Bash" | "exec_command" | "shell_command" => ToolCategory::Shell,
                    _ => ToolCategory::Other,
                };
                self.call(
                    CallId::new(field(payload, "call_id")?)?,
                    name.into(),
                    serde_json::from_value(
                        payload
                            .get("input")
                            .ok_or(EvidenceCode::AdapterOutputInvalid)?
                            .clone(),
                    )
                    .map_err(|_| EvidenceCode::AdapterOutputInvalid)?,
                    position,
                    tool_category,
                )?;
            }
            "custom_tool_call_output" => {
                let outcome = match payload.get("is_error") {
                    Some(Value::Bool(true)) => ToolOutcome::Failed,
                    Some(Value::Bool(false)) => ToolOutcome::Succeeded,
                    None => ToolOutcome::ResultPresentStatusUnknown,
                    _ => return Err(EvidenceCode::AdapterOutputInvalid.into()),
                };
                self.result(
                    CallId::new(field(payload, "call_id")?)?,
                    ToolResult::FullObserved {
                        text: observed_text(
                            payload
                                .get("output")
                                .ok_or(EvidenceCode::AdapterOutputInvalid)?,
                        )?,
                    },
                    outcome,
                    position,
                );
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
            _ => self
                .output
                .gaps
                .push(CoverageGap::UnsupportedRecord { position }),
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
pub(crate) fn normalize_builtin(format: &str, bytes: &[u8]) -> AdapterOutput {
    let mut state = NormalizationState::default();
    let mut position = 0_u64;
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        let parsed = serde_json::from_slice(line);
        let result = match parsed {
            Ok(value) if format == "claude_code_jsonl" => state.claude(value, position),
            Ok(value) => state.codex(value, position),
            Err(_) => Err(EvidenceCode::AdapterOutputInvalid.into()),
        };
        if result.is_err() {
            state
                .output
                .gaps
                .push(CoverageGap::MalformedRecord { position });
        }
        position += line.len() as u64;
    }
    state.output
}
