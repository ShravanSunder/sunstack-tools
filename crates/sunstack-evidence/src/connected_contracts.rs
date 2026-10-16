use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroUsize, path::PathBuf};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Rust,
    Typescript,
    Tsx,
    Markdown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraphLanguage {
    Rust,
    Typescript,
    Tsx,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnitKindFilter {
    Function,
    Method,
    Class,
    Paragraph,
    Section,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceSides {
    AfterOnly,
    BeforeOnly,
    BothAvailable,
    PairedRequired,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RelationDirection {
    Callers,
    Callees,
    Imports,
    ImportedBy,
}
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SourceUnitKey {
    pub snapshot: SnapshotId,
    pub repository: RepositoryId,
    pub revision: SourceRevision,
    pub path: PathBuf,
    pub byte_start: usize,
    pub byte_end: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceUnitKind {
    Function { signature: String },
    Method { signature: String },
    Class { signature: String },
    Paragraph,
    Section,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxQuality {
    Parsed,
    RecoveredWithErrors,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceUnit {
    pub key: SourceUnitKey,
    pub kind: SourceUnitKind,
    pub span: SourceSpan,
    pub content_identity: String,
    pub syntax_quality: SyntaxQuality,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectedSourceUnit {
    pub unit: SourceUnit,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceUnitSeeds {
    ChangedUnits { repository_ids: Vec<RepositoryId> },
    SelectorUnits { selectors: Vec<EvidenceSelector> },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceUnitQuery {
    pub seeds: SourceUnitSeeds,
    pub languages: Vec<SourceLanguage>,
    pub unit_kinds: Vec<UnitKindFilter>,
    pub source_sides: SourceSides,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelationshipQuery {
    pub source_units: SourceUnitQuery,
    pub repository_ids: Vec<RepositoryId>,
    pub directions: Vec<RelationDirection>,
    pub requested_hops: NonZeroUsize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitContent {
    WholeUnit,
    BoundedExcerpt { max_bytes: NonZeroUsize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChunkGrouping {
    SingleGroup,
    PerSeed,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChunkSettings {
    pub grouping: ChunkGrouping,
    pub seed_content: UnitContent,
    pub related_content: UnitContent,
    pub max_chunk_bytes: NonZeroUsize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryBounds {
    pub repository_ids: Vec<RepositoryId>,
    pub requested_hops: usize,
    pub windows: Vec<ResolvedTimeWindow>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryLimit {
    Files,
    FileBytes,
    TotalBytes,
    Matches,
    UnitVisits,
    RelationshipCandidates,
    FanOut,
    HistoryCommits,
    LocalTime,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCutoff {
    pub limit: DiscoveryLimit,
    pub frontier: Vec<SourceUnitKey>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryCoverage {
    CompleteWithinDeclaredScope {
        query_bounds: QueryBounds,
    },
    Limited {
        query_bounds: QueryBounds,
        cutoffs: Vec<DiscoveryCutoff>,
    },
    Unavailable {
        diagnostics: DiagnosticList,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionCoverage {
    SingleSource {
        capture: CaptureCoverage,
    },
    ChangeSet {
        repository: RepositoryId,
        sources: Vec<SourceRef>,
        discovery: DiscoveryCoverage,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelationshipFact {
    Declaration {
        span: SourceSpan,
    },
    ImportBinding {
        span: SourceSpan,
    },
    StaticCall {
        span: SourceSpan,
    },
    PackageMapping {
        importing_repository: RepositoryId,
        target_repository: RepositoryId,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipUncertainty {
    SyntaxOnly,
    DynamicDispatch,
    AmbiguousDeclaration,
    UnresolvedPackage,
    UnsupportedImport,
    MissingSource,
    RecoveredSyntax,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelationshipCandidate {
    Candidate {
        source_unit: SourceUnitKey,
        target_unit: SourceUnitKey,
        call_site_span: SourceSpan,
        relation_kind: RelationDirection,
        basis_spans: Vec<SourceSpan>,
        facts: Vec<RelationshipFact>,
        uncertainties: Vec<RelationshipUncertainty>,
    },
    Contradicted {
        source_unit: SourceUnitKey,
        possible_target: SourceUnitKey,
        basis_spans: Vec<SourceSpan>,
        contradiction: RelationshipContradiction,
    },
    Unresolved {
        source_unit: SourceUnitKey,
        target_hint: String,
        relation_kind: RelationDirection,
        basis_spans: Vec<SourceSpan>,
        reason: RelationshipUncertainty,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipContradiction {
    LexicalShadow,
    AmbiguousBinding,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExcerptContext {
    Code {
        enclosing_signature: String,
        import_context: Vec<SourceSpan>,
    },
    Markdown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Excerpt {
    pub unit: SourceUnitKey,
    pub span: SourceSpan,
    pub text: String,
    pub membership: UnitContent,
    pub context: ExcerptContext,
    pub content_identity: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticChunk {
    pub key: String,
    pub units: Vec<SourceUnitKey>,
    pub relationships: Vec<RelationshipCandidate>,
    pub excerpts: Vec<Excerpt>,
    pub omitted_optional: Vec<Omission>,
    pub unknown_recency_context: Vec<SourceRef>,
    pub coverage: DiscoveryCoverage,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContextGroupOutcome {
    Complete {
        group: SemanticChunk,
        coverage: DiscoveryCoverage,
    },
    Partial {
        group: SemanticChunk,
        coverage: DiscoveryCoverage,
        omissions: Vec<Omission>,
        diagnostics: DiagnosticList,
    },
    Unavailable {
        diagnostics: DiagnosticList,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeedContextGroup {
    pub seed: SourceUnitKey,
    pub outcome: ContextGroupOutcome,
}
