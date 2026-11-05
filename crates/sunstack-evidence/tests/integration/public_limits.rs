#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/mod.rs"]
mod support;
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn public_required_groups_refuse_real_acquisition_and_graph_cutoffs() {
    let fixture = GitFixture::new();
    fixture.write("seed.ts", "function seed() { return 0; }");
    fixture.write(
        "helpers.ts",
        "export function first() {} export function second() {} export function third() {}",
    );
    fixture.stage("seed.ts");
    fixture.stage("helpers.ts");
    fixture.commit(1000);
    fixture.write("seed.ts", "import { first, second, third } from './helpers'; function seed() { first(); second(); third(); }");
    for expected in [
        DiscoveryLimit::Files,
        DiscoveryLimit::FileBytes,
        DiscoveryLimit::TotalBytes,
        DiscoveryLimit::UnitVisits,
        DiscoveryLimit::RelationshipCandidates,
        DiscoveryLimit::FanOut,
    ] {
        let mut selection = related(RelationDirection::Callees);
        if let EvidenceSelection::RelatedUnits { requested_hops, .. } = &mut selection {
            *requested_hops = 2.try_into().unwrap();
        }
        let mut input = plan(
            vec![fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            })],
            selection,
            EvidenceRequirement::Required,
        );
        let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
            unreachable!()
        };
        match expected {
            DiscoveryLimit::Files => limits.max_files = 1.try_into().unwrap(),
            DiscoveryLimit::FileBytes => limits.max_file_bytes = 1.try_into().unwrap(),
            DiscoveryLimit::TotalBytes => limits.max_total_bytes = 32.try_into().unwrap(),
            DiscoveryLimit::UnitVisits => limits.max_unit_visits = 1.try_into().unwrap(),
            DiscoveryLimit::RelationshipCandidates => {
                limits.max_relationship_candidates = 1.try_into().unwrap()
            }
            DiscoveryLimit::FanOut => limits.max_fan_out = 1.try_into().unwrap(),
            _ => unreachable!(),
        }
        let outcome = prepare_evidence(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Incomplete { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::DiscoveryLimit)),
            "required {expected:?} cutoff must refuse through prepare_evidence"
        );
    }
}
#[tokio::test]
async fn public_fixed_workload_reaches_the_declared_real_local_time_cutoff() {
    let fixture = GitFixture::new();
    for index in 0..500 {
        fixture.write(
            &format!("source-{index:04}.ts"),
            &format!("function selected_{index}() {{}}"),
        );
    }
    let mut input = plan(
        vec![fixture.binding(ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Untracked],
        })],
        related(RelationDirection::Callees),
        EvidenceRequirement::Required,
    );
    input.limits.max_files = 1000.try_into().unwrap();
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        unreachable!()
    };
    limits.max_files = 1000.try_into().unwrap();
    limits.max_local_time_ms = 1.try_into().unwrap();
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Incomplete {
        change_selections,
        diagnostics,
        ..
    } = outcome
    else {
        panic!("500-file/1ms real probe must be reported honestly: {outcome:?}");
    };
    assert!(
        diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == EvidenceCode::DiscoveryLimit)
    );
    assert!(
        matches!(&change_selections[0].coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::LocalTime)),
        "a different cutoff would not prove the declared time bound"
    );
}

#[tokio::test]
async fn public_native_queries_share_cumulative_visits_and_matches() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("source.ts"),
        "function seedOne() { shared(); } function seedTwo() { shared(); } function shared() {}",
    )
    .unwrap();
    let mut input = native_plan(root.path(), &["seedOne", "seedTwo"]);
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        unreachable!()
    };
    limits.max_unit_visits = 1.try_into().unwrap();
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Incomplete {
        selections,
        measurements,
        ..
    } = outcome
    else {
        panic!("second query must respect the consumed visit budget: {outcome:?}");
    };
    assert!(matches!(
        &selections[0].readiness,
        SelectionReadiness::Ready { .. }
    ));
    assert!(
        matches!(&selections[1].readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::DiscoveryLimit))
    );
    assert_eq!(measurements.files_read, 1);
    let mut input = native_plan(root.path(), &["seed"]);
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        unreachable!()
    };
    limits.max_matches = 1.try_into().unwrap();
    let EvidenceSelector::RelatedUnits { chunk_settings, .. } =
        &mut input.native_queries[0].selector
    else {
        unreachable!()
    };
    chunk_settings.grouping = ChunkGrouping::PerSeed;
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome, EvidencePlanOutcome::Prepared { selections, .. } if matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::RelatedGroups { groups }, coverage, .. } if groups.len() == 1 && matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { discovery: DiscoveryCoverage::Limited { cutoffs, .. }, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::Matches))))
    );
}
fn native_plan(root: &std::path::Path, needles: &[&str]) -> EvidencePlanInput {
    let binding = RepositoryBinding {
        key: RepositoryId::new("plain").unwrap(),
        root: root.into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full {},
    };
    let mut input = plan(
        vec![binding],
        related(RelationDirection::Callees),
        EvidenceRequirement::Required,
    );
    input.sources.clear();
    input.declarations.clear();
    input.native_queries = needles
        .iter()
        .enumerate()
        .map(|(index, needle)| NativeQuery {
            id: DeclarationId::new(format!("query_{index}")).unwrap(),
            selector: EvidenceSelector::RelatedUnits {
                query: RelationshipQuery {
                    source_units: SourceUnitQuery {
                        seeds: SourceUnitSeeds::SelectorUnits {
                            selectors: vec![EvidenceSelector::LiteralText {
                                repository_id: RepositoryId::new("plain").unwrap(),
                                paths: vec![QueryPath::Exact {
                                    path: "source.ts".into(),
                                }],
                                text: (*needle).into(),
                            }],
                        },
                        languages: vec![SourceLanguage::Typescript],
                        unit_kinds: vec![UnitKindFilter::Function],
                        source_sides: SourceSides::AfterOnly,
                    },
                    repository_ids: vec![RepositoryId::new("plain").unwrap()],
                    directions: vec![RelationDirection::Callees],
                    requested_hops: 1.try_into().unwrap(),
                },
                chunk_settings: ChunkSettings {
                    grouping: ChunkGrouping::SingleGroup,
                    seed_content: UnitContent::WholeUnit {},
                    related_content: UnitContent::WholeUnit {},
                    max_chunk_bytes: 10000.try_into().unwrap(),
                },
                minimum_related_units: 1,
            },
        })
        .collect();
    input
}

#[tokio::test]
async fn public_history_budget_is_shared_across_repository_selections() {
    let first = GitFixture::new();
    let second = GitFixture::new();
    for fixture in [&first, &second] {
        fixture.write("source.ts", "function selected() {}");
        fixture.stage("source.ts");
        fixture.commit(1000);
    }
    let scope = ChangeScope::RecentCommits {
        reference: "HEAD".into(),
        window: TimeWindow::Absolute {
            since_utc: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            until_utc: chrono::DateTime::from_timestamp(2000, 0).unwrap(),
        },
        parent_policy: CommitParentPolicy::FirstParent,
    };
    let mut other = second.binding(scope.clone());
    other.key = RepositoryId::new("other").unwrap();
    let mut input = plan(
        vec![first.binding(scope), other.clone()],
        EvidenceSelection::ChangeSetLiteralText {
            paths: vec![QueryPath::Glob {
                pattern: "*.ts".into(),
            }],
            text: "selected".into(),
        },
        EvidenceRequirement::Required,
    );
    input.sources.insert(
        SourceName::new("other_changes").unwrap(),
        SourceSpec::ChangeSet {
            repository: other.key,
        },
    );
    let mut second_declaration = input.declarations[0].clone();
    second_declaration.id = DeclarationId::new("other_context").unwrap();
    second_declaration.input = SourceName::new("other_changes").unwrap();
    input.declarations.push(second_declaration);
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        unreachable!()
    };
    limits.max_history_commits = 1.try_into().unwrap();
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        change_selections,
        ..
    } = outcome
    else {
        panic!("simple evidence can expose limited coverage: {outcome:?}");
    };
    assert!(
        matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::Matches { matches }, .. } if matches.len() == 1)
    );
    assert!(
        matches!(&selections[1].readiness, SelectionReadiness::Ready { content: SelectionContent::Matches { matches }, .. } if matches.is_empty())
    );
    let second_report = change_selections
        .iter()
        .find(|report| report.repository.as_str() == "other")
        .unwrap();
    assert!(
        matches!(&second_report.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::HistoryCommits))
    );
    assert!(
        second_report.members[0]
            .diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == EvidenceCode::HistoryIncomplete)
    );
}
