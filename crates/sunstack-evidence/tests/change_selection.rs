#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;
fn anchor() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(2000, 0).unwrap()
}
fn side_text(side: &CapturedFileSide) -> Option<&str> {
    if let CapturedFileSide::Present { file } = side {
        Some(&file.text)
    } else {
        None
    }
}
#[tokio::test]
async fn real_git_layers_preserve_cancelling_staged_and_unstaged_edits() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn target() { first(); }\n");
    fixture.stage("main.rs");
    let first = fixture.commit(1000);
    fixture.write("main.rs", "fn target() { second(); }\n");
    fixture.stage("main.rs");
    fixture.write("main.rs", "fn target() { first(); }\n");
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Staged, DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(captured.members.len(), 2);
    for (member, layer, before, after) in [
        (
            &captured.members[0],
            DirtyLayer::Staged,
            "fn target() { first(); }\n",
            "fn target() { second(); }\n",
        ),
        (
            &captured.members[1],
            DirtyLayer::Unstaged,
            "fn target() { second(); }\n",
            "fn target() { first(); }\n",
        ),
    ] {
        assert_eq!(member.basis, ChangeMemberBasis::DirtyLayer { layer });
        assert_eq!(member.changes.len(), 1);
        let change = &member.changes[0];
        assert_eq!(side_text(&change.before), Some(before));
        assert_eq!(side_text(&change.after), Some(after));
        assert!(!change.before_spans.is_empty());
        assert!(!change.after_spans.is_empty());
    }
    let CapturedFileSide::Present { file } = &captured.members[0].changes[0].before else {
        unreachable!()
    };
    assert!(
        matches!(&file.source,SourceRef::RepositoryFile{revision:SourceRevision::Commit{commit_id,..},..} if commit_id.as_str()==first.to_string())
    );
}
#[tokio::test]
async fn real_root_commit_has_only_after_side_and_full_scope_has_no_changed_seeds() {
    let fixture = GitFixture::new();
    fixture.write("root.ts", "export function target() {}\n");
    fixture.stage("root.ts");
    let root = fixture.commit(1000);
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::CommitSet {
                commit_ids: vec![GitObjectId::new(root.to_string()).unwrap()],
                parent_policy: CommitParentPolicy::EachParent,
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let expected_blob = fixture
        .repository
        .find_commit(root)
        .unwrap()
        .tree()
        .unwrap()
        .get_path(std::path::Path::new("root.ts"))
        .unwrap()
        .id()
        .to_string();
    let CapturedFileSide::Present { file } = &captured.members[0].changes[0].after else {
        unreachable!()
    };
    assert!(
        matches!(&file.source, SourceRef::RepositoryFile { content_identity, .. } if content_identity == &expected_blob)
    );
    assert_eq!(captured.members.len(), 1);
    assert!(matches!(
        captured.members[0].basis,
        ChangeMemberBasis::Commit {
            parent: ParentBasis::Root,
            ..
        }
    ));
    assert_eq!(
        captured.members[0].changes[0].before,
        CapturedFileSide::Absent
    );
    assert_eq!(
        side_text(&captured.members[0].changes[0].after),
        Some("export function target() {}\n")
    );
    assert_eq!(
        capture_change_selection(
            ChangeCaptureRequest {
                binding: fixture.binding(ChangeScope::Full),
                limits: discovery_limits(),
                anchor_utc: anchor()
            },
            CancellationToken::new()
        )
        .await
        .unwrap_err()
        .code,
        EvidenceCode::NoChangeSelection
    );
}

#[tokio::test]
async fn git_insertion_uses_a_narrow_before_anchor_instead_of_the_whole_file() {
    let fixture = GitFixture::new();
    let before = "fn unchanged() {}\n";
    fixture.write("main.rs", before);
    fixture.stage("main.rs");
    fixture.commit(1000);
    fixture.write("main.rs", "fn unchanged() {}\nfn added() {}\n");
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        captured.members[0].changes[0].before_spans,
        vec![ChangeSpan {
            byte_start: before.len(),
            byte_end: before.len()
        }]
    );
}
#[tokio::test]
async fn final_verification_refuses_real_file_and_index_mutation_but_same_bytes_are_stable() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn first() {}\n");
    fixture.stage("main.rs");
    fixture.commit(1000);
    fixture.write("main.rs", "fn second() {}\n");
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    fixture.write("main.rs", "fn second() {}\n");
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();
    fixture.write("main.rs", "fn third() {}\n");
    let error = verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(error.diagnostic().retry_advice, RetryAdvice::CaptureAgain);
    assert_eq!(
        side_text(&captured.members[0].changes[0].after),
        Some("fn second() {}\n")
    );
    fixture.stage("main.rs");
    assert_eq!(
        verify_change_capture(&captured, CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        EvidenceCode::SourceChanged
    );
}
#[tokio::test]
async fn reachable_nonmonotonic_commit_time_and_real_history_cutoff() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn recent() {}\n");
    fixture.stage("main.rs");
    let recent = fixture.commit(1500);
    fixture.write("main.rs", "fn old_head() {}\n");
    fixture.stage("main.rs");
    fixture.commit(100);
    let scope = ChangeScope::RecentCommits {
        reference: "HEAD".into(),
        window: TimeWindow::Absolute {
            since_utc: chrono::DateTime::from_timestamp(1400, 0).unwrap(),
            until_utc: chrono::DateTime::from_timestamp(1600, 0).unwrap(),
        },
        parent_policy: CommitParentPolicy::FirstParent,
    };
    let request = ChangeCaptureRequest {
        binding: fixture.binding(scope),
        limits: discovery_limits(),
        anchor_utc: anchor(),
    };
    let captured = capture_change_selection(request.clone(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(captured.members.len(), 1);
    assert!(
        matches!(&captured.members[0].basis,ChangeMemberBasis::Commit{commit_id,..} if commit_id.as_str()==recent.to_string())
    );
    let CapturedRecencyBasis::CommitTime { window } = &captured.members[0].recency else {
        unreachable!()
    };
    assert_eq!(window.anchor_utc, anchor());
    let mut limited = request;
    limited.limits.max_history_commits = 1.try_into().unwrap();
    let captured = capture_change_selection(limited, CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(captured.coverage,DiscoveryCoverage::Limited{cutoffs,..} if cutoffs.iter().any(|cutoff|cutoff.limit==DiscoveryLimit::HistoryCommits))
    );
}
#[tokio::test]
async fn explicit_commit_events_preserve_revert_while_endpoint_is_empty_and_union_order_is_kept() {
    let fixture = GitFixture::new();
    fixture.write("main.rs", "fn one() {}\n");
    fixture.stage("main.rs");
    let base = fixture.commit(1000);
    fixture.write("main.rs", "fn two() {}\n");
    fixture.stage("main.rs");
    let change = fixture.commit(1100);
    fixture.write("main.rs", "fn one() {}\n");
    fixture.stage("main.rs");
    let revert = fixture.commit(1200);
    let event_scope = ChangeScope::CommitSet {
        commit_ids: vec![
            GitObjectId::new(change.to_string()).unwrap(),
            GitObjectId::new(revert.to_string()).unwrap(),
        ],
        parent_policy: CommitParentPolicy::FirstParent,
    };
    let endpoint = ChangeScope::RevisionDiff {
        base_revision: base.to_string(),
        head: RevisionHead::Commit {
            revision: revert.to_string(),
        },
    };
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Union {
                members: vec![event_scope, endpoint],
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(captured.members.len(), 3);
    assert_eq!(
        side_text(&captured.members[0].changes[0].after),
        Some("fn two() {}\n")
    );
    assert_eq!(
        side_text(&captured.members[1].changes[0].after),
        Some("fn one() {}\n")
    );
    assert!(captured.members[2].changes.is_empty());
    let nested = ChangeScope::Union {
        members: vec![ChangeScope::Union {
            members: vec![ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Staged],
            }],
        }],
    };
    assert_eq!(
        nested.validate().unwrap_err().code,
        EvidenceCode::InvalidRepositoryBinding
    );
}
#[tokio::test]
async fn untracked_selection_is_explicit_and_rename_keeps_captured_sides() {
    let fixture = GitFixture::new();
    fixture.write("old.rs", "fn moved() {}\n");
    fixture.stage("old.rs");
    fixture.commit(1000);
    std::fs::rename(
        fixture.directory.path().join("old.rs"),
        fixture.directory.path().join("new.rs"),
    )
    .unwrap();
    let request = |layers| ChangeCaptureRequest {
        binding: fixture.binding(ChangeScope::Uncommitted { layers }),
        limits: discovery_limits(),
        anchor_utc: anchor(),
    };
    let tracked = capture_change_selection(
        request(vec![DirtyLayer::Unstaged]),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(tracked.members[0].changes.len(), 1);
    assert_eq!(
        tracked.members[0].changes[0].path,
        std::path::PathBuf::from("old.rs")
    );
    assert_eq!(
        tracked.members[0].changes[0].after,
        CapturedFileSide::Absent
    );
    let untracked = capture_change_selection(
        request(vec![DirtyLayer::Untracked]),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(untracked.members[0].changes.len(), 1);
    assert_eq!(
        untracked.members[0].changes[0].before,
        CapturedFileSide::Absent
    );
    assert_eq!(
        side_text(&untracked.members[0].changes[0].after),
        Some("fn moved() {}\n")
    );
}

#[tokio::test]
async fn explicit_commit_events_obey_the_history_budget() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function first() {}");
    fixture.stage("source.ts");
    let first = fixture.commit(1000);
    fixture.write("source.ts", "function second() {}");
    fixture.stage("source.ts");
    let second = fixture.commit(1100);
    let mut limits = discovery_limits();
    limits.max_history_commits = 1.try_into().unwrap();
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::CommitSet {
                commit_ids: vec![
                    GitObjectId::new(first.to_string()).unwrap(),
                    GitObjectId::new(second.to_string()).unwrap(),
                ],
                parent_policy: CommitParentPolicy::FirstParent,
            }),
            limits,
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        captured.members.len(),
        1,
        "the second requested commit must be omitted with a cutoff"
    );
    assert!(
        matches!(&captured.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::HistoryCommits))
    );
}
