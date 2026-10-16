#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
mod support;

#[tokio::test]
async fn full_context_reads_plain_directory_without_git_and_preserves_every_source() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("one.ts"), "function first() {}").unwrap();
    std::fs::write(directory.path().join("two.ts"), "function second() {}").unwrap();
    let binding = RepositoryBinding {
        key: RepositoryId::new("plain").unwrap(),
        root: directory.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let capture = capture_repository_context(
        ChangeCaptureRequest {
            binding: binding.clone(),
            limits: support::discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(capture.sources.len(), 2);
    assert!(capture.members.is_empty());
    assert!(matches!(
        capture.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    let mut limits = support::discovery_limits();
    limits.max_files = 1.try_into().unwrap();
    let limited = capture_repository_context(
        ChangeCaptureRequest {
            binding,
            limits,
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(limited.sources.len(), 1);
    assert!(
        matches!(limited.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::Files))
    );
}
