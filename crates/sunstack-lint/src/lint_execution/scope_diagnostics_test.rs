#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::{EffectiveLimits, OutputFormat, ScopeChoice, render};
use serde_json::{Value, json};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

fn discovery_limits() -> DiscoveryLimits {
    let EvidenceDiscovery::Bounded { limits } = EffectiveLimits::default().discovery().unwrap()
    else {
        panic!("default discovery must be bounded")
    };
    limits
}

fn report(root: &Path) -> LintReport {
    LintReport::empty(
        root.join("config.json"),
        ScopeChoice::Since { rev: "HEAD".into() },
        EffectiveLimits::default(),
    )
}

#[tokio::test]
async fn actual_source_changed_error_keeps_its_path_in_lint_json_and_sarif() {
    // Arrange: capture and verify a genuine Git/worktree change before mutation.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let asset = root.join("asset.txt");
    std::fs::write(&asset, "before\n").unwrap();
    let repository = git2::Repository::init(&root).unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("asset.txt")).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let author = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let revision = repository
        .commit(Some("HEAD"), &author, &author, "Before", &tree, &[])
        .unwrap();
    std::fs::write(&asset, "after\n").unwrap();
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: RepositoryBinding {
                key: RepositoryId::new("fixture").unwrap(),
                root: root.clone(),
                input_paths: vec![QueryPath::Glob {
                    pattern: "**".into(),
                }],
                change_scope: ChangeScope::RevisionDiff {
                    base_revision: revision.to_string(),
                    head: RevisionHead::Worktree {},
                },
            },
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();

    // Act: the existing snapshot guard produces the actual error consumed by lint.
    std::fs::write(&asset, "later\n").unwrap();
    let error = verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    let report = capture_refusal(report(&root), &root, error);

    // Assert: both report surfaces retain source identity and invalid status.
    assert_eq!(report.status, LintStatus::Invalid);
    let json: Value = serde_json::from_str(&render(&report, OutputFormat::Json)).unwrap();
    assert_eq!(json["diagnostics"][0]["path"], json!(asset));
    assert!(
        json["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("SourceChanged")
    );
    let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
    assert_eq!(
        sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"][0]["locations"][0]["physicalLocation"]
            ["artifactLocation"]["uri"],
        json!(asset.to_string_lossy())
    );
}

#[test]
fn non_file_cutoffs_name_the_configured_cap_without_fabricating_a_path() {
    // Arrange: history and time cutoffs have no particular stopped file.
    let root = Path::new("/fixture");
    let mut limits = discovery_limits();
    limits.max_history_commits = 7.try_into().unwrap();
    limits.max_local_time_ms = 29.try_into().unwrap();
    for (limit, message) in [
        (DiscoveryLimit::HistoryCommits, "max_history_commits=7"),
        (DiscoveryLimit::LocalTime, "max_local_time_ms=29"),
    ] {
        let coverage = DiscoveryCoverage::Limited {
            query_bounds: QueryBounds {
                repository_ids: vec![RepositoryId::new("fixture").unwrap()],
                requested_hops: 0,
                windows: vec![],
            },
            cutoffs: vec![DiscoveryCutoff {
                limit,
                frontier: vec![],
                path: None,
            }],
        };

        // Act: exercise the same projection used by the CLI's capture boundary.
        let report = incomplete_capture(report(root), root, &coverage, &limits);

        // Assert: a global incomplete stop stays global and names its actual cap.
        assert_eq!(report.status, LintStatus::Incomplete);
        let json = serde_json::to_value(&report).unwrap();
        assert!(
            json["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains(message)
        );
        assert!(json["diagnostics"][0].get("path").is_none());
        assert_eq!(json["incomplete"][0]["file"], Value::Null);
        let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
        let notification = &sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"][0];
        assert!(
            notification["message"]["text"]
                .as_str()
                .unwrap()
                .contains(message)
        );
        assert!(notification.get("locations").is_none());
    }
}
