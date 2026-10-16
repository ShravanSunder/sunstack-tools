#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[test]
fn invalid_windows_and_empty_commit_head_refuse_without_opening_sources() {
    let later = chrono::DateTime::from_timestamp(2000, 0).unwrap();
    let earlier = chrono::DateTime::from_timestamp(1000, 0).unwrap();
    for scope in [
        ChangeScope::RecentCommits {
            reference: "HEAD".into(),
            window: TimeWindow::Absolute {
                since_utc: later,
                until_utc: earlier,
            },
            parent_policy: CommitParentPolicy::FirstParent,
        },
        ChangeScope::RecentUncommitted {
            layers: vec![DirtyLayer::Unstaged],
            window: TimeWindow::Absolute {
                since_utc: later,
                until_utc: earlier,
            },
            unknown_recency: UnknownRecencyPolicy::ExcludeWithOmission,
        },
        ChangeScope::RevisionDiff {
            base_revision: "HEAD".into(),
            head: RevisionHead::Commit {
                revision: String::new(),
            },
        },
    ] {
        assert_eq!(
            scope.validate().unwrap_err().code,
            EvidenceCode::InvalidRepositoryBinding
        );
    }
}

#[tokio::test]
async fn duplicate_connected_query_fields_are_rejected_at_admission() {
    let fixture = GitFixture::new();
    for field in ["languages", "unit_kinds", "directions"] {
        let mut selection = related(RelationDirection::Callees);
        if let EvidenceSelection::RelatedUnits {
            languages,
            unit_kinds,
            directions,
            ..
        } = &mut selection
        {
            match field {
                "languages" => languages.push(languages[0]),
                "unit_kinds" => unit_kinds.push(unit_kinds[0]),
                _ => directions.push(directions[0]),
            }
        }
        let input = plan(
            vec![fixture.binding(ChangeScope::Full)],
            selection,
            EvidenceRequirement::Required,
        );
        let outcome = prepare_evidence(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Refused { .. }),
            "duplicate {field} must refuse before acquisition: {outcome:?}"
        );
    }
}

#[tokio::test]
async fn native_connected_query_unique_fields_are_validated_before_capture() {
    let fixture = GitFixture::new();
    let base = serde_json::json!({
        "source_units": {"seeds": {"kind": "changed_units", "repository_ids": ["fixture"]}, "languages": ["typescript"], "unit_kinds": ["function"], "source_sides": "after_only"},
        "repository_ids": ["fixture"], "directions": ["callees"], "requested_hops": 1
    });
    for pointer in [
        "/source_units/seeds/repository_ids",
        "/source_units/languages",
        "/source_units/unit_kinds",
        "/repository_ids",
        "/directions",
    ] {
        let mut value = base.clone();
        let values = value.pointer_mut(pointer).unwrap().as_array_mut().unwrap();
        values.push(values[0].clone());
        let mut input = plan(
            vec![fixture.binding(ChangeScope::Full)],
            related(RelationDirection::Callees),
            EvidenceRequirement::Required,
        );
        input.declarations.clear();
        input.native_queries.push(NativeQuery {
            id: DeclarationId::new("native").unwrap(),
            selector: EvidenceSelector::RelatedUnits {
                query: serde_json::from_value(value).unwrap(),
                chunk_settings: ChunkSettings {
                    grouping: ChunkGrouping::PerSeed,
                    seed_content: UnitContent::WholeUnit,
                    related_content: UnitContent::WholeUnit,
                    max_chunk_bytes: 1000.try_into().unwrap(),
                },
                minimum_related_units: 0,
            },
        });
        let outcome = prepare_evidence(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Refused { .. }),
            "duplicate native {pointer} reached capture: {outcome:?}"
        );
    }
}

#[tokio::test]
async fn disabled_discovery_has_its_own_closed_admission_code() {
    let fixture = GitFixture::new();
    let mut input = plan(
        vec![fixture.binding(ChangeScope::Full)],
        related(RelationDirection::Callees),
        EvidenceRequirement::Required,
    );
    input.discovery = EvidenceDiscovery::None;
    let mut native = input.clone();
    native.sources.clear();
    native.declarations.clear();
    native.native_queries.push(NativeQuery {
        id: DeclarationId::new("native").unwrap(),
        selector: EvidenceSelector::RelatedUnits {
            query: serde_json::from_value(serde_json::json!({
                "source_units": {"seeds": {"kind": "changed_units", "repository_ids": ["fixture"]}, "languages": ["typescript"], "unit_kinds": ["function"], "source_sides": "after_only"},
                "repository_ids": ["fixture"], "directions": ["callees"], "requested_hops": 1
            })).unwrap(),
            chunk_settings: ChunkSettings { grouping: ChunkGrouping::SingleGroup, seed_content: UnitContent::WholeUnit, related_content: UnitContent::WholeUnit, max_chunk_bytes: 1000.try_into().unwrap() },
            minimum_related_units: 0,
        },
    });
    for input in [input, native] {
        let outcome = prepare_evidence(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        let EvidencePlanOutcome::Refused { diagnostics } = outcome else {
            panic!("discovery must refuse before work: {outcome:?}")
        };
        assert_eq!(
            serde_json::to_value(diagnostics.items[0].code).unwrap(),
            "discovery_not_enabled"
        );
    }
}
