use crate::{EvidenceCode, EvidenceResult, evidence_contracts::validated_name};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

validated_name!(MarkerId, |value: &str| value.len() == 4
    && value.as_bytes()[0].is_ascii_uppercase()
    && value.as_bytes()[1..].iter().all(u8::is_ascii_digit));
validated_name!(MarkerPrefix, |value: &str| value.len() == 1
    && value.as_bytes()[0].is_ascii_uppercase());

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownHostKind {
    Section,
    Paragraph,
    ListItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkdownTextSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl MarkdownTextSpan {
    pub(crate) fn from_range(text: &str, range: std::ops::Range<usize>) -> Self {
        let (start_line, start_column) = crate::source_capture::text_position(text, range.start);
        let (end_line, end_column) = crate::source_capture::text_position(text, range.end);
        Self {
            byte_start: range.start,
            byte_end: range.end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkdownMarker {
    pub id: MarkerId,
    pub span: MarkdownTextSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundMarkdownText {
    pub marker: MarkdownMarker,
    pub host: MarkdownHostKind,
    pub span: MarkdownTextSpan,
    pub question_text: String,
    pub filter_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarkdownBindingRefusal {
    ForbiddenHost,
    SpacedHeading,
    DuplicateId,
    QuestionTooLarge {
        word_count: usize,
        max_question_words: usize,
    },
}

impl MarkdownBindingRefusal {
    pub fn guidance(&self) -> &'static str {
        match self {
            Self::ForbiddenHost => {
                "Move the marker to an allowed heading, paragraph or list item outside forbidden containers."
            }
            Self::SpacedHeading => {
                "Place the marker directly against the heading text, without whitespace before it."
            }
            Self::DuplicateId => "Use each marker ID at most once in a Markdown source.",
            Self::QuestionTooLarge { .. } => {
                "Use a narrower host for this marker or increase max_question_words."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarkdownMarkerBinding {
    Bound {
        binding: BoundMarkdownText,
    },
    Refused {
        marker: MarkdownMarker,
        reason: MarkdownBindingRefusal,
    },
}
