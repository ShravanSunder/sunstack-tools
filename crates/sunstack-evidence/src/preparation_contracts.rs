use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PayloadSchemaPath {
    pub name: SchemaName,
    pub path: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeQuery {
    pub id: DeclarationId,
    pub selector: EvidenceSelector,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceSelector {
    Paragraph {
        repository_id: RepositoryId,
        source_path: PathBuf,
        target_id: TargetId,
    },
    FileText {
        repository_id: RepositoryId,
        source_path: PathBuf,
    },
    LiteralText {
        repository_id: RepositoryId,
        paths: Vec<QueryPath>,
        text: String,
    },
    RegexText {
        repository_id: RepositoryId,
        paths: Vec<QueryPath>,
        pattern: String,
    },
    AstPattern {
        repository_id: RepositoryId,
        paths: Vec<QueryPath>,
        language: String,
        pattern: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageSourceBinding {
    pub importing_repository_id: RepositoryId,
    pub module_prefix: String,
    pub target_repository_id: RepositoryId,
    pub target_path_prefix: PathBuf,
    pub language: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLimits {
    pub max_files: NonZeroUsize,
    pub max_file_bytes: NonZeroUsize,
    pub max_total_bytes: NonZeroUsize,
    pub max_matches: NonZeroUsize,
    pub include_ignored: bool,
    pub max_unit_visits: NonZeroUsize,
    pub max_relationship_candidates: NonZeroUsize,
    pub max_fan_out: NonZeroUsize,
    pub max_history_commits: NonZeroUsize,
    pub max_local_time_ms: NonZeroUsize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceDiscovery {
    None,
    Bounded { limits: DiscoveryLimits },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidencePlanInput {
    pub repositories: Vec<RepositoryBinding>,
    pub sources: BTreeMap<SourceName, SourceSpec>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub cursors: Vec<ReturnedCursor>,
    pub declarations: Vec<EvidenceDeclaration>,
    pub native_queries: Vec<NativeQuery>,
    pub package_bindings: Vec<PackageSourceBinding>,
    pub limits: InvocationLimits,
    pub discovery: EvidenceDiscovery,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMeasurements {
    pub files_considered: usize,
    pub files_read: usize,
    pub files_parsed: usize,
    pub transcript_bytes_read: u64,
    pub transcript_bytes_parsed: u64,
    pub bytes_scanned: usize,
    pub bytes_included: usize,
    pub local_elapsed_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartialEvidencePlan {
    pub selections: Vec<SelectionResult>,
    pub captures: Vec<CaptureCoverage>,
    pub cursors: Vec<ReturnedCursor>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceCancellationProgress {
    BeforePlanAdmission,
    PlanAdmitted { partial: PartialEvidencePlan },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidencePlanOutcome {
    Prepared {
        selections: Vec<SelectionResult>,
        captures: Vec<CaptureCoverage>,
        cursors: Vec<ReturnedCursor>,
        measurements: EvidenceMeasurements,
    },
    Incomplete {
        selections: Vec<SelectionResult>,
        captures: Vec<CaptureCoverage>,
        cursors: Vec<ReturnedCursor>,
        measurements: EvidenceMeasurements,
        diagnostics: DiagnosticList,
    },
    Refused {
        diagnostics: DiagnosticList,
    },
    Cancelled {
        progress: EvidenceCancellationProgress,
        diagnostics: DiagnosticList,
        measurements: EvidenceMeasurements,
    },
}
