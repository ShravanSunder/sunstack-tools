#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::test_support as support;
use crate::*;
use tokio_util::sync::CancellationToken;

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
        change_scope: ChangeScope::Full {},
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
            binding: binding.clone(),
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

    let mut file_bytes = support::discovery_limits();
    file_bytes.max_file_bytes = 1.try_into().unwrap();
    let mut total_bytes = support::discovery_limits();
    total_bytes.max_total_bytes = 1.try_into().unwrap();
    for (limits, expected_limit) in [
        (file_bytes, DiscoveryLimit::FileBytes),
        (total_bytes, DiscoveryLimit::TotalBytes),
    ] {
        let limited = capture_repository_context(
            ChangeCaptureRequest {
                binding: binding.clone(),
                limits,
                anchor_utc: chrono::Utc::now(),
            },
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(limited.sources.is_empty());
        assert!(
            matches!(limited.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == expected_limit))
        );
    }
}
