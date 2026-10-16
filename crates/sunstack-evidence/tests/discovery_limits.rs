#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn real_graph_limits_preserve_frontier_and_semantic_partial_group() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function root() { first(); second(); third(); } function first() {} function second() {} function third() {}");
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Untracked],
            }),
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let seeds = changed_source_units(
        &captured,
        &[SourceLanguage::Typescript],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap()
    .into_iter()
    .filter(|unit| unit.text.starts_with("function root"))
    .collect::<Vec<_>>();
    let captures = [captured];
    let query = RelationshipQuery {
        source_units: SourceUnitQuery {
            seeds: SourceUnitSeeds::ChangedUnits {
                repository_ids: vec![RepositoryId::new("fixture").unwrap()],
            },
            languages: vec![SourceLanguage::Typescript],
            unit_kinds: vec![UnitKindFilter::Function],
            source_sides: SourceSides::AfterOnly,
        },
        repository_ids: vec![RepositoryId::new("fixture").unwrap()],
        directions: vec![RelationDirection::Callees],
        requested_hops: 3.try_into().unwrap(),
    };
    for expected in [
        DiscoveryLimit::UnitVisits,
        DiscoveryLimit::RelationshipCandidates,
        DiscoveryLimit::FanOut,
    ] {
        let mut limits = discovery_limits();
        match expected {
            DiscoveryLimit::UnitVisits => limits.max_unit_visits = 1.try_into().unwrap(),
            DiscoveryLimit::RelationshipCandidates => {
                limits.max_relationship_candidates = 1.try_into().unwrap()
            }
            DiscoveryLimit::FanOut => limits.max_fan_out = 1.try_into().unwrap(),
            _ => unreachable!(),
        }
        let discovery = discover_relationships(RelationshipDiscoveryRequest {
            captures: &captures,
            seeds: &seeds,
            query: &query,
            package_bindings: &[],
            limits: &limits,
            cancellation: CancellationToken::new(),
        })
        .unwrap();
        assert!(
            matches!(&discovery.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == expected && !cutoff.frontier.is_empty()))
        );
        let settings = ChunkSettings {
            grouping: ChunkGrouping::SingleGroup,
            seed_content: UnitContent::WholeUnit,
            related_content: UnitContent::WholeUnit,
            max_chunk_bytes: 10000.try_into().unwrap(),
        };
        assert!(matches!(
            &build_semantic_groups(&discovery, &settings, 0).unwrap()[0],
            ContextGroupOutcome::Partial { .. }
        ));
    }
}

#[tokio::test]
async fn real_file_bytes_total_bytes_and_time_have_distinct_cutoffs() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..500 {
        std::fs::write(
            directory.path().join(format!("source-{index:04}.ts")),
            "function target() { return '1234567890'; }",
        )
        .unwrap();
    }
    let binding = RepositoryBinding {
        key: RepositoryId::new("plain").unwrap(),
        root: directory.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    for expected in [
        DiscoveryLimit::FileBytes,
        DiscoveryLimit::TotalBytes,
        DiscoveryLimit::LocalTime,
    ] {
        let mut limits = discovery_limits();
        limits.max_files = 1000.try_into().unwrap();
        match expected {
            DiscoveryLimit::FileBytes => limits.max_file_bytes = 1.try_into().unwrap(),
            DiscoveryLimit::TotalBytes => limits.max_total_bytes = 50.try_into().unwrap(),
            DiscoveryLimit::LocalTime => limits.max_local_time_ms = 1.try_into().unwrap(),
            _ => unreachable!(),
        }
        let captured = capture_repository_context(
            ChangeCaptureRequest {
                binding: binding.clone(),
                limits,
                anchor_utc: chrono::Utc::now(),
            },
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(
            matches!(&captured.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == expected)),
            "fixed 500-file workload, declared budget, observed {expected:?}: {:?}",
            captured.coverage
        );
    }
}
