use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

validated_name!(ToolName, |value: &str| !value.is_empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourcePosition {
    pub byte_offset: u64,
    pub line: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordAuthor {
    User,
    Assistant,
    Tool,
    System,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TranscriptEvent {
    MessagePart {
        message_id: RecordId,
        author: RecordAuthor,
        position: SourcePosition,
        blocks: Vec<MessageBlock>,
        injection: InjectionOrigin,
    },
    ToolCall {
        call_id: CallId,
        tool: ToolName,
        category: ToolCategory,
        input: StructuredJson,
        position: SourcePosition,
    },
    ToolResult {
        call_id: CallId,
        outcome: ToolOutcome,
        result: ToolResult,
        position: SourcePosition,
    },
    Gap {
        gap: CoverageGap,
    },
}

/// Read source envelopes before either consumer folds messages or joins tool results.
pub trait TranscriptEventReader: Send + Sync {
    fn format_name(&self) -> &TranscriptFormatName;
    fn events(&self, capture: &CapturedTranscript<'_>) -> Vec<TranscriptEvent>;
}
