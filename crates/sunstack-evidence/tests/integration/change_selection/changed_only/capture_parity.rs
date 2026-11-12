use super::*;
use std::collections::BTreeMap;

fn fingerprint(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, String>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
            let path = entry.path();
            let kind = std::fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                result.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    format!(
                        "link:{}",
                        content_digest(target.as_os_str().as_encoded_bytes())
                    ),
                );
            } else if kind.is_file() {
                result.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    format!("file:{}", content_digest(&std::fs::read(&path).unwrap())),
                );
            } else if kind.is_dir() {
                visit(root, &path, result);
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
fn git_guard(root: &Path) -> serde_json::Value {
    let repository = git2::Repository::open(root).unwrap();
    serde_json::json!({
        "head":repository.head().unwrap().peel_to_commit().unwrap().id().to_string(),
        "index_sha256":content_digest(&std::fs::read(repository.path().join("index")).unwrap())
    })
}

/// Optional immutable inputs use the same permanent public-API consumer as the owned CI fixture.
#[tokio::test]
async fn public_capture_modes_produce_equal_raw_changes_with_fewer_acquisitions()
-> Result<(), String> {
    let input = (
        std::env::var_os("SUNSTACK_CAPTURE_PARITY_ROOT"),
        std::env::var_os("SUNSTACK_CAPTURE_PARITY_BASE"),
        std::env::var_os("SUNSTACK_CAPTURE_PARITY_RECEIPT"),
    );
    let owned;
    let (root, base, receipt) = match input {
        (None, None, None) => {
            owned = GitFixture::new();
            for index in 0..20 {
                let file = format!("stable-{index}.md");
                owned.write(&file, "context\n");
                owned.stage(&file);
            }
            owned.write("changed.md", "before\n");
            owned.stage("changed.md");
            let base = owned.commit(1000).to_string();
            owned.write("changed.md", "after\n");
            (owned.directory.path().to_path_buf(), base, None)
        }
        (Some(root), Some(base), Some(receipt)) => (
            PathBuf::from(root).canonicalize().unwrap(),
            base.into_string().unwrap(),
            Some(PathBuf::from(receipt)),
        ),
        _ => return Err("capture parity requires root, base and receipt together".into()),
    };
    // Arrange: bind identical immutable inputs/byte limits, with only the oracle file cap raised.
    let original = fingerprint(&root);
    let original_git = git_guard(&root);
    let mut limits = discovery_limits();
    limits.max_files = 2000.try_into().unwrap();
    limits.max_file_bytes = 1_048_576.try_into().unwrap();
    limits.max_total_bytes = 67_108_864.try_into().unwrap();
    limits.max_matches = 20_000.try_into().unwrap();
    limits.max_unit_visits = 10_000.try_into().unwrap();
    limits.max_relationship_candidates = 10_000.try_into().unwrap();
    limits.max_history_commits = 1000.try_into().unwrap();
    let request = ChangeCaptureRequest {
        context: ContextCapture::ChangedOnly,
        binding: RepositoryBinding {
            key: RepositoryId::new("lint").unwrap(),
            root: root.clone(),
            input_paths: vec![QueryPath::Glob {
                pattern: "**".into(),
            }],
            change_scope: ChangeScope::RevisionDiff {
                base_revision: base.clone(),
                head: RevisionHead::Worktree {},
            },
        },
        limits: limits.clone(),
        anchor_utc: chrono::Utc::now(),
    };
    let mut full_request = request.clone();
    full_request.context = ContextCapture::Full;
    full_request.limits.max_files = 10_000.try_into().unwrap();
    // Act: exercise both modes through the public capture API, not a duplicate candidate algorithm.
    let changed = capture_change_selection(request, CancellationToken::new())
        .await
        .unwrap();
    let full = capture_change_selection(full_request, CancellationToken::new())
        .await
        .unwrap();
    // Assert: all raw changed payloads/spans match; source input, HEAD and index stay untouched.
    assert!(matches!(
        changed.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert!(matches!(
        full.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(signature(&changed), signature(&full));
    assert_eq!(changed_paths(&changed), changed_paths(&full));
    assert!(changed.acquired_file_count() < full.acquired_file_count());
    assert_eq!(fingerprint(&root), original);
    assert_eq!(git_guard(&root), original_git);
    if let Some(receipt) = receipt {
        let parent = receipt.parent().unwrap().canonicalize().unwrap();
        assert!(
            !parent.starts_with(&root),
            "receipt must be outside the immutable input"
        );
        let raw = serde_json::to_vec(&signature(&changed)).unwrap();
        let raw_changes = changed
            .members
            .iter()
            .map(|member| {
                member.changes.iter().map(|change| serde_json::json!({
            "path":change.path,
            "before_sha256":side_text(&change.before).map(|text|content_digest(text.as_bytes())),
            "after_sha256":side_text(&change.after).map(|text|content_digest(text.as_bytes())),
            "before_spans":change.before_spans,"after_spans":change.after_spans
        })).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let account = serde_json::json!({
            "version":env!("CARGO_PKG_VERSION"),"root":root,"base_revision":base,"git_guard":original_git,
            "input_fingerprints":original,"limits":limits,"oracle_max_files":10_000,
            "changed_only":{"acquired_files":changed.acquired_file_count(),"paths":changed_paths(&changed)},
            "full":{"acquired_files":full.acquired_file_count(),"paths":changed_paths(&full)},
            "raw_changes":raw_changes,"raw_changes_equal":true,"raw_change_signature_sha256":content_digest(&raw),"input_unchanged":true
        });
        std::fs::write(receipt, serde_json::to_vec_pretty(&account).unwrap()).unwrap();
    }
    Ok(())
}
