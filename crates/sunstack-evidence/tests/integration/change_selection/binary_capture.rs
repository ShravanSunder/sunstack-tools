use super::*;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const BEFORE: &str = "old\n";
const AFTER: &str = "new\n";

fn fixture_with_binary() -> (GitFixture, git2::Oid) {
    let fixture = GitFixture::new();
    std::fs::write(fixture.directory.path().join("asset.png"), PNG).unwrap();
    fixture.write("selected.md", BEFORE);
    fixture.stage("asset.png");
    fixture.stage("selected.md");
    let base = fixture.commit(1000);
    fixture.write("selected.md", AFTER);
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
fn assert_selected(captured: &CapturedChangeSelection, base: git2::Oid, head: SourceRevision) {
    assert_eq!(captured.members.len(), 1);
    let changes = &captured.members[0].changes;
    assert_eq!(changes.len(), 1);
    let change = &changes[0];
    assert_eq!(change.path, std::path::Path::new("selected.md"));
    assert_eq!(side_text(&change.before), Some(BEFORE));
    assert_eq!(side_text(&change.after), Some(AFTER));
    for (side, revision, identity) in [
        (
            &change.before,
            SourceRevision::Commit {
                commit_id: GitObjectId::new(base.to_string()).unwrap(),
                side: SourceSide::Before,
            },
            git2::Oid::hash_object(git2::ObjectType::Blob, BEFORE.as_bytes())
                .unwrap()
                .to_string(),
        ),
        (
            &change.after,
            head.clone(),
            if matches!(head, SourceRevision::Worktree { .. }) {
                content_digest(AFTER.as_bytes())
            } else {
                git2::Oid::hash_object(git2::ObjectType::Blob, AFTER.as_bytes())
                    .unwrap()
                    .to_string()
            },
        ),
    ] {
        let CapturedFileSide::Present { file } = side else {
            unreachable!()
        };
        assert_eq!(
            file.source,
            SourceRef::RepositoryFile {
                repository: RepositoryId::new("fixture").unwrap(),
                revision,
                path: "selected.md".into(),
                content_identity: identity
            }
        );
    }
    assert_eq!(
        change.before_spans,
        vec![ChangeSpan {
            byte_start: 0,
            byte_end: BEFORE.len()
        }]
    );
    assert_eq!(
        change.after_spans,
        vec![ChangeSpan {
            byte_start: 0,
            byte_end: AFTER.len()
        }]
    );
}

#[tokio::test]
async fn equal_binary_worktree_keeps_exact_changed_text_and_mutation_guard() {
    let (fixture, base) = fixture_with_binary();
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    assert_selected(
        &captured,
        base,
        SourceRevision::Worktree {
            snapshot: captured.snapshot.clone(),
            side: SourceSide::After,
        },
    );
    std::fs::write(fixture.directory.path().join("asset.png"), PNG).unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();
    std::fs::write(fixture.directory.path().join("asset.png"), b"\x89changed").unwrap();
    let error = verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(
        serde_json::to_value(error.diagnostic()).unwrap()["path"],
        "asset.png"
    );
}

#[tokio::test]
async fn equal_binary_blob_keeps_exact_changed_text_in_commit_and_index() {
    let (fixture, base) = fixture_with_binary();
    fixture.stage("selected.md");
    let commit = fixture.commit(1100);
    for scope in [
        ChangeScope::RevisionDiff {
            base_revision: base.to_string(),
            head: RevisionHead::Commit {
                revision: commit.to_string(),
            },
        },
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Staged],
        },
    ] {
        if matches!(scope, ChangeScope::Uncommitted { .. }) {
            fixture.write("selected.md", BEFORE);
            fixture.stage("selected.md");
        }
        let captured = capture_change_selection(
            ChangeCaptureRequest {
                context: ContextCapture::Full,
                binding: fixture.binding(scope),
                limits: discovery_limits(),
                anchor_utc: anchor(),
            },
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let change = &captured.members[0].changes[0];
        assert_eq!(captured.members[0].changes.len(), 1);
        assert_eq!(change.path, std::path::Path::new("selected.md"));
        assert_eq!(
            change.before_spans,
            vec![ChangeSpan {
                byte_start: 0,
                byte_end: BEFORE.len()
            }]
        );
        assert_eq!(
            change.after_spans,
            vec![ChangeSpan {
                byte_start: 0,
                byte_end: AFTER.len()
            }]
        );
        assert!(captured.sources.iter().all(|file| matches!(&file.source, SourceRef::RepositoryFile { path, .. } if path == std::path::Path::new("selected.md"))));
        if matches!(captured.scope, ChangeScope::RevisionDiff { .. }) {
            assert_selected(
                &captured,
                base,
                SourceRevision::Commit {
                    commit_id: GitObjectId::new(commit.to_string()).unwrap(),
                    side: SourceSide::After,
                },
            );
        } else {
            assert_eq!(side_text(&change.before), Some(AFTER));
            assert_eq!(side_text(&change.after), Some(BEFORE));
        }
    }
}

#[tokio::test]
async fn equal_binary_obeys_file_and_byte_budgets() {
    let (fixture, base) = fixture_with_binary();
    let mut exact = request(&fixture, base);
    // Two distinct reads per path: commit blob and worktree bytes.
    exact.limits.max_files = 4.try_into().unwrap();
    exact.limits.max_file_bytes = PNG.len().try_into().unwrap();
    exact.limits.max_total_bytes = (2 * PNG.len() + BEFORE.len() + AFTER.len())
        .try_into()
        .unwrap();
    let captured = capture_change_selection(exact.clone(), CancellationToken::new())
        .await
        .unwrap();
    assert!(matches!(
        captured.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(captured.members[0].changes.len(), 1);
    for limit in [
        DiscoveryLimit::Files,
        DiscoveryLimit::FileBytes,
        DiscoveryLimit::TotalBytes,
    ] {
        let mut limited = exact.clone();
        match limit {
            DiscoveryLimit::Files => limited.limits.max_files = 3.try_into().unwrap(),
            DiscoveryLimit::FileBytes => {
                limited.limits.max_file_bytes = (PNG.len() - 1).try_into().unwrap()
            }
            DiscoveryLimit::TotalBytes => {
                limited.limits.max_total_bytes = (2 * PNG.len() + BEFORE.len() + AFTER.len() - 1)
                    .try_into()
                    .unwrap()
            }
            _ => unreachable!(),
        }
        let captured = capture_change_selection(limited, CancellationToken::new())
            .await
            .unwrap();
        assert!(
            matches!(&captured.coverage, DiscoveryCoverage::Limited { cutoffs, .. } if cutoffs.iter().any(|cutoff| cutoff.limit == limit))
        );
        let expected_path = if limit == DiscoveryLimit::FileBytes {
            "asset.png"
        } else {
            "selected.md"
        };
        assert_eq!(
            serde_json::to_value(&captured.coverage).unwrap()["cutoffs"][0]["path"],
            expected_path
        );
        if limit != DiscoveryLimit::FileBytes {
            assert!(captured.members[0].changes.is_empty());
        }
    }
}

#[tokio::test]
async fn selected_invalid_before_or_after_still_refuses_encoding() {
    for invalid_before in [true, false] {
        let fixture = GitFixture::new();
        std::fs::write(
            fixture.directory.path().join("selected.md"),
            if invalid_before {
                PNG
            } else {
                BEFORE.as_bytes()
            },
        )
        .unwrap();
        fixture.stage("selected.md");
        let base = fixture.commit(1000);
        std::fs::write(
            fixture.directory.path().join("selected.md"),
            if invalid_before {
                AFTER.as_bytes()
            } else {
                PNG
            },
        )
        .unwrap();
        let error = capture_change_selection(request(&fixture, base), CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(error.code, EvidenceCode::UnsupportedEncoding);
        assert_eq!(
            serde_json::to_value(error.diagnostic()).unwrap()["path"],
            "selected.md"
        );
    }
}

#[tokio::test]
async fn changed_index_refusal_names_the_captured_path() {
    // Arrange: the index snapshot is stable before a second staged mutation.
    let (fixture, _) = fixture_with_binary();
    fixture.stage("selected.md");
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            context: ContextCapture::Full,
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Staged],
            }),
            limits: discovery_limits(),
            anchor_utc: anchor(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap();

    // Act: replace the staged blob after its immutable identity was captured.
    fixture.write("selected.md", "Later staged bytes.\n");
    fixture.stage("selected.md");
    let error = verify_change_capture(&captured, CancellationToken::new())
        .await
        .unwrap_err();

    // Assert: the refused source is explicit, independently of the error code.
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(
        serde_json::to_value(error.diagnostic()).unwrap()["path"],
        "selected.md"
    );
}

#[tokio::test]
async fn empty_additions_and_deletions_remain_distinct_from_absence() {
    let fixture = GitFixture::new();
    fixture.write("deleted.md", "");
    fixture.stage("deleted.md");
    let base = fixture.commit(1000);
    std::fs::remove_file(fixture.directory.path().join("deleted.md")).unwrap();
    fixture.write("added.md", "");
    let captured = capture_change_selection(request(&fixture, base), CancellationToken::new())
        .await
        .unwrap();
    let changes = &captured.members[0].changes;
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].path, std::path::Path::new("added.md"));
    assert_eq!(changes[0].before, CapturedFileSide::Absent);
    assert_eq!(side_text(&changes[0].after), Some(""));
    assert_eq!(changes[1].path, std::path::Path::new("deleted.md"));
    assert_eq!(side_text(&changes[1].before), Some(""));
    assert_eq!(changes[1].after, CapturedFileSide::Absent);
}
