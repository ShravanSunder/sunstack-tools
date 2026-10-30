use crate::{DiagnosticList, SourceRef, SourceSpan, StructuredJson};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FrontmatterUnavailableReason {
    Missing,
    Malformed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FrontmatterFieldContent {
    Present {
        source: SourceRef,
        field: String,
        value: StructuredJson,
        /// The YAML decoder's reference span, mapped to the original document.
        span: SourceSpan,
    },
    Absent {
        source: SourceRef,
        field: String,
        frontmatter_span: SourceSpan,
    },
    BlockUnavailable {
        source: SourceRef,
        field: String,
        reason: FrontmatterUnavailableReason,
        diagnostics: DiagnosticList,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FrontmatterFieldLocation {
    Present {
        span: SourceSpan,
    },
    Absent {
        frontmatter_span: SourceSpan,
    },
    BlockUnavailable {
        reason: FrontmatterUnavailableReason,
    },
}
