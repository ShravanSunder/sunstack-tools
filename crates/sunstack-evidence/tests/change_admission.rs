#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;

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
