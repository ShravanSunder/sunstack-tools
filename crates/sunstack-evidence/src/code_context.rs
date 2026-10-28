use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

validated_name!(PieceId, |value: &str| !value.is_empty());
validated_name!(CodeContextId, |value: &str| !value.is_empty());
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextPart {
    Change,
    TouchedUnit,
    Related,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CodePatternLanguage {
    Rust,
    Typescript,
    Tsx,
}
impl CodePatternLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Typescript => "typescript",
            Self::Tsx => "tsx",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RelatedPiece {
    Resolved {
        piece_id: PieceId,
        seed: SourceUnitKey,
        connection: RelationDirection,
        unit: SourceUnitKey,
        facts: Vec<RelationshipFact>,
        uncertainties: Vec<RelationshipUncertainty>,
        contradictions: Vec<RelationshipContradiction>,
    },
    Unresolved {
        piece_id: PieceId,
        seed: SourceUnitKey,
        connection: RelationDirection,
        name_as_written: String,
        call_site: SourceSpan,
        reason: RelationshipUncertainty,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextUnit {
    pub piece_id: PieceId,
    pub unit: SourceUnit,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeRedaction {
    pub span: SourceSpan,
    pub private_kind: PrivateKind,
    pub count: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateContext {
    pub context_id: CodeContextId,
    pub input_id: SourceInputId,
    pub repository: RepositoryId,
    pub matched: CodeExcerpt,
    pub changes: Vec<ChangeHunk>,
    pub units: Vec<ContextUnit>,
    pub redactions: Vec<CodeRedaction>,
    pub related: Vec<RelatedPiece>,
    pub imports: Vec<CodeExcerpt>,
    pub coverage: DiscoveryCoverage,
}
#[derive(Clone, Debug)]
pub struct CodeContextIndex {
    pub input: SourceInput,
    pub input_id: SourceInputId,
    pub repository: RepositoryId,
    pub changes: Vec<ChangeHunk>,
    pub units: Vec<SelectedSourceUnit>,
    pub relationships: Vec<RelationshipCandidate>,
    pub coverage: DiscoveryCoverage,
    pub(crate) capture: Arc<CapturedChangeSelection>,
    pub(crate) limits: DiscoveryLimits,
    pub(crate) imports: BTreeMap<SourceUnitKey, Vec<SourceSpan>>,
    pub(crate) cancellation: tokio_util::sync::CancellationToken,
}
#[derive(Clone, Debug)]
pub struct DiscoveredCodeMatch {
    pub occurrence: SourceOccurrence,
    pub context: CandidateContext,
}

pub(crate) fn build_index(
    input: SourceInput,
    capture: CapturedChangeSelection,
    limits: DiscoveryLimits,
    cancellation: tokio_util::sync::CancellationToken,
) -> EvidenceResult<CodeContextIndex> {
    let input_id = input.source_identity()?;
    let changes = change_hunks(&input_id, &capture)?;
    let mut units = Vec::new();
    for file in &capture.sources {
        if cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        units.extend(extract_source_units(file, &capture.snapshot)?);
    }
    let mut index = CodeContextIndex {
        input,
        input_id,
        repository: capture.repository.clone(),
        changes,
        units,
        relationships: vec![],
        coverage: capture.coverage.clone(),
        capture: Arc::new(capture),
        limits: limits.clone(),
        imports: BTreeMap::new(),
        cancellation: cancellation.clone(),
    };
    if !index.units.is_empty() {
        let query = RelationshipQuery {
            source_units: SourceUnitQuery {
                seeds: SourceUnitSeeds::ChangedUnits {
                    repository_ids: vec![index.repository.clone()],
                },
                languages: vec![
                    SourceLanguage::Rust,
                    SourceLanguage::Typescript,
                    SourceLanguage::Tsx,
                ],
                unit_kinds: vec![
                    UnitKindFilter::Function,
                    UnitKindFilter::Method,
                    UnitKindFilter::Class,
                ],
                source_sides: SourceSides::BothAvailable,
            },
            repository_ids: vec![index.repository.clone()],
            directions: vec![
                RelationDirection::Callers,
                RelationDirection::Callees,
                RelationDirection::Imports,
                RelationDirection::ImportedBy,
            ],
            requested_hops: 1.try_into().map_err(|_| EvidenceCode::InvalidSelector)?,
        };
        // Every captured unit is indexed once; candidate contexts choose incident facts afterward.
        let discovery = discover_relationships(RelationshipDiscoveryRequest {
            captures: std::slice::from_ref(index.capture.as_ref()),
            seeds: &index.units,
            query: &query,
            package_bindings: &[],
            limits: &limits,
            cancellation,
        })?;
        index.relationships = discovery.relationships;
        index.imports = discovery.import_context;
        index.coverage = discovery.coverage;
    }
    Ok(index)
}
fn change_hunks(
    input: &SourceInputId,
    capture: &CapturedChangeSelection,
) -> EvidenceResult<Vec<ChangeHunk>> {
    let mut hunks = Vec::new();
    for (member_position, member) in capture.members.iter().enumerate() {
        for change in &member.changes {
            for (position, (before, after)) in change
                .before_spans
                .iter()
                .zip(&change.after_spans)
                .enumerate()
            {
                let before = side_excerpt(&change.before, before)?;
                let after = side_excerpt(&change.after, after)?;
                let identities = [before.as_ref(), after.as_ref()]
                    .into_iter()
                    .map(|excerpt| {
                        excerpt
                            .map(|excerpt| {
                                crate::code_occurrence_identity::code_occurrence_identity(
                                    input,
                                    None,
                                    &excerpt.span,
                                )
                            })
                            .transpose()
                    })
                    .collect::<EvidenceResult<Vec<_>>>()?;
                let bytes =
                    serde_json::to_vec(&(member_position, &change.path, position, identities))
                        .map_err(|_| EvidenceCode::InvalidSelector)?;
                hunks.push(ChangeHunk {
                    hunk_id: HunkId::new(format!("{}:{}", input.as_str(), content_digest(&bytes)))?,
                    path: change.path.clone(),
                    before,
                    after,
                });
            }
        }
    }
    Ok(hunks)
}
fn side_excerpt(
    side: &CapturedFileSide,
    range: &ChangeSpan,
) -> EvidenceResult<Option<CodeExcerpt>> {
    match side {
        CapturedFileSide::Absent => Ok(None),
        CapturedFileSide::Present { file } => Ok(Some(CodeExcerpt {
            span: source_span(&file.source, &file.text, range.byte_start..range.byte_end),
            text: file
                .text
                .get(range.byte_start..range.byte_end)
                .ok_or(EvidenceCode::InvalidSelector)?
                .into(),
        })),
    }
}
