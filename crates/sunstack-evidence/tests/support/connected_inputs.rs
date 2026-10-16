use super::discovery_limits;
use std::collections::BTreeMap;
use sunstack_evidence::*;

pub fn related(direction: RelationDirection) -> EvidenceSelection {
    EvidenceSelection::RelatedUnits {
        languages: vec![GraphLanguage::Typescript],
        unit_kinds: vec![UnitKindFilter::Function],
        source_sides: SourceSides::AfterOnly,
        directions: vec![direction],
        requested_hops: 1.try_into().unwrap(),
        chunk_settings: ChunkSettings {
            grouping: ChunkGrouping::SingleGroup,
            seed_content: UnitContent::WholeUnit,
            related_content: UnitContent::WholeUnit,
            max_chunk_bytes: 10000.try_into().unwrap(),
        },
        minimum_related_units: 1,
    }
}

pub fn plan(
    repositories: Vec<RepositoryBinding>,
    select: EvidenceSelection,
    requirement: EvidenceRequirement,
) -> EvidencePlanInput {
    let repository = repositories[0].key.clone();
    EvidencePlanInput {
        repositories,
        sources: BTreeMap::from([(
            SourceName::new("changes").unwrap(),
            SourceSpec::ChangeSet { repository },
        )]),
        payload_schemas: vec![],
        cursors: vec![],
        declarations: vec![EvidenceDeclaration {
            id: DeclarationId::new("context").unwrap(),
            input: SourceName::new("changes").unwrap(),
            requirement,
            select,
            max_items: None,
            max_bytes: None,
        }],
        native_queries: vec![],
        package_bindings: vec![],
        limits: InvocationLimits {
            max_files: 100.try_into().unwrap(),
            max_file_bytes: 1000000.try_into().unwrap(),
            max_total_bytes: 10000000.try_into().unwrap(),
            max_matches: 1000.try_into().unwrap(),
            max_transcript_bytes: 100000.try_into().unwrap(),
            include_ignored: false,
        },
        discovery: EvidenceDiscovery::Bounded {
            limits: discovery_limits(),
        },
    }
}
