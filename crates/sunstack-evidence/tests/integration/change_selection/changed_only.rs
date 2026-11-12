use super::*;

#[path = "changed_only/blind_spots.rs"]
mod blind_spots;
#[path = "changed_only/capture_parity.rs"]
mod capture_parity;
use std::path::{Path, PathBuf};

fn request(fixture: &GitFixture, base: git2::Oid, context: ContextCapture) -> ChangeCaptureRequest {
    ChangeCaptureRequest {
        context,
        binding: fixture.binding(ChangeScope::RevisionDiff {
            base_revision: base.to_string(),
            head: RevisionHead::Worktree {},
        }),
        limits: discovery_limits(),
        anchor_utc: anchor(),
    }
}
fn changed_paths(capture: &CapturedChangeSelection) -> Vec<PathBuf> {
    capture
        .members
        .iter()
        .flat_map(|member| member.changes.iter().map(|change| change.path.clone()))
        .collect()
}
fn signature(capture: &CapturedChangeSelection) -> serde_json::Value {
    serde_json::json!(
        capture
            .members
            .iter()
            .map(|member| {
                member.changes.iter().map(|change| serde_json::json!({
        "path":change.path,"before":side_text(&change.before),"after":side_text(&change.after),
        "before_spans":change.before_spans,"after_spans":change.after_spans
    })).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

#[tokio::test]
async fn changed_only_acquires_two_changed_sides_under_a_small_file_budget() {
    // Arrange: the repository has far more unchanged sources than the acquisition budget.
    let fixture = GitFixture::new();
    for index in 0..32 {
        let path = format!("stable-{index:02}.md");
        fixture.write(&path, "unchanged\n");
        fixture.stage(&path);
    }
    fixture.write("z-changed.md", "before\n");
    fixture.stage("z-changed.md");
    let base = fixture.commit(1000);
    fixture.write("z-changed.md", "after\n");
    let mut changed = request(&fixture, base, ContextCapture::ChangedOnly);
    changed.limits.max_files = 2.try_into().unwrap();

    // Act: raw identity probes select only the changed path for acquisition.
    let capture = capture_change_selection(changed, CancellationToken::new())
        .await
        .unwrap();

    // Assert: the file cap governs two actual acquisitions, not the unchanged tree.
    assert!(matches!(
        capture.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(changed_paths(&capture), vec![PathBuf::from("z-changed.md")]);
    assert_eq!(capture.acquired_file_count(), 2);
    assert!(capture.sources.iter().all(|file| matches!(&file.source, SourceRef::RepositoryFile { path, .. } if path == Path::new("z-changed.md"))));
}

#[tokio::test]
async fn changed_only_omits_unchanged_context_while_full_retains_it() {
    // Arrange: one stable file provides context to existing connected-evidence callers.
    let fixture = GitFixture::new();
    fixture.write("stable.md", "context\n");
    fixture.stage("stable.md");
    fixture.write("changed.md", "before\n");
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    fixture.write("changed.md", "after\n");

    // Act: run both explicit modes against the same real repository views.
    let changed = capture_change_selection(
        request(&fixture, base, ContextCapture::ChangedOnly),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let full = capture_change_selection(
        request(&fixture, base, ContextCapture::Full),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    // Assert: changes remain equal; only full-context capture retains unchanged text.
    assert_eq!(signature(&changed), signature(&full));
    assert_eq!(changed.acquired_file_count(), 2);
    assert_eq!(full.acquired_file_count(), 4);
    assert_eq!(ContextCapture::default(), ContextCapture::Full);
    assert!(full.sources.iter().any(|file| matches!(&file.source, SourceRef::RepositoryFile { path, .. } if path == Path::new("stable.md"))));
    assert!(!changed.sources.iter().any(|file| matches!(&file.source, SourceRef::RepositoryFile { path, .. } if path == Path::new("stable.md"))));
}

#[tokio::test]
async fn changed_only_preserves_staged_unstaged_untracked_and_revision_answers() {
    // Arrange: independent dirty layers, a rename, a deletion, and ignored untracked content.
    let fixture = GitFixture::new();
    for file in [
        "staged.md",
        "unstaged.md",
        "deleted.md",
        "old-name.md",
        "stable.md",
    ] {
        fixture.write(file, "before\n");
        fixture.stage(file);
    }
    fixture.write(".gitignore", "ignored.md\n");
    fixture.stage(".gitignore");
    let base = fixture.commit(1000);
    fixture.write("staged.md", "staged\n");
    fixture.stage("staged.md");
    fixture.write("unstaged.md", "unstaged\n");
    std::fs::remove_file(fixture.directory.path().join("deleted.md")).unwrap();
    std::fs::rename(
        fixture.directory.path().join("old-name.md"),
        fixture.directory.path().join("new-name.md"),
    )
    .unwrap();
    fixture.write("untracked.md", "new\n");
    fixture.write("ignored.md", "invisible\n");
    let scopes = [
        (
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Staged],
            },
            vec!["staged.md"],
        ),
        (
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
            vec!["deleted.md", "old-name.md", "unstaged.md"],
        ),
        (
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Untracked],
            },
            vec!["new-name.md", "untracked.md"],
        ),
        (
            ChangeScope::RevisionDiff {
                base_revision: base.to_string(),
                head: RevisionHead::Worktree {},
            },
            vec![
                "deleted.md",
                "new-name.md",
                "old-name.md",
                "staged.md",
                "unstaged.md",
                "untracked.md",
            ],
        ),
    ];
    for (scope, expected) in scopes {
        let mut changed = request(&fixture, base, ContextCapture::ChangedOnly);
        changed.binding.change_scope = scope;
        let mut full = changed.clone();
        full.context = ContextCapture::Full;
        // Act: both capture strategies see the same catalogues and exact file bytes.
        let changed = capture_change_selection(changed, CancellationToken::new())
            .await
            .unwrap();
        let full = capture_change_selection(full, CancellationToken::new())
            .await
            .unwrap();
        // Assert: no dirty layer, deleted side, rename side, or ignored-path policy changed.
        assert_eq!(signature(&changed), signature(&full));
        assert_eq!(
            changed_paths(&changed),
            expected.into_iter().map(PathBuf::from).collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn changed_only_still_verifies_changed_and_unchanged_mutable_files() {
    for path in ["changed.md", "stable.md"] {
        // Arrange: stable text is hash-guarded without being frozen or acquired.
        let fixture = GitFixture::new();
        fixture.write("changed.md", "before\n");
        fixture.stage("changed.md");
        fixture.write("stable.md", "stable\n");
        fixture.stage("stable.md");
        let base = fixture.commit(1000);
        fixture.write("changed.md", "after\n");
        let capture = capture_change_selection(
            request(&fixture, base, ContextCapture::ChangedOnly),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        verify_change_capture(&capture, CancellationToken::new())
            .await
            .unwrap();
        // Act: a file mutates after the completed capture.
        fixture.write(path, "mutated\n");
        let error = verify_change_capture(&capture, CancellationToken::new())
            .await
            .unwrap_err();
        // Assert: identity-only guards retain the existing SourceChanged refusal and path.
        assert_eq!(error.code, EvidenceCode::SourceChanged);
        assert_eq!(error.path, Some(PathBuf::from(path)));
    }
}

#[tokio::test]
async fn unchanged_hashing_is_bounded_by_file_and_total_bytes() {
    for limit in [DiscoveryLimit::FileBytes, DiscoveryLimit::TotalBytes] {
        // Arrange: a single unchanged file requires no acquisition but still costs hashing bytes.
        let fixture = GitFixture::new();
        fixture.write("stable.md", "unchanged\n");
        fixture.stage("stable.md");
        let base = fixture.commit(1000);
        let mut capture = request(&fixture, base, ContextCapture::ChangedOnly);
        capture.limits.max_files = 1.try_into().unwrap();
        if limit == DiscoveryLimit::FileBytes {
            capture.limits.max_file_bytes = 8.try_into().unwrap();
        } else {
            capture.limits.max_total_bytes = 8.try_into().unwrap();
        }
        // Act: the identity probe meets the existing byte bound.
        let capture = capture_change_selection(capture, CancellationToken::new())
            .await
            .unwrap();
        // Assert: report a bounded incomplete discovery with the actual source, no new acquisition.
        assert!(
            matches!(&capture.coverage, DiscoveryCoverage::Limited { cutoffs, .. }
            if cutoffs.iter().any(|cutoff| cutoff.limit == limit
                && cutoff.path == Some(PathBuf::from("stable.md"))))
        );
        assert_eq!(capture.acquired_file_count(), 0);
        assert!(capture.sources.is_empty());
    }
}

#[tokio::test]
async fn equal_binary_is_skipped_but_changed_binary_still_refuses_with_its_path() {
    // Arrange: invalid UTF-8 is unchanged alongside a changed text document.
    let fixture = GitFixture::new();
    std::fs::write(fixture.directory.path().join("asset.png"), b"\x89PNG\x00").unwrap();
    fixture.stage("asset.png");
    fixture.write("changed.md", "before\n");
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    fixture.write("changed.md", "after\n");
    let mut capture = request(&fixture, base, ContextCapture::ChangedOnly);
    capture.limits.max_files = 2.try_into().unwrap();
    let unchanged = capture_change_selection(capture.clone(), CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(
        unchanged.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(unchanged.acquired_file_count(), 2);
    // Act: changing the binary now makes it an acquired change witness.
    std::fs::write(fixture.directory.path().join("asset.png"), b"\x89PNG\x01").unwrap();
    let error = capture_change_selection(capture, CancellationToken::new())
        .await
        .unwrap_err();
    // Assert: encoding refusal and path remain exactly the existing contract.
    assert_eq!(error.code, EvidenceCode::UnsupportedEncoding);
    assert_eq!(error.path, Some(PathBuf::from("asset.png")));
}

#[cfg(unix)]
#[tokio::test]
async fn equal_mode_and_symlink_payloads_do_not_acquire_context() {
    use std::os::unix::fs::PermissionsExt;
    // Arrange: an executable-bit change and stable dangling symlink carry identical bytes.
    let fixture = GitFixture::new();
    fixture.write("mode.md", "same\n");
    fixture.stage("mode.md");
    std::os::unix::fs::symlink("missing.md", fixture.directory.path().join("link.md")).unwrap();
    fixture.stage("link.md");
    let base = fixture.commit(1000);
    let file = fixture.directory.path().join("mode.md");
    let mut permissions = std::fs::metadata(&file).unwrap().permissions();
    permissions.set_mode(permissions.mode() ^ 0o100);
    std::fs::set_permissions(&file, permissions).unwrap();
    // Act: only content/presence identities decide acquisition.
    let stable = capture_change_selection(
        request(&fixture, base, ContextCapture::ChangedOnly),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    // Assert: neither metadata-only nor equal link payload becomes a change.
    assert!(changed_paths(&stable).is_empty());
    assert_eq!(stable.acquired_file_count(), 0);
    assert!(stable.sources.is_empty());
    std::fs::remove_file(fixture.directory.path().join("link.md")).unwrap();
    std::os::unix::fs::symlink(
        "different-missing.md",
        fixture.directory.path().join("link.md"),
    )
    .unwrap();
    let changed = capture_change_selection(
        request(&fixture, base, ContextCapture::ChangedOnly),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(changed_paths(&changed), vec![PathBuf::from("link.md")]);
    assert_eq!(changed.acquired_file_count(), 2);
}

#[tokio::test]
async fn gitlinks_keep_existing_catalogue_and_index_refusal_semantics() {
    // Arrange: an owned checked-out nested repository is registered as a real index gitlink.
    let fixture = GitFixture::new();
    fixture.write("stable.md", "stable\n");
    fixture.stage("stable.md");
    let module_root = fixture.directory.path().join("module");
    std::fs::create_dir(&module_root).unwrap();
    let module = git2::Repository::init(&module_root).unwrap();
    std::fs::write(module_root.join("child.md"), "child\n").unwrap();
    let mut module_index = module.index().unwrap();
    module_index.add_path(Path::new("child.md")).unwrap();
    module_index.write().unwrap();
    let tree_id = module_index.write_tree().unwrap();
    let tree = module.find_tree(tree_id).unwrap();
    let author = git2::Signature::now("Fixture Author", "fixture@example.invalid").unwrap();
    let commit = module
        .commit(Some("HEAD"), &author, &author, "nested fixture", &tree, &[])
        .unwrap();
    let mut index = fixture.repository.index().unwrap();
    let mut entry = index.get_path(Path::new("stable.md"), 0).unwrap();
    entry.path = b"module".to_vec();
    entry.mode = 0o160000;
    entry.id = commit;
    entry.flags = 6;
    entry.flags_extended = 0;
    entry.file_size = 0;
    index.add(&entry).unwrap();
    index.write().unwrap();
    let base = fixture.commit(1000);
    // Act: revision-to-worktree uses today's blob-only tree catalogue and descended worktree.
    let changed = capture_change_selection(
        request(&fixture, base, ContextCapture::ChangedOnly),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let full = capture_change_selection(
        request(&fixture, base, ContextCapture::Full),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    // Assert: the nested file appears as added; no new Git submodule-pointer semantics are invented.
    assert_eq!(signature(&changed), signature(&full));
    assert_eq!(
        changed_paths(&changed),
        vec![PathBuf::from("module/child.md")]
    );
    assert_eq!(changed.acquired_file_count(), 1);
    for layer in [DirtyLayer::Staged, DirtyLayer::Unstaged] {
        for context in [ContextCapture::Full, ContextCapture::ChangedOnly] {
            let mut request = request(&fixture, base, context);
            request.binding.change_scope = ChangeScope::Uncommitted {
                layers: vec![layer],
            };
            let error = capture_change_selection(request, CancellationToken::new())
                .await
                .unwrap_err();
            // The existing index catalogue treats a gitlink as a blob and refuses its commit OID.
            assert_eq!(error.code, EvidenceCode::RevisionUnavailable);
            assert_eq!(error.path, Some(PathBuf::from("module")));
        }
    }
}

#[tokio::test]
async fn identity_only_index_entries_still_refuse_index_mutation_after_capture() {
    // Arrange: both views are equal, so no file is acquired or frozen.
    let fixture = GitFixture::new();
    fixture.write("stable.md", "stable\n");
    fixture.stage("stable.md");
    let base = fixture.commit(1000);
    let mut request = request(&fixture, base, ContextCapture::ChangedOnly);
    request.binding.change_scope = ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    };
    let capture = capture_change_selection(request, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(capture.acquired_file_count(), 0);
    assert!(capture.sources.is_empty());
    // Act: replace only the index blob; leave the current file bytes unchanged.
    let replacement = fixture.repository.blob(b"staged replacement\n").unwrap();
    let mut index = fixture.repository.index().unwrap();
    let mut entry = index.get_path(Path::new("stable.md"), 0).unwrap();
    entry.id = replacement;
    index.add(&entry).unwrap();
    index.write().unwrap();
    let error = verify_change_capture(&capture, CancellationToken::new())
        .await
        .unwrap_err();
    // Assert: the identity probe kept an index guard even though acquisition was skipped.
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(error.path, Some(PathBuf::from("stable.md")));
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("stable.md")).unwrap(),
        "stable\n"
    );
}
