use crate::{evidence_contracts::*, evidence_diagnostics::*, structured_json::StructuredJson};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf};
validated_name!(TranscriptFormatName, |value: &str| !value.is_empty()
    && value.len() <= 128);
validated_name!(RecordId, |value: &str| !value.is_empty());
validated_name!(CallId, |value: &str| !value.is_empty());
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Inclusion {
    Included,
    Excluded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssistantSelection {
    Excluded,
    All,
    FinalOnly,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TurnWindow {
    All {},
    Last { count: NonZeroUsize },
    SinceCursor {},
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolFilter {
    All {},
    Named { names: Vec<String> },
    Categories { categories: Vec<ToolCategory> },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolUseSelection {
    Excluded {},
    Included {
        tools: ToolFilter,
        results: Inclusion,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversationSelection {
    pub turns: TurnWindow,
    pub user_messages: Inclusion,
    pub assistant_messages: AssistantSelection,
    pub tool_use: ToolUseSelection,
    pub hook_injected: Inclusion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    ReadFile,
    EditFile,
    Shell,
    Search,
    Web,
    Mcp,
    Other,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Succeeded,
    Failed,
    ResultPresentStatusUnknown,
    ResultMissing,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OriginalBytes {
    Known { bytes: usize },
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolResult {
    FullObserved {
        text: String,
    },
    Clipped {
        text: String,
        original_bytes: OriginalBytes,
    },
    Absent,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolUse {
    pub call_id: CallId,
    pub tool_name: String,
    pub category: ToolCategory,
    pub input: StructuredJson,
    pub outcome: ToolOutcome,
    pub result: ToolResult,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MessageBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolCall {
        call_id: CallId,
    },
    Unsupported {
        block_type: String,
        content: StructuredJson,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversationBody {
    UserMessage { blocks: Vec<MessageBlock> },
    AssistantMessage { blocks: Vec<MessageBlock> },
    ToolUse(ToolUse),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordProvenance {
    Transcript,
    HookSupplement,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InjectionOrigin {
    KnownInjected,
    NotInjected,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversationRecord {
    pub record_id: RecordId,
    pub position: u64,
    pub body: ConversationBody,
    pub provenance: RecordProvenance,
    pub injection: InjectionOrigin,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguousReason {
    ConsecutiveUserMessages,
    UnresolvedBranch,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TurnOpening {
    UserMessage { record_id: RecordId },
    CaptureStart,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TurnBoundary {
    Clear,
    Ambiguous { reason: AmbiguousReason },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversationTurn {
    pub opening: TurnOpening,
    pub records: Vec<RecordId>,
    pub boundary: TurnBoundary,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CoverageGap {
    UnsupportedRecord {
        position: u64,
    },
    MalformedRecord {
        position: u64,
    },
    OrphanResult {
        call_id: CallId,
        position: u64,
    },
    AmbiguousTurn {
        position: u64,
        reason: AmbiguousReason,
    },
    UnknownInjection {
        record_id: RecordId,
    },
}
/// Source identity includes the file object, so same-bytes replacement invalidates a cursor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TranscriptSource {
    pub path: PathBuf,
    pub file_identity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MonitoringCursor {
    pub source: TranscriptSource,
    pub format: TranscriptFormatName,
    pub committed_bytes: u64,
    pub prefix_digest: String,
    pub record_watermarks: BTreeMap<RecordId, u64>,
    pub supplement_digest: Option<String>,
}
/// The adapter receives immutable committed bytes and identity, never a path or open handle.
pub struct CapturedTranscript<'capture> {
    pub bytes: &'capture [u8],
    pub source_identity: &'capture str,
}
#[derive(Clone, Debug, Default)]
pub struct AdapterOutput {
    pub records: Vec<ConversationRecord>,
    pub turns: Vec<ConversationTurn>,
    pub gaps: Vec<CoverageGap>,
}
#[derive(Clone, Debug)]
pub struct ConversationCapture {
    pub interval: crate::source_capture::CaptureInterval,
    pub source: TranscriptSource,
    pub format: TranscriptFormatName,
    pub committed_bytes: u64,
    pub records: Vec<ConversationRecord>,
    pub turns: Vec<ConversationTurn>,
    pub tail: CaptureTail,
    pub gaps: Vec<CoverageGap>,
    pub supplement: Option<ConversationRecord>,
    pub coverage: CaptureCoverage,
    pub cursor: MonitoringCursor,
    pub(crate) previous_cursor: Option<MonitoringCursor>,
}
