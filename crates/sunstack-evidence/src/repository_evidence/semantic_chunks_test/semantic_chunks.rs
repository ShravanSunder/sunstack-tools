#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;
use std::collections::BTreeMap;

#[test]
fn semantic_group_keeps_frozen_text_and_refuses_whole_unit_overflow() {
    let unit = selected("function giant() { return 'a long string'; }");
    let discovery = discovery(unit.clone());
    let settings = ChunkSettings {
        grouping: ChunkGrouping::SingleGroup,
        seed_content: UnitContent::WholeUnit {},
        related_content: UnitContent::WholeUnit {},
        max_chunk_bytes: 4096.try_into().unwrap(),
    };
    let groups = build_semantic_groups(&discovery, &settings, 0).unwrap();
    let [ContextGroupOutcome::Complete { group, .. }] = groups.as_slice() else {
        panic!("expected complete frozen group: {groups:?}");
    };
    assert_eq!(group.excerpts.len(), 1);
    assert_eq!(group.excerpts[0].text, unit.text);
    assert_eq!(group.excerpts[0].span, unit.unit.span);
    let tiny = ChunkSettings {
        max_chunk_bytes: 8.try_into().unwrap(),
        ..settings.clone()
    };
    let groups = build_semantic_groups(&discovery, &tiny, 0).unwrap();
    assert!(
        matches!(&groups[0], ContextGroupOutcome::Partial { group, omissions, .. } if group.excerpts.is_empty() && !omissions.is_empty())
    );
    let bounded = ChunkSettings {
        seed_content: UnitContent::BoundedExcerpt {
            max_bytes: 8.try_into().unwrap(),
        },
        ..tiny
    };
    let groups = build_semantic_groups(&discovery, &bounded, 0).unwrap();
    let ContextGroupOutcome::Complete { group, .. } = &groups[0] else {
        panic!("explicit bounded excerpt must fit");
    };
    assert_eq!(group.excerpts[0].text, "function");
    assert_eq!(group.excerpts[0].span.byte_end, 8);
    assert!(matches!(
        group.excerpts[0].membership,
        UnitContent::BoundedExcerpt { .. }
    ));
}

#[test]
fn group_readiness_preserves_cutoff_unresolved_and_minimum_failures() {
    let unit = selected("function seed() {}");
    let mut discovery = discovery(unit.clone());
    let settings = ChunkSettings {
        grouping: ChunkGrouping::SingleGroup,
        seed_content: UnitContent::WholeUnit {},
        related_content: UnitContent::WholeUnit {},
        max_chunk_bytes: 4096.try_into().unwrap(),
    };
    assert!(
        matches!(&build_semantic_groups(&discovery, &settings, 1).unwrap()[0], ContextGroupOutcome::Partial { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::MissingRelatedContext))
    );
    discovery
        .relationships
        .push(RelationshipCandidate::Unresolved {
            source_unit: unit.unit.key.clone(),
            target_hint: "dynamic".into(),
            relation_kind: RelationDirection::Callees,
            basis_spans: vec![unit.unit.span.clone()],
            reason: RelationshipUncertainty::DynamicDispatch,
        });
    assert!(
        matches!(&build_semantic_groups(&discovery, &settings, 0).unwrap()[0], ContextGroupOutcome::Partial { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::RelationshipUnresolved))
    );
    discovery.relationships.clear();
    discovery.coverage = DiscoveryCoverage::Limited {
        query_bounds: bounds(),
        cutoffs: vec![DiscoveryCutoff {
            limit: DiscoveryLimit::UnitVisits,
            frontier: vec![unit.unit.key],
        }],
    };
    assert!(matches!(
        &build_semantic_groups(&discovery, &settings, 0).unwrap()[0],
        ContextGroupOutcome::Partial {
            coverage: DiscoveryCoverage::Limited { .. },
            ..
        }
    ));
}
fn bounds() -> QueryBounds {
    QueryBounds {
        repository_ids: vec![RepositoryId::new("fixture").unwrap()],
        requested_hops: 1,
        windows: vec![],
    }
}
fn discovery(unit: SelectedSourceUnit) -> RelationshipDiscovery {
    RelationshipDiscovery {
        seeds: vec![unit.clone()],
        units: vec![unit],
        relationships: vec![],
        coverage: DiscoveryCoverage::CompleteWithinDeclaredScope {
            query_bounds: bounds(),
        },
        import_context: BTreeMap::new(),
        unknown_recency_context: vec![],
        unit_visits: 0,
        relationship_candidates: 0,
        files_parsed: 0,
        bytes_scanned: 0,
    }
}
fn selected(text: &str) -> SelectedSourceUnit {
    let snapshot = SnapshotId::new("fixture").unwrap();
    let source = SourceRef::RepositoryFile {
        repository: RepositoryId::new("fixture").unwrap(),
        revision: SourceRevision::Worktree {
            snapshot: snapshot.clone(),
            side: SourceSide::After,
        },
        path: "source.ts".into(),
        content_identity: content_digest(text.as_bytes()),
    };
    extract_source_units(
        &FrozenRepositoryFile {
            source,
            text: text.into(),
            side: SourceSide::After,
        },
        &snapshot,
    )
    .unwrap()
    .remove(0)
}
