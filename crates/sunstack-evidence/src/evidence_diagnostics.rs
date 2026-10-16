use crate::evidence_contracts::{SourceRef, SourceSpan};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOrigin {
    Source,
    Transcript,
    InputBinding,
    Selection,
    Invocation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCode {
    SourceUnavailable,
    PathOutsideRoot,
    UnsupportedEncoding,
    SourceChanged,
    MissingTarget,
    DuplicateTarget,
    DeletedTarget,
    EvidenceLimit,
    MissingContext,
    UnknownTranscriptFormat,
    TranscriptUnavailable,
    TranscriptDiscontinuous,
    InvalidCursor,
    AdapterOutputInvalid,
    TranscriptLimit,
    UnknownPayloadSchema,
    PayloadSchemaMismatch,
    PayloadSchemaInvalid,
    InvalidPointer,
    InvalidSelector,
    UnsupportedLanguage,
    SelectorLimit,
    EmptyProjection,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticLocation {
    None,
    Field {
        field_path: Vec<String>,
    },
    Source {
        source: SourceRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<SourceSpan>,
    },
    Record {
        source: SourceRef,
        record_id: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetryAdvice {
    RetryAfter { milliseconds: u64 },
    RepairRequest,
    RepairSource,
    CaptureAgain,
    RepairConfiguration,
    RestoreProviderAccess,
    DoNotRetry,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub origin: DiagnosticOrigin,
    pub code: EvidenceCode,
    pub location: DiagnosticLocation,
    pub guidance: String,
    pub retry_advice: RetryAdvice,
}
impl Diagnostic {
    /// Guidance is selected from closed static copy, never constructed from received values.
    pub fn new(code: EvidenceCode) -> Self {
        use EvidenceCode::*;
        let (origin, guidance, retry_advice) = match code {
            UnknownTranscriptFormat => (
                DiagnosticOrigin::Transcript,
                "Register a supported transcript format.",
                RetryAdvice::RepairConfiguration,
            ),
            TranscriptUnavailable => (
                DiagnosticOrigin::Transcript,
                "Make the bound transcript readable.",
                RetryAdvice::RepairSource,
            ),
            TranscriptDiscontinuous => (
                DiagnosticOrigin::Transcript,
                "Capture again without the discontinuous cursor.",
                RetryAdvice::CaptureAgain,
            ),
            InvalidCursor => (
                DiagnosticOrigin::Transcript,
                "Supply a cursor returned for this source and format.",
                RetryAdvice::RepairRequest,
            ),
            AdapterOutputInvalid => (
                DiagnosticOrigin::Transcript,
                "Repair the adapter output invariants.",
                RetryAdvice::RepairConfiguration,
            ),
            TranscriptLimit => (
                DiagnosticOrigin::Transcript,
                "Increase the explicit transcript byte limit.",
                RetryAdvice::RepairConfiguration,
            ),
            UnknownPayloadSchema | PayloadSchemaInvalid => (
                DiagnosticOrigin::InputBinding,
                "Supply a valid offline payload schema catalog.",
                RetryAdvice::RepairConfiguration,
            ),
            PayloadSchemaMismatch => (
                DiagnosticOrigin::InputBinding,
                "Repair the payload to match its declared schema.",
                RetryAdvice::RepairRequest,
            ),
            InvalidPointer => (
                DiagnosticOrigin::Selection,
                "Supply a valid RFC 6901 pointer to an existing field.",
                RetryAdvice::RepairRequest,
            ),
            InvalidSelector | EmptyProjection | UnsupportedLanguage => (
                DiagnosticOrigin::Selection,
                "Supply a supported selector with a nonempty projection.",
                RetryAdvice::RepairRequest,
            ),
            SelectorLimit | EvidenceLimit => (
                DiagnosticOrigin::Selection,
                "Increase the explicit selection limit or narrow the declaration.",
                RetryAdvice::RepairConfiguration,
            ),
            PathOutsideRoot => (
                DiagnosticOrigin::Source,
                "Bind a path inside the repository and its allowed input paths.",
                RetryAdvice::RepairRequest,
            ),
            MissingTarget | DuplicateTarget | DeletedTarget => (
                DiagnosticOrigin::Source,
                "Resolve the declared paragraph to one current eligible paragraph.",
                RetryAdvice::RepairSource,
            ),
            SourceChanged => (
                DiagnosticOrigin::Source,
                "Capture the source again after the writer settles.",
                RetryAdvice::CaptureAgain,
            ),
            SourceUnavailable | UnsupportedEncoding | MissingContext => (
                DiagnosticOrigin::Source,
                "Supply complete readable UTF-8 source evidence.",
                RetryAdvice::RepairSource,
            ),
        };
        Self {
            origin,
            code,
            location: DiagnosticLocation::None,
            guidance: guidance.into(),
            retry_advice,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticList {
    pub items: Vec<Diagnostic>,
    pub omitted_count: usize,
}
impl DiagnosticList {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        if self.items.len() < 100 {
            self.items.push(diagnostic);
        } else {
            self.omitted_count += 1;
        }
    }
    pub fn one(code: EvidenceCode) -> Self {
        Self {
            items: vec![Diagnostic::new(code)],
            omitted_count: 0,
        }
    }
}
#[derive(Clone, Debug, thiserror::Error)]
#[error("Evidence preparation refused: {code:?}")]
pub struct EvidenceError {
    pub code: EvidenceCode,
    pub field_path: Vec<String>,
}
impl From<EvidenceCode> for EvidenceError {
    fn from(code: EvidenceCode) -> Self {
        Self {
            code,
            field_path: Vec::new(),
        }
    }
}
pub type EvidenceResult<TValue> = Result<TValue, EvidenceError>;

impl EvidenceError {
    pub fn diagnostic(&self) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(self.code);
        if !self.field_path.is_empty() {
            diagnostic.location = DiagnosticLocation::Field {
                field_path: self.field_path.clone(),
            };
        }
        diagnostic
    }
    pub fn diagnostics(&self) -> DiagnosticList {
        DiagnosticList {
            items: vec![self.diagnostic()],
            omitted_count: 0,
        }
    }
}
