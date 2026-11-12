use super::*;
use std::os::unix::fs::symlink;

fn linked_fixture(target: &std::path::Path) -> (GitFixture, git2::Oid) {
    let fixture = GitFixture::new();
    fixture.write("AGENTS.md", "Guidance.\n");
    fixture.write("selected.md", "Before.\n");
    symlink(target, fixture.directory.path().join("CLAUDE.md")).unwrap();
    for path in ["AGENTS.md", "selected.md", "CLAUDE.md"] {
        fixture.stage(path);
    }
    let base = fixture.commit(1000);
    fixture.write("selected.md", "After.\n");
    (fixture, base)
}
fn request(fixture: &GitFixture, base: git2::Oid) -> ChangeCaptureRequest {
    ChangeCaptureRequest {
        context: ContextCapture::Full,
        binding: fixture.binding(ChangeScope::RevisionDiff {
            base_revision: base.to_string(),
            head: RevisionHead::Worktree {},
        }),
        limits: discovery_limits(),
        anchor_utc: anchor(),
    }
}

#[tokio::test]
async fn unchanged_internal_symlink_keeps_only_the_changed_text() {
    let (fixture, base) = linked_fixture(std::path::Path::new("AGENTS.md"));
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(
        captured.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(captured.members[0].changes.len(), 1);
    assert_eq!(
        captured.members[0].changes[0].path,
        std::path::Path::new("selected.md")
    );
    let links: Vec<_> = captured.sources.iter().filter(|file| matches!(&file.source, SourceRef::RepositoryFile { path, .. } if path == std::path::Path::new("CLAUDE.md"))).collect();
    assert_eq!(links.len(), 2);
    assert!(links.iter().all(|file| file.text == "AGENTS.md"));
}

#[tokio::test]
async fn equal_link_payload_is_stable_but_payload_or_type_mutation_refuses() {
    let (fixture, base) = linked_fixture(std::path::Path::new("AGENTS.md"));
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    let alias = fixture.directory.path().join("CLAUDE.md");
    std::fs::remove_file(&alias).unwrap();
    symlink("AGENTS.md", &alias).unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();
    std::fs::remove_file(&alias).unwrap();
    symlink("different.md", &alias).unwrap();
    let error = verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(
        serde_json::to_value(error.diagnostic()).unwrap()["path"],
        "CLAUDE.md"
    );
    std::fs::remove_file(&alias).unwrap();
    symlink("AGENTS.md", &alias).unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();
    std::fs::remove_file(&alias).unwrap();
    std::fs::write(&alias, "AGENTS.md").unwrap();
    assert_eq!(
        verify_change_capture(&captured, CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        EvidenceCode::SourceChanged
    );
}

#[tokio::test]
async fn changed_link_captures_git_payload_instead_of_target_contents() {
    let (fixture, base) = linked_fixture(std::path::Path::new("AGENTS.md"));
    fixture.write("selected.md", "Before.\n");
    let alias = fixture.directory.path().join("CLAUDE.md");
    std::fs::remove_file(&alias).unwrap();
    symlink("missing.md", &alias).unwrap();
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(captured.members[0].changes.len(), 1);
    let change = &captured.members[0].changes[0];
    assert_eq!(change.path, std::path::Path::new("CLAUDE.md"));
    assert_eq!(side_text(&change.before), Some("AGENTS.md"));
    assert_eq!(side_text(&change.after), Some("missing.md"));
    assert_eq!(
        change.before_spans,
        vec![ChangeSpan {
            byte_start: 0,
            byte_end: 9
        }]
    );
    assert_eq!(
        change.after_spans,
        vec![ChangeSpan {
            byte_start: 0,
            byte_end: 10
        }]
    );
    let CapturedFileSide::Present { file } = &change.after else {
        unreachable!()
    };
    assert!(
        matches!(&file.source, SourceRef::RepositoryFile { content_identity, .. } if content_identity == &content_digest(b"missing.md"))
    );
}

#[tokio::test]
async fn outside_link_target_is_never_read_or_snapshot_verified() {
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("private.md");
    std::fs::write(&target, vec![0xff; 4096]).unwrap();
    let (fixture, base) = linked_fixture(&target);
    let mut limited = request(&fixture, base);
    // Only the link payload and the two small text files fit these budgets.
    let payload_bytes = target.as_os_str().as_encoded_bytes().len();
    limited.limits.max_file_bytes = payload_bytes.try_into().unwrap();
    let total = 2 * payload_bytes + 2 * "Guidance.\n".len() + "Before.\n".len() + "After.\n".len();
    limited.limits.max_total_bytes = total.try_into().unwrap();
    let captured = capture_change_selection(limited.clone(), CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(
        captured.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(captured.members[0].changes.len(), 1);
    assert_eq!(
        captured.members[0].changes[0].path,
        std::path::Path::new("selected.md")
    );
    std::fs::write(&target, b"external target changed").unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();
    limited.limits.max_file_bytes = (payload_bytes - 1).try_into().unwrap();
    let partial = capture_change_selection(limited, CancellationToken::new())
        .await
        .unwrap();
    assert!(
        matches!(partial.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == DiscoveryLimit::FileBytes))
    );
}

#[tokio::test]
async fn regular_file_replaced_by_link_to_identical_bytes_refuses_snapshot() {
    let fixture = GitFixture::new();
    fixture.write("original.md", "Same bytes.\n");
    fixture.write("copy.md", "Same bytes.\n");
    fixture.write("selected.md", "Before.\n");
    for path in ["original.md", "copy.md", "selected.md"] {
        fixture.stage(path);
    }
    let base = fixture.commit(1000);
    fixture.write("selected.md", "After.\n");
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    let original = fixture.directory.path().join("original.md");
    std::fs::remove_file(&original).unwrap();
    symlink("copy.md", &original).unwrap();
    assert_eq!(std::fs::read(&original).unwrap(), b"Same bytes.\n");
    assert_eq!(
        verify_change_capture(&captured, CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        EvidenceCode::SourceChanged
    );
}

#[tokio::test]
async fn path_equivalent_but_byte_different_link_payload_refuses_snapshot() {
    let (fixture, base) = linked_fixture(std::path::Path::new("dir/file"));
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    let alias = fixture.directory.path().join("CLAUDE.md");
    std::fs::remove_file(&alias).unwrap();
    symlink("dir//file", &alias).unwrap();
    assert_eq!(
        std::path::Path::new("dir/file"),
        std::path::Path::new("dir//file")
    );
    assert_eq!(
        std::fs::read_link(&alias)
            .unwrap()
            .as_os_str()
            .as_encoded_bytes(),
        b"dir//file"
    );
    assert_eq!(
        verify_change_capture(&captured, CancellationToken::new())
            .await
            .unwrap_err()
            .code,
        EvidenceCode::SourceChanged
    );
}

async fn capture_recent_link(
    fixture: &GitFixture,
    since: chrono::DateTime<chrono::Utc>,
    until: chrono::DateTime<chrono::Utc>,
) -> CapturedChangeSelection {
    let mut binding = fixture.binding(ChangeScope::RecentUncommitted {
        layers: vec![DirtyLayer::Unstaged],
        window: TimeWindow::Absolute {
            since_utc: since,
            until_utc: until,
        },
        unknown_recency: UnknownRecencyPolicy::ExcludeWithOmission,
    });
    binding.input_paths = vec![QueryPath::Exact {
        path: "CLAUDE.md".into(),
    }];
    capture_change_selection(
        ChangeCaptureRequest {
            context: ContextCapture::Full,
            binding,
            limits: discovery_limits(),
            anchor_utc: until + chrono::Duration::seconds(1),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
fn changed_link_and_mtime() -> (GitFixture, chrono::DateTime<chrono::Utc>) {
    let (fixture, _) = linked_fixture(std::path::Path::new("AGENTS.md"));
    fixture.write("other.md", "Target contents are not link recency.\n");
    let alias = fixture.directory.path().join("CLAUDE.md");
    std::fs::remove_file(&alias).unwrap();
    symlink("other.md", &alias).unwrap();
    let link_mtime = std::fs::symlink_metadata(&alias)
        .unwrap()
        .modified()
        .unwrap()
        .into();
    (fixture, link_mtime)
}
#[tokio::test]
async fn old_link_is_excluded_even_when_its_referent_is_recent() {
    let (fixture, link_mtime) = changed_link_and_mtime();
    // Use a controlled future anchor/window instead of changing link timestamps
    // or sleeping. The referent lies in the window; the link itself predates it.
    let recent = link_mtime + chrono::Duration::hours(1);
    std::fs::File::open(fixture.directory.path().join("other.md"))
        .unwrap()
        .set_modified(recent.into())
        .unwrap();
    let capture = capture_recent_link(
        &fixture,
        recent - chrono::Duration::seconds(1),
        recent + chrono::Duration::seconds(1),
    )
    .await;
    assert!(capture.members[0].changes.is_empty());
    assert!(capture.sources.is_empty());
}
#[tokio::test]
async fn recent_link_is_selected_with_old_or_dangling_referent() {
    let (fixture, link_mtime) = changed_link_and_mtime();
    let target = fixture.directory.path().join("other.md");
    std::fs::File::open(&target)
        .unwrap()
        .set_modified((link_mtime - chrono::Duration::hours(1)).into())
        .unwrap();
    for dangling in [false, true] {
        if dangling {
            std::fs::remove_file(&target).unwrap();
        }
        let capture = capture_recent_link(
            &fixture,
            link_mtime - chrono::Duration::seconds(1),
            link_mtime + chrono::Duration::seconds(1),
        )
        .await;
        assert_eq!(capture.members[0].changes.len(), 1);
        let change = &capture.members[0].changes[0];
        assert_eq!(change.path, std::path::Path::new("CLAUDE.md"));
        assert_eq!(side_text(&change.before), Some("AGENTS.md"));
        assert_eq!(side_text(&change.after), Some("other.md"));
        assert!(
            matches!(&capture.members[0].recency, CapturedRecencyBasis::ApproximateFileMtime { excluded_unknown_paths, included_unknown, .. } if excluded_unknown_paths.is_empty() && included_unknown.is_empty())
        );
    }
}
