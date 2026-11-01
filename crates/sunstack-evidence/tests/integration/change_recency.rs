#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/mod.rs"]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn real_merge_parent_policy_preserves_each_parent_delta() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn base() {}\n");
    fixture.stage("main.rs");
    fixture.commit(1000);
    fixture.write("main.rs", "fn left() {}\n");
    fixture.stage("main.rs");
    let left = fixture.commit(1100);
    fixture.write("main.rs", "fn right() {}\n");
    fixture.stage("main.rs");
    let right = fixture.commit(1200);
    fixture.write("main.rs", "fn merged() {}\n");
    fixture.stage("main.rs");
    let tree = fixture
        .repository
        .find_tree(fixture.repository.index().unwrap().write_tree().unwrap())
        .unwrap();
    let signature = git2::Signature::new(
        "Fixture",
        "fixture@example.invalid",
        &git2::Time::new(1300, 0),
    )
    .unwrap();
    let parents = [
        fixture.repository.find_commit(left).unwrap(),
        fixture.repository.find_commit(right).unwrap(),
    ];
    let merge = fixture
        .repository
        .commit(
            None,
            &signature,
            &signature,
            "merge fixture",
            &tree,
            &[&parents[0], &parents[1]],
        )
        .unwrap();
    for (policy, expected) in [
        (CommitParentPolicy::FirstParent, 1),
        (CommitParentPolicy::EachParent, 2),
    ] {
        let capture = capture(
            &fixture,
            ChangeScope::CommitSet {
                commit_ids: vec![GitObjectId::new(merge.to_string()).unwrap()],
                parent_policy: policy,
            },
        )
        .await;
        assert_eq!(capture.members.len(), expected);
        for (position, member) in capture.members.iter().enumerate() {
            let expected_parent = if position == 0 { left } else { right };
            assert!(
                matches!(&member.basis, ChangeMemberBasis::Commit { parent: ParentBasis::Parent { commit_id, position: actual }, .. } if *actual == position && commit_id.as_str() == expected_parent.to_string())
            );
            let CapturedFileSide::Present { file } = &member.changes[0].before else {
                panic!("before source missing");
            };
            assert_eq!(
                file.text,
                if position == 0 {
                    "fn left() {}\n"
                } else {
                    "fn right() {}\n"
                }
            );
        }
    }
}

#[tokio::test]
async fn actual_mtime_selects_the_entire_diff_and_unknown_policy_is_explicit() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn first() { old(); }\n\nfn last() { old(); }\n");
    fixture.stage("main.rs");
    fixture.commit(1000);
    fixture.write(
        "main.rs",
        "fn first() { earlier_edit(); }\n\nfn last() { old(); }\n",
    );
    set_mtime(&fixture, 1000);
    let old = capture(
        &fixture,
        recent(
            DirtyLayer::Unstaged,
            UnknownRecencyPolicy::ExcludeWithOmission,
        ),
    )
    .await;
    assert!(old.members[0].changes.is_empty());
    fixture.write(
        "main.rs",
        "fn first() { earlier_edit(); }\n\nfn last() { recent_edit(); }\n",
    );
    set_mtime(&fixture, 1950);
    let recent_capture = capture(
        &fixture,
        recent(
            DirtyLayer::Unstaged,
            UnknownRecencyPolicy::ExcludeWithOmission,
        ),
    )
    .await;
    assert_eq!(recent_capture.members[0].changes[0].after_spans.len(), 2);
    let CapturedFileSide::Present { file } = &recent_capture.members[0].changes[0].after else {
        panic!("after source missing");
    };
    assert!(file.text.contains("earlier_edit"));
    assert!(file.text.contains("recent_edit"));
    // A future timestamp cannot establish recency relative to the invocation anchor.
    set_mtime(&fixture, 3000);
    for policy in [
        UnknownRecencyPolicy::ExcludeWithOmission,
        UnknownRecencyPolicy::IncludeWithDiagnostic,
    ] {
        let captured = capture(&fixture, recent(DirtyLayer::Unstaged, policy)).await;
        let include = policy == UnknownRecencyPolicy::IncludeWithDiagnostic;
        assert_eq!(captured.members[0].changes.len(), usize::from(include));
        assert!(
            matches!(&captured.members[0].recency, CapturedRecencyBasis::ApproximateFileMtime { included_unknown, excluded_unknown_paths, .. } if included_unknown.len() == usize::from(include) && excluded_unknown_paths.len() == usize::from(!include))
        );
        if include {
            assert!(
                captured
                    .diagnostics
                    .items
                    .iter()
                    .any(|diagnostic| diagnostic.code == EvidenceCode::RecencyUnknown)
            );
        } else {
            assert!(
                captured.sources.is_empty(),
                "unknown-recency exclusion must precede source freezing"
            );
        }
    }
    fixture.stage("main.rs");
    set_mtime(&fixture, 1950);
    let staged = capture(
        &fixture,
        recent(
            DirtyLayer::Staged,
            UnknownRecencyPolicy::ExcludeWithOmission,
        ),
    )
    .await;
    assert!(
        staged.members[0].changes.is_empty(),
        "worktree mtime is not staging time"
    );
    std::fs::remove_file(fixture.directory.path().join("main.rs")).unwrap();
    let deleted = capture(
        &fixture,
        recent(
            DirtyLayer::Unstaged,
            UnknownRecencyPolicy::IncludeWithDiagnostic,
        ),
    )
    .await;
    assert_eq!(deleted.members[0].changes.len(), 1);
    assert!(matches!(
        deleted.members[0].changes[0].after,
        CapturedFileSide::Absent
    ));
}
#[tokio::test]
async fn linked_worktree_root_is_distinct_and_lookback_uses_one_supplied_anchor() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn base() {}\n");
    fixture.stage("main.rs");
    fixture.commit(1900);
    let linked_directory = tempfile::tempdir().unwrap();
    let linked_path = linked_directory.path().join("linked");
    let linked = fixture
        .repository
        .worktree("fixture-linked", &linked_path, None)
        .unwrap();
    assert_eq!(
        linked.path().canonicalize().unwrap(),
        linked_path.canonicalize().unwrap()
    );
    std::fs::write(linked_path.join("main.rs"), "fn linked_only() {}\n").unwrap();
    let scope = ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    };
    let primary = capture(&fixture, scope.clone()).await;
    let mut binding = fixture.binding(scope);
    binding.root = linked_path;
    let secondary = capture_change_selection(
        ChangeCaptureRequest {
            binding,
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(primary.members[0].changes.is_empty());
    assert_eq!(secondary.members[0].changes.len(), 1);
    let window = TimeWindow::Lookback {
        seconds: 200.try_into().unwrap(),
    };
    let resolved = resolve_time_window(&window, anchor()).unwrap();
    assert_eq!(resolved.since_utc.timestamp(), 1800);
    assert_eq!(resolved.until_utc.timestamp(), 2000);
}
fn recent(layer: DirtyLayer, policy: UnknownRecencyPolicy) -> ChangeScope {
    ChangeScope::RecentUncommitted {
        layers: vec![layer],
        window: TimeWindow::Lookback {
            seconds: 200.try_into().unwrap(),
        },
        unknown_recency: policy,
    }
}
fn anchor() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(2000, 0).unwrap()
}
async fn capture(fixture: &GitFixture, scope: ChangeScope) -> CapturedChangeSelection {
    capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(scope),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
fn set_mtime(fixture: &GitFixture, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(fixture.directory.path().join("main.rs"))
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds)),
        )
        .unwrap();
}
