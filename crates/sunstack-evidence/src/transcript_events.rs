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

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrivateKind {
    Thinking,
    AnalysisChannel,
    EncryptedReasoning,
    CredentialLike,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        private_kind: Option<PrivateKind>,
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
    PrivateRecord {
        record_id: RecordId,
        author: RecordAuthor,
        position: SourcePosition,
        private_kind: PrivateKind,
        text: Option<String>,
    },
    CustomRecord {
        injection: InjectionOrigin,
        author: RecordAuthor,
        field_privacy: std::collections::BTreeMap<String, MappedFieldPrivacy>,
        fields: std::collections::BTreeMap<String, StructuredJson>,
        position: SourcePosition,
    },
    Gap {
        gap: CoverageGap,
    },
}

#[derive(Clone, Debug)]
pub enum LegacyDisposition {
    /// Replay the old parser's events, including any partial effects before a syntax error.
    Folded,
    RejectedWhole {
        reason: CoverageGap,
        preceding_gaps: Vec<CoverageGap>,
    },
}

#[derive(Clone, Debug)]
pub struct SourceLine {
    pub position: SourcePosition,
    pub legacy: LegacyDisposition,
    pub events: Vec<TranscriptEvent>,
    pub parent: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranscriptFraming {
    JsonLines,
    JsonDocument,
}

/// Read source envelopes before either consumer folds messages or joins tool results.
pub trait TranscriptEventReader: Send + Sync {
    fn format_name(&self) -> &TranscriptFormatName;
    fn framing(&self) -> TranscriptFraming {
        TranscriptFraming::JsonLines
    }
    fn lines(&self, capture: &CapturedTranscript<'_>) -> Vec<SourceLine>;
    fn events(&self, capture: &CapturedTranscript<'_>) -> Vec<TranscriptEvent> {
        self.lines(capture)
            .into_iter()
            .flat_map(|line| line.events)
            .collect()
    }
}
