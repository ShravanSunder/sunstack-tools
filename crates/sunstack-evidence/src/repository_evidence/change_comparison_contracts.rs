use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum ChangeComparisonSubject {
    Declaration { declaration: EvidenceDeclaration },
    Input { input: SourceName },
    DefinitionFile { path: PathBuf },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonLevel {
    Path,
    Content,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BeforeUnknownReason {
    BaselineUnavailable,
    HistoryIncomplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NoBeforeSideReason {
    FullScope,
    ConversationInput,
    PayloadInput,
    OutsideChangeScopePaths,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeComparison {
    Unchanged {},
    Changed { members: Vec<usize> },
    PathChanged { members: Vec<usize> },
    Added { members: Vec<usize> },
    Deleted { members: Vec<usize> },
    BeforeUnknown { reason: BeforeUnknownReason },
    NoBeforeSide { reason: NoBeforeSideReason },
    Refused { diagnostics: DiagnosticList },
}

#[cfg(test)]
#[path = "change_comparison_contracts_test.rs"]
mod tests;
