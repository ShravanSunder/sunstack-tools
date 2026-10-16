#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn public_native_per_seed_groups_attribute_shared_related_units_without_recapture() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("source.ts"),
        "function seedOne() { shared(); } function seedTwo() { shared(); } function shared() {}",
    )
    .unwrap();
    let binding = RepositoryBinding {
        key: RepositoryId::new("plain").unwrap(),
        root: directory.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let mut input = plan(
        vec![binding],
        related(RelationDirection::Callees),
        EvidenceRequirement::Required,
    );
    input.sources.clear();
    input.declarations.clear();
    let query = RelationshipQuery {
        source_units: SourceUnitQuery {
            seeds: SourceUnitSeeds::SelectorUnits {
                selectors: vec![EvidenceSelector::LiteralText {
                    repository_id: RepositoryId::new("plain").unwrap(),
                    paths: vec![QueryPath::Exact {
                        path: "source.ts".into(),
                    }],
                    text: "seed".into(),
                }],
            },
            languages: vec![SourceLanguage::Typescript],
            unit_kinds: vec![UnitKindFilter::Function],
            source_sides: SourceSides::AfterOnly,
        },
        repository_ids: vec![RepositoryId::new("plain").unwrap()],
        directions: vec![RelationDirection::Callees],
        requested_hops: 1.try_into().unwrap(),
    };
    input.native_queries.push(NativeQuery {
        id: DeclarationId::new("native_groups").unwrap(),
        selector: EvidenceSelector::RelatedUnits {
            query,
            chunk_settings: ChunkSettings {
                grouping: ChunkGrouping::PerSeed,
                seed_content: UnitContent::WholeUnit,
                related_content: UnitContent::WholeUnit,
                max_chunk_bytes: 10000.try_into().unwrap(),
            },
            minimum_related_units: 1,
        },
    });
    let mut limited_input = input.clone();
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        measurements,
        ..
    } = outcome
    else {
        panic!("expected native groups: {outcome:?}");
    };
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroups { groups },
        ..
    } = &selections[0].readiness
    else {
        panic!("native per_seed must use related_groups");
    };
    assert_eq!(groups.len(), 2);
    assert!(groups[0].seed < groups[1].seed);
    for entry in groups {
        let ContextGroupOutcome::Complete { group, .. } = &entry.outcome else {
            panic!("self-contained group incomplete");
        };
        assert_eq!(group.excerpts.len(), 2);
        assert!(
            group
                .excerpts
                .iter()
                .any(|excerpt| excerpt.unit == entry.seed)
        );
        assert!(
            group
                .excerpts
                .iter()
                .any(|excerpt| excerpt.text == "function shared() {}")
        );
    }
    assert_eq!(measurements.files_read, 1);
    if let EvidenceDiscovery::Bounded { limits } = &mut limited_input.discovery {
        limits.max_unit_visits = 1.try_into().unwrap();
    }
    let outcome = prepare_evidence(
        limited_input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        measurements,
        captures,
        ..
    } = outcome
    else {
        panic!("native callers compose per-seed outcomes");
    };
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroups { groups },
        ..
    } = &selections[0].readiness
    else {
        panic!("seed-attributed partial groups missing");
    };
    assert!(
        matches!(&groups[0].outcome, ContextGroupOutcome::Complete { .. }),
        "a cutoff at the other seed must not invalidate this completed neighborhood"
    );
    assert!(matches!(
        &groups[1].outcome,
        ContextGroupOutcome::Partial { .. }
    ));
    assert_eq!(measurements.files_read, 1);
    assert_eq!(captures.len(), 1);
}

#[tokio::test]
async fn public_required_failures_preserve_their_cause_and_optional_groups_keep_partial_evidence() {
    let fixture = GitFixture::new();
    fixture.write("seed.ts", "function seed() { return 0; }");
    fixture.write("helper.ts", "export function helper() { return 1; }");
    fixture.stage("seed.ts");
    fixture.stage("helper.ts");
    fixture.commit(1000);
    for (case, expected) in [
        ("unresolved", EvidenceCode::RelationshipUnresolved),
        ("minimum", EvidenceCode::MissingRelatedContext),
        ("overflow", EvidenceCode::RequiredGroupIncomplete),
        ("visits", EvidenceCode::DiscoveryLimit),
    ] {
        fixture.write("seed.ts", if case == "unresolved" { "import { helper } from './helper'; function seed() { helper(); receiver.dynamic(); }" } else { "import { helper } from './helper'; function seed() { helper(); }" });
        let mut select = related(RelationDirection::Callees);
        if let EvidenceSelection::RelatedUnits {
            minimum_related_units,
            chunk_settings,
            requested_hops,
            ..
        } = &mut select
        {
            if case == "minimum" {
                *minimum_related_units = 100;
            }
            if case == "overflow" {
                chunk_settings.max_chunk_bytes = 1.try_into().unwrap();
            }
            if case == "visits" {
                *requested_hops = 2.try_into().unwrap();
            }
        }
        let mut input = plan(
            vec![fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            })],
            select,
            EvidenceRequirement::Required,
        );
        if case == "visits"
            && let EvidenceDiscovery::Bounded { limits } = &mut input.discovery
        {
            limits.max_unit_visits = 1.try_into().unwrap();
        }
        let outcome = prepare_evidence(
            input.clone(),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Incomplete { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == expected)),
            "required {case} must preserve its actual cause"
        );
        input.declarations[0].requirement = EvidenceRequirement::Optional;
        let outcome = prepare_evidence(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Prepared { selections, .. } if matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::RelatedGroup { group }, .. } if matches!(group.as_ref(), ContextGroupOutcome::Partial { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == expected)))),
            "optional {case} must retain a labelled partial group"
        );
    }
}
#[tokio::test]
async fn public_unknown_recency_context_is_labelled_without_becoming_a_changed_seed() {
    let fixture = GitFixture::new();
    fixture.write("seed.ts", "function seed() { return 0; }");
    fixture.write("helper.ts", "export function helper() { return 0; }");
    fixture.stage("seed.ts");
    fixture.stage("helper.ts");
    fixture.commit(1000);
    fixture.write(
        "seed.ts",
        "import { helper } from './helper'; function seed() { helper(); }",
    );
    fixture.write("helper.ts", "export function helper() { return 2; }");
    std::fs::File::options()
        .write(true)
        .open(fixture.directory.path().join("helper.ts"))
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(3600)),
        )
        .unwrap();
    let scope = ChangeScope::RecentUncommitted {
        layers: vec![DirtyLayer::Unstaged],
        window: TimeWindow::Lookback {
            seconds: 60.try_into().unwrap(),
        },
        unknown_recency: UnknownRecencyPolicy::ExcludeWithOmission,
    };
    let outcome = prepare_evidence(
        plan(
            vec![fixture.binding(scope)],
            related(RelationDirection::Callees),
            EvidenceRequirement::Required,
        ),
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
        panic!("unknown-time context should remain usable: {outcome:?}");
    };
    assert_eq!(change_selections[0].members[0].changed_paths.len(), 1);
    assert_eq!(
        change_selections[0].members[0].changed_paths[0]
            .path
            .to_str(),
        Some("seed.ts")
    );
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroup { group },
        ..
    } = &selections[0].readiness
    else {
        panic!("related group missing");
    };
    let ContextGroupOutcome::Complete { group, .. } = group.as_ref() else {
        panic!("group incomplete");
    };
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function helper() { return 2; }")
    );
    assert_eq!(group.unknown_recency_context.len(), 1);
    assert!(
        matches!(&group.unknown_recency_context[0], SourceRef::RepositoryFile { path, revision: SourceRevision::Worktree { .. }, .. } if path.to_str() == Some("helper.ts"))
    );
}
