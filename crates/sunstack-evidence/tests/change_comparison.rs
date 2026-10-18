#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use std::collections::BTreeMap;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;
fn declaration(select: EvidenceSelection) -> EvidenceDeclaration {
    EvidenceDeclaration {
        id: DeclarationId::new("selected").unwrap(),
        input: SourceName::new("document").unwrap(),
        requirement: EvidenceRequirement::Required,
        select,
        max_items: None,
        max_bytes: None,
    }
}
async fn session(fixture: &GitFixture, scope: ChangeScope) -> EvidenceSession {
    let mut input = support::connected_inputs::plan(
        vec![fixture.binding(scope)],
        EvidenceSelection::WholeDocument {},
        EvidenceRequirement::Required,
    );
    input.sources = BTreeMap::from([(
        SourceName::new("document").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "source.md".into(),
        },
    )]);
    input.declarations.clear();
    EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn content_comparison_ignores_locations_and_detects_added_deleted_and_changed_paragraphs() {
    for (before, after, expected) in [
        (
            "U001: stable\n",
            Some("# Prefix\n\nU001: stable\n"),
            "unchanged",
        ),
        ("U001: old\n", Some("U001: new\n"), "changed"),
        (
            "U002: neighbor\n",
            Some("U001: new\n\nU002: neighbor\n"),
            "added",
        ),
        (
            "U001: removed\n\nU002: neighbor\n",
            Some("U002: neighbor\n"),
            "deleted",
        ),
        ("U001: removed\n", None, "deleted"),
    ] {
        let fixture = GitFixture::new();
        fixture.write("source.md", before);
        fixture.stage("source.md");
        fixture.commit(1000);
        if let Some(after) = after {
            fixture.write("source.md", after);
        } else {
            std::fs::remove_file(fixture.directory.path().join("source.md")).unwrap();
        }
        let mut session = session(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        )
        .await;
        let result = session
            .compare_change_sides(
                ChangeComparisonSubject::Declaration {
                    declaration: declaration(EvidenceSelection::Paragraph {
                        target_id: TargetId::new("U001").unwrap(),
                    }),
                },
                ComparisonLevel::Content,
            )
            .await;
        assert_eq!(
            serde_json::to_value(result).unwrap()["kind"],
            expected,
            "{before:?} -> {after:?}"
        );
    }
}

#[tokio::test]
async fn markdown_occurrence_fingerprints_ignore_location_but_keep_multiplicity_and_content() {
    for (before, after, expected) in [
        ("Use stable.\n", "# Prefix\n\nUse stable.\n", "unchanged"),
        ("Use old.\n", "Use new.\n", "changed"),
        ("Use stable.\n", "Use stable.\n\nUse stable.\n", "changed"),
        ("Nothing.\n", "Use new.\n", "added"),
        ("Use old.\n", "Nothing.\n", "deleted"),
    ] {
        let fixture = GitFixture::new();
        fixture.write("source.md", before);
        fixture.stage("source.md");
        fixture.commit(1000);
        fixture.write("source.md", after);
        let mut session = session(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        )
        .await;
        let select=serde_json::from_value(serde_json::json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"Use"})).unwrap();
        let result = session
            .compare_change_sides(
                ChangeComparisonSubject::Declaration {
                    declaration: declaration(select),
                },
                ComparisonLevel::Content,
            )
            .await;
        assert_eq!(
            serde_json::to_value(result).unwrap()["kind"],
            expected,
            "{before:?} -> {after:?}"
        );
    }
}

#[tokio::test]
async fn frontmatter_fingerprints_use_decoded_values_and_stay_conservative_for_bad_blocks() {
    for (before, after, expected) in [
        (
            "---\nname: stable\n---\nOld body\n",
            "---\nname: 'stable'\n---\nNew body\n",
            "unchanged",
        ),
        ("---\nname: old\n---\n", "---\nname: new\n---\n", "changed"),
        ("---\nother: true\n---\n", "---\nname: new\n---\n", "added"),
        (
            "---\nname: old\n---\n",
            "---\nother: true\n---\n",
            "deleted",
        ),
        ("---\nname: [\n---\n", "---\nname: {\n---\n", "changed"),
    ] {
        let fixture = GitFixture::new();
        fixture.write("source.md", before);
        fixture.stage("source.md");
        fixture.commit(1000);
        fixture.write("source.md", after);
        let mut session = session(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        )
        .await;
        let result = session
            .compare_change_sides(
                ChangeComparisonSubject::Declaration {
                    declaration: declaration(EvidenceSelection::FrontmatterField {
                        field: "name".into(),
                    }),
                },
                ComparisonLevel::Content,
            )
            .await;
        assert_eq!(
            serde_json::to_value(result).unwrap()["kind"],
            expected,
            "{before:?} -> {after:?}"
        );
    }
}
#[tokio::test]
async fn path_comparison_precedes_content_and_reuses_one_frozen_capture_after_live_mutation() {
    let fixture = GitFixture::new();
    fixture.write("source.md", "U001: stable\n");
    fixture.stage("source.md");
    fixture.commit(1000);
    fixture.write("source.md", "# Prefix\n\nU001: stable\n");
    let mut session = session(
        &fixture,
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Unstaged],
        },
    )
    .await;
    let subject = ChangeComparisonSubject::Declaration {
        declaration: declaration(EvidenceSelection::Paragraph {
            target_id: TargetId::new("U001").unwrap(),
        }),
    };
    assert!(
        matches!(session.compare_change_sides(subject.clone(),ComparisonLevel::Path).await,ChangeComparison::PathChanged{members} if members==vec![0])
    );
    assert_eq!(session.measurements().files_read, 2);
    fixture.write("source.md", "U001: later mutation\n");
    assert!(matches!(
        session
            .compare_change_sides(subject, ComparisonLevel::Content)
            .await,
        ChangeComparison::Unchanged {}
    ));
    let selected = session
        .select(&declaration(EvidenceSelection::WholeDocument {}))
        .await;
    assert!(
        matches!(selected.readiness,SelectionReadiness::Ready{content:SelectionContent::DocumentText{ref text,..},..} if text=="# Prefix\n\nU001: stable\n")
    );
    assert_eq!(session.measurements().files_read, 2);
    assert_eq!(session.close().change_selections.len(), 1);
}
#[tokio::test]
async fn full_scope_is_not_change_tracked_and_capture_limits_refuse_instead_of_proving_unchanged() {
    let fixture = GitFixture::new();
    fixture.write("source.md", "U001: old\n");
    fixture.stage("source.md");
    fixture.commit(1000);
    fixture.write("source.md", "U001: new\n");
    let mut full = session(&fixture, ChangeScope::Full {}).await;
    assert!(matches!(
        full.compare_change_sides(
            ChangeComparisonSubject::Input {
                input: SourceName::new("document").unwrap()
            },
            ComparisonLevel::Content
        )
        .await,
        ChangeComparison::NoBeforeSide {
            reason: NoBeforeSideReason::FullScope
        }
    ));
    assert_eq!(full.measurements().files_read, 0);
    let mut input = support::connected_inputs::plan(
        vec![fixture.binding(ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Unstaged],
        })],
        EvidenceSelection::WholeDocument {},
        EvidenceRequirement::Required,
    );
    input.sources = BTreeMap::from([(
        SourceName::new("document").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "source.md".into(),
        },
    )]);
    input.declarations.clear();
    input.limits.max_files = 1.try_into().unwrap();
    let mut limited = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        limited
            .compare_change_sides(
                ChangeComparisonSubject::Input {
                    input: SourceName::new("document").unwrap()
                },
                ComparisonLevel::Content
            )
            .await,
        ChangeComparison::Refused { .. }
    ));
    assert_eq!(limited.measurements().files_read, 1);
}

#[tokio::test]
async fn match_fingerprints_keep_multiplicity_and_capture_text_but_ignore_location_and_order() {
    for (before, after, expected) in [
        ("alpha beta", "prefix\nbeta alpha", "unchanged"),
        ("alpha alpha", "alpha", "changed"),
        ("alpha", "beta", "changed"),
        ("no selected token", "still no selected token", "unchanged"),
    ] {
        let fixture = GitFixture::new();
        fixture.write("source.md", before);
        fixture.stage("source.md");
        fixture.commit(1000);
        fixture.write("source.md", after);
        let mut session = session(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        )
        .await;
        let result = session
            .compare_change_sides(
                ChangeComparisonSubject::Declaration {
                    declaration: declaration(EvidenceSelection::RegexText {
                        pattern: "(?P<word>alpha|beta)".into(),
                    }),
                },
                ComparisonLevel::Content,
            )
            .await;
        assert_eq!(
            serde_json::to_value(result).unwrap()["kind"],
            expected,
            "{before} -> {after}"
        );
    }
}
#[tokio::test]
async fn cancelling_dirty_layers_still_expose_each_changed_member() {
    let fixture = GitFixture::new();
    fixture.write("source.md", "U001: original\n");
    fixture.stage("source.md");
    fixture.commit(1000);
    fixture.write("source.md", "U001: staged\n");
    fixture.stage("source.md");
    fixture.write("source.md", "U001: original\n");
    let mut session = session(
        &fixture,
        ChangeScope::Union {
            members: vec![
                ChangeScope::Uncommitted {
                    layers: vec![DirtyLayer::Staged],
                },
                ChangeScope::Uncommitted {
                    layers: vec![DirtyLayer::Unstaged],
                },
            ],
        },
    )
    .await;
    let result = session
        .compare_change_sides(
            ChangeComparisonSubject::Declaration {
                declaration: declaration(EvidenceSelection::Paragraph {
                    target_id: TargetId::new("U001").unwrap(),
                }),
            },
            ComparisonLevel::Content,
        )
        .await;
    assert!(matches!(result,ChangeComparison::Changed{members} if members==vec![0,1]));
    assert_eq!(session.close().change_selections[0].members.len(), 2);
}
#[tokio::test]
async fn unavailable_baseline_and_exhausted_history_cannot_prove_irrelevance() {
    let first = GitFixture::new();
    let second = GitFixture::new();
    for fixture in [&first, &second] {
        fixture.write("source.md", "U001: stable\n");
        fixture.stage("source.md");
        fixture.commit(1000);
    }
    let mut missing = session(
        &first,
        ChangeScope::RevisionDiff {
            base_revision: "missing-baseline".into(),
            head: RevisionHead::Worktree {},
        },
    )
    .await;
    assert!(matches!(
        missing
            .compare_change_sides(
                ChangeComparisonSubject::Input {
                    input: SourceName::new("document").unwrap()
                },
                ComparisonLevel::Path
            )
            .await,
        ChangeComparison::BeforeUnknown {
            reason: BeforeUnknownReason::BaselineUnavailable
        }
    ));
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
    let mut input = support::connected_inputs::plan(
        vec![first.binding(scope), other],
        EvidenceSelection::WholeDocument {},
        EvidenceRequirement::Required,
    );
    input.declarations.clear();
    input.sources = BTreeMap::from([
        (
            SourceName::new("first").unwrap(),
            SourceSpec::Document {
                repository: RepositoryId::new("fixture").unwrap(),
                path: "source.md".into(),
            },
        ),
        (
            SourceName::new("second").unwrap(),
            SourceSpec::Document {
                repository: RepositoryId::new("other").unwrap(),
                path: "source.md".into(),
            },
        ),
    ]);
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        panic!("discovery")
    };
    limits.max_history_commits = 1.try_into().unwrap();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        session
            .compare_change_sides(
                ChangeComparisonSubject::Input {
                    input: SourceName::new("first").unwrap()
                },
                ComparisonLevel::Path
            )
            .await,
        ChangeComparison::Added { .. }
    ));
    assert!(matches!(
        session
            .compare_change_sides(
                ChangeComparisonSubject::Input {
                    input: SourceName::new("second").unwrap()
                },
                ComparisonLevel::Path
            )
            .await,
        ChangeComparison::BeforeUnknown {
            reason: BeforeUnknownReason::HistoryIncomplete
        }
    ));
    let summary = session.close();
    assert_eq!(summary.change_selections.len(), 2);
    assert_eq!(
        summary.change_selections[0].anchor_utc,
        summary.change_selections[1].anchor_utc
    );
}

#[test]
fn native_comparison_results_reject_extra_fields_on_empty_outcomes() {
    assert!(
        serde_json::from_value::<ChangeComparison>(
            serde_json::json!({"kind":"unchanged","members":[0]})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ChangeComparison>(serde_json::json!({"kind":"unchanged"})).is_ok()
    );
}

#[tokio::test]
async fn unavailable_current_revision_is_refused_even_when_the_baseline_is_missing() {
    for base_revision in ["HEAD", "missing-baseline"] {
        let fixture = GitFixture::new();
        fixture.write("source.md", "U001: current\n");
        fixture.stage("source.md");
        fixture.commit(1000);
        let mut current = session(
            &fixture,
            ChangeScope::RevisionDiff {
                base_revision: base_revision.into(),
                head: RevisionHead::Commit {
                    revision: "missing-current".into(),
                },
            },
        )
        .await;
        let result = current
            .compare_change_sides(
                ChangeComparisonSubject::Input {
                    input: SourceName::new("document").unwrap(),
                },
                ComparisonLevel::Path,
            )
            .await;
        assert!(
            matches!(result, ChangeComparison::Refused { .. }),
            "unreadable current side must not become a baseline fallback: {result:?}"
        );
    }
}
