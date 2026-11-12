use super::*;
use std::process::Command;

fn git(fixture: &GitFixture, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(fixture.directory.path())
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture_with_stable_file() -> GitFixture {
    let fixture = GitFixture::new();
    fixture.write("stable.md", "stable\n");
    fixture.stage("stable.md");
    fixture
}

async fn assert_raw_change(fixture: &GitFixture, base: git2::Oid, before: &str, after: &str) {
    let changed = capture_change_selection(
        request(fixture, base, ContextCapture::ChangedOnly),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let full = capture_change_selection(
        request(fixture, base, ContextCapture::Full),
        CancellationToken::new(),
    )
    .await
    .unwrap();

    assert!(matches!(
        changed.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert_eq!(changed_paths(&changed), vec![PathBuf::from("changed.md")]);
    let change = &changed.members[0].changes[0];
    assert_eq!(side_text(&change.before), Some(before));
    assert_eq!(side_text(&change.after), Some(after));
    assert_eq!(signature(&changed), signature(&full));
    assert_eq!(changed.acquired_file_count(), 2);
    assert!(changed.sources.iter().all(|file| matches!(
        &file.source,
        SourceRef::RepositoryFile { path, .. } if path == Path::new("changed.md")
    )));
}

#[tokio::test]
async fn detects_an_edited_assume_unchanged_entry() {
    let fixture = fixture_with_stable_file();
    fixture.write("changed.md", "before\n");
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    git(
        &fixture,
        &["update-index", "--assume-unchanged", "changed.md"],
    );
    fixture.write("changed.md", "after\n");

    assert_raw_change(&fixture, base, "before\n", "after\n").await;
}

#[tokio::test]
async fn detects_an_edited_skip_worktree_entry() {
    let fixture = fixture_with_stable_file();
    fixture.write("changed.md", "before\n");
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    git(&fixture, &["update-index", "--skip-worktree", "changed.md"]);
    fixture.write("changed.md", "after\n");

    assert_raw_change(&fixture, base, "before\n", "after\n").await;
}

#[tokio::test]
async fn detects_crlf_only_change_with_autocrlf_and_text_auto() {
    let fixture = fixture_with_stable_file();
    git(&fixture, &["config", "core.autocrlf", "true"]);
    fixture.write(".gitattributes", "*.md text=auto\n");
    fixture.stage(".gitattributes");
    fixture.write("changed.md", "line\n");
    git(&fixture, &["add", "changed.md"]);
    // The fixture's long-lived libgit2 handle must observe the CLI's index write.
    fixture.repository.index().unwrap().read(true).unwrap();
    let base = fixture.commit(1000);
    fixture.write("changed.md", "line\r\n");
    assert_eq!(
        git(
            &fixture,
            &["hash-object", "--path=changed.md", "changed.md"]
        ),
        git(&fixture, &["rev-parse", "HEAD:changed.md"])
    );

    assert_raw_change(&fixture, base, "line\n", "line\r\n").await;
}

#[tokio::test]
async fn detects_crlf_only_change_with_explicit_lf_attribute() {
    let fixture = fixture_with_stable_file();
    fixture.write(".gitattributes", "*.md text eol=lf\n");
    fixture.stage(".gitattributes");
    fixture.write("changed.md", "line\n");
    git(&fixture, &["add", "changed.md"]);
    // The fixture's long-lived libgit2 handle must observe the CLI's index write.
    fixture.repository.index().unwrap().read(true).unwrap();
    let base = fixture.commit(1000);
    fixture.write("changed.md", "line\r\n");
    assert_eq!(
        git(
            &fixture,
            &["hash-object", "--path=changed.md", "changed.md"]
        ),
        git(&fixture, &["rev-parse", "HEAD:changed.md"])
    );

    assert_raw_change(&fixture, base, "line\n", "line\r\n").await;
}

#[tokio::test]
async fn detects_raw_change_hidden_by_clean_filter() {
    let fixture = fixture_with_stable_file();
    git(
        &fixture,
        &["config", "filter.lowercase.clean", "tr A-Z a-z"],
    );
    git(&fixture, &["config", "filter.lowercase.smudge", "cat"]);
    fixture.write(".gitattributes", "changed.md filter=lowercase\n");
    fixture.stage(".gitattributes");
    fixture.write("changed.md", "lowercase\n");
    git(&fixture, &["add", "changed.md"]);
    // The fixture's long-lived libgit2 handle must observe the CLI's index write.
    fixture.repository.index().unwrap().read(true).unwrap();
    let base = fixture.commit(1000);
    fixture.write("changed.md", "LOWERCASE\n");
    assert_eq!(
        git(
            &fixture,
            &["hash-object", "--path=changed.md", "changed.md"]
        ),
        git(&fixture, &["rev-parse", "HEAD:changed.md"])
    );

    assert_raw_change(&fixture, base, "lowercase\n", "LOWERCASE\n").await;
}

#[tokio::test]
async fn detects_same_length_edit_with_restored_mtime() {
    let fixture = fixture_with_stable_file();
    git(&fixture, &["config", "core.trustctime", "false"]);
    fixture.write("changed.md", "before\n");
    let path = fixture.directory.path().join("changed.md");
    let old_mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(old_mtime))
        .unwrap();
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    fixture.write("changed.md", "after!\n");
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(old_mtime))
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        old_mtime
    );

    assert_raw_change(&fixture, base, "before\n", "after!\n").await;
}

#[cfg(unix)]
#[tokio::test]
async fn equal_file_and_symlink_payloads_remain_unchanged() {
    let fixture = fixture_with_stable_file();
    fixture.write("changed.md", "target.md");
    fixture.stage("changed.md");
    let base = fixture.commit(1000);
    let path = fixture.directory.path().join("changed.md");
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("target.md", &path).unwrap();

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
    assert!(matches!(
        changed.coverage,
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
    ));
    assert!(changed_paths(&changed).is_empty());
    assert_eq!(signature(&changed), signature(&full));
    assert_eq!(changed.acquired_file_count(), 0);
    assert!(changed.sources.is_empty());
}
