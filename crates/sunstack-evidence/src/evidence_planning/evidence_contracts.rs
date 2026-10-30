use crate::{
    ChangeScope, conversation_evidence::conversation_contracts::*,
    evidence_planning::evidence_diagnostics::*,
    source_acquisition::structured_json::StructuredJson,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf};

macro_rules! validated_name {
    ($name:ident, $validator:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> EvidenceResult<Self> {
                let value = value.into();
                if ($validator)(&value) {
                    Ok(Self(value))
                } else {
                    Err(EvidenceCode::InvalidSelector.into())
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl schemars::JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                let mut schema = schemars::schema_for!(String);
                schema.insert("minLength".into(), serde_json::json!(1));
                match stringify!($name) {
                    "SourceName" | "DeclarationId" => {
                        schema.insert(
                            "pattern".into(),
                            serde_json::json!("^[a-z][a-z0-9_]{0,63}$"),
                        );
                    }
                    "TargetId" => {
                        schema.insert("pattern".into(), serde_json::json!("^[A-Za-z][0-9]{3,}$"));
                    }
                    "MarkerId" => {
                        schema.insert("pattern".into(), serde_json::json!("^[A-Z][0-9]{3}$"));
                    }
                    "MarkerPrefix" => {
                        schema.insert("pattern".into(), serde_json::json!("^[A-Z]$"));
                    }
                    "TranscriptFormatName" => {
                        schema.insert("maxLength".into(), serde_json::json!(128));
                    }
                    _ => {}
                }
                schema
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use validated_name;
fn source_name_valid(value: &str) -> bool {
    value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
validated_name!(SourceName, source_name_valid);
validated_name!(DeclarationId, source_name_valid);
validated_name!(RepositoryId, |value: &str| !value.is_empty());
validated_name!(SchemaName, |value: &str| !value.is_empty());
validated_name!(TargetId, |value: &str| value.len() >= 4
    && value.as_bytes()[0].is_ascii_alphabetic()
    && value.as_bytes()[1..].iter().all(u8::is_ascii_digit));

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceRef {
    RepositoryFile {
        repository: RepositoryId,
        revision: crate::SourceRevision,
        path: PathBuf,
        content_identity: String,
    },
    Document {
        repository: RepositoryId,
        path: PathBuf,
        content_identity: String,
    },
    Transcript {
        path: PathBuf,
        format: TranscriptFormatName,
        committed_bytes: u64,
    },
    Payload {
        group_position: usize,
        input: SourceName,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub source: SourceRef,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceSpec {
    Document {
        repository: RepositoryId,
        path: PathBuf,
    },
    Conversation {
        transcript_path: PathBuf,
        format: TranscriptFormatName,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        final_assistant_text: Option<String>,
    },
    Payload {
        schema: SchemaName,
        value: StructuredJson,
    },
    ChangeSet {
        repository: RepositoryId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryPath {
    Exact { path: PathBuf },
    Glob { pattern: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBinding {
    pub key: RepositoryId,
    pub root: PathBuf,
    pub input_paths: Vec<QueryPath>,
    pub change_scope: ChangeScope,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvocationLimits {
    pub max_files: NonZeroUsize,
    pub max_file_bytes: NonZeroUsize,
    pub max_total_bytes: NonZeroUsize,
    pub max_matches: NonZeroUsize,
    pub max_transcript_bytes: NonZeroUsize,
    pub include_ignored: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRequirement {
    #[default]
    Required,
    Optional,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceDeclaration {
    pub id: DeclarationId,
    pub input: SourceName,
    #[serde(default)]
    pub requirement: EvidenceRequirement,
    pub select: EvidenceSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_items: Option<NonZeroUsize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<NonZeroUsize>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceSelection {
    FrontmatterField {
        field: String,
    },
    // Schemars relaxes nested untagged alternatives when adding the kind tag.
    // Restore the same closed, mutually exclusive filter wire as Serde.
    #[schemars(extend("unevaluatedProperties" = false, "not" = {"required": ["starts_with", "pattern"]}))]
    MarkdownUnits(crate::MarkdownUnitSelection),
    ChangeSetLiteralText {
        paths: Vec<QueryPath>,
        text: String,
    },
    ChangeSetRegexText {
        paths: Vec<QueryPath>,
        pattern: String,
    },
    ChangeSetAstPattern {
        paths: Vec<QueryPath>,
        language: String,
        pattern: String,
    },
    ChangedUnits {
        languages: Vec<crate::SourceLanguage>,
        unit_kinds: Vec<crate::UnitKindFilter>,
        source_sides: crate::SourceSides,
    },
    RelatedUnits {
        languages: Vec<crate::GraphLanguage>,
        unit_kinds: Vec<crate::UnitKindFilter>,
        source_sides: crate::SourceSides,
        directions: Vec<crate::RelationDirection>,
        requested_hops: NonZeroUsize,
        chunk_settings: crate::ChunkSettings,
        minimum_related_units: usize,
    },
    Paragraph {
        target_id: TargetId,
    },
    WholeDocument {},
    LiteralText {
        text: String,
    },
    RegexText {
        pattern: String,
    },
    AstPattern {
        language: String,
        pattern: String,
    },
    Conversation {
        turns: TurnWindow,
        user_messages: Inclusion,
        assistant_messages: AssistantSelection,
        tool_use: ToolUseSelection,
        hook_injected: Inclusion,
    },
    WholePayload {},
    PayloadField {
        pointer: String,
    },
}
impl EvidenceSelection {
    pub fn conversation(&self) -> Option<ConversationSelection> {
        if let Self::Conversation {
            turns,
            user_messages,
            assistant_messages,
            tool_use,
            hook_injected,
        } = self
        {
            Some(ConversationSelection {
                turns: turns.clone(),
                user_messages: *user_messages,
                assistant_messages: *assistant_messages,
                tool_use: tool_use.clone(),
                hook_injected: *hook_injected,
            })
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureCoverage {
    pub source: SourceRef,
    pub cursor_basis: CursorBasis,
    pub tail: CaptureTail,
    pub gaps: Vec<CoverageGap>,
    pub bytes_read: u64,
    pub bytes_parsed: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CursorBasis {
    NoCursorSupplied,
    ContinuedFromCursor,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CaptureTail {
    Complete,
    Incomplete { bytes: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnedCursor {
    pub transcript_path: PathBuf,
    pub format: TranscriptFormatName,
    pub cursor: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OmissionReason {
    Window,
    Projection,
    Limit,
    OptionalUnavailable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Omission {
    pub reason: OmissionReason,
    pub count: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatch {
    pub span: SourceSpan,
    pub text: String,
    pub captures: BTreeMap<String, SourceSpan>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionContent {
    FrontmatterField {
        result: Box<crate::FrontmatterFieldContent>,
    },
    MarkdownUnits {
        units: Vec<crate::SelectedMarkdownUnit>,
    },
    RelatedGroups {
        groups: Vec<crate::SeedContextGroup>,
    },
    ChangedUnits {
        units: Vec<crate::SelectedSourceUnit>,
    },
    RelatedGroup {
        group: Box<crate::ContextGroupOutcome>,
    },
    Paragraph {
        span: SourceSpan,
        text: String,
    },
    DocumentText {
        span: SourceSpan,
        text: String,
    },
    Matches {
        matches: Vec<EvidenceMatch>,
    },
    ConversationRecords {
        source: SourceRef,
        records: Vec<ConversationRecord>,
    },
    PayloadValue {
        input: SourceName,
        pointer: String,
        value: StructuredJson,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionReadiness {
    Ready {
        content: SelectionContent,
        omissions: Vec<Omission>,
        coverage: Box<crate::SelectionCoverage>,
    },
    Refused {
        diagnostics: DiagnosticList,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectionResult {
    pub id: DeclarationId,
    pub readiness: SelectionReadiness,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetLocation {
    FrontmatterField {
        source: SourceRef,
        field: String,
        state: Box<crate::FrontmatterFieldLocation>,
    },
    MarkdownUnit {
        occurrence: Box<crate::MarkdownOccurrence>,
    },
    CodeUnits {
        units: Vec<crate::SourceUnitKey>,
    },
    Paragraph {
        span: SourceSpan,
        target_id: TargetId,
    },
    ConversationRecords {
        source: SourceRef,
        record_ids: Vec<RecordId>,
    },
    PayloadField {
        input: SourceName,
        pointer: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetResolution {
    Resolved { target: TargetLocation },
    Missing,
    Ambiguous,
    Deleted,
    Unavailable,
}
