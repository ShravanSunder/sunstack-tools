//! A direct byte binding cannot turn unacquired source into an extra acquisition.
use super::*;

fn counted_source(
    root: &Path,
    reads: &Arc<Mutex<Vec<PathBuf>>>,
    resolves: &Arc<Mutex<Vec<PathBuf>>>,
) -> Box<dyn CheckedFileSource + Send> {
    Box::new(RecordingSource {
        inner: FilesystemCheckedSource::new(root).unwrap(),
        reads: reads.clone(),
        resolves: resolves.clone(),
        fail_path: Some("docs/two.md".into()),
    })
}

fn target_attempts(attempts: &Arc<Mutex<Vec<PathBuf>>>) -> usize {
    attempts
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.as_path() == Path::new("docs/two.md"))
        .count()
}

#[tokio::test]
async fn unacquired_byte_binding_never_retries_a_witnessed_failed_read_or_resolves_its_target() {
    // Arrange: the target is regular and readable, but the injected source fails its read.
    // Count attempts before failure, independently of successful acquisition statistics.
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    std::fs::write(
        root.path().join("docs/two.md"),
        "regular readable sentinel\n",
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join("docs/two.md")).unwrap(),
        "regular readable sentinel\n"
    );
    let reads = Arc::new(Mutex::new(Vec::new()));
    let resolves = Arc::new(Mutex::new(Vec::new()));

    // Sensitivity: real run_with_source acquisition reaches the deliberate failed read.
    let positive = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
        Some(counted_source(root.path(), &reads, &resolves)),
    )
    .await;
    assert_eq!(positive.status, LintStatus::Incomplete, "{positive:?}");
    assert_eq!(positive.usage.checked_files_acquired, 1);
    assert_eq!(
        target_attempts(&reads),
        1,
        "a failed acquisition must be observable"
    );
    assert!(
        target_attempts(&resolves) > 0,
        "the same target's resolve attempt must be observable"
    );
    assert!(
        positive
            .incomplete
            .iter()
            .any(|part| part.file.as_deref() == Some(Path::new("docs/two.md")))
    );
    let before_target = (target_attempts(&reads), target_attempts(&resolves));

    // Act: target remains on disk, but only one.md belongs to this invocation's scope.
    let mut request = fixture::request(root.path());
    request.paths = vec![root.path().join("docs/one.md")];
    let baseline = run_with_source(
        request.clone(),
        CancellationToken::new(),
        None,
        Some(counted_source(root.path(), &reads, &resolves)),
    )
    .await;
    assert_eq!(baseline.status, LintStatus::Findings, "{baseline:?}");
    assert_eq!(
        (target_attempts(&reads), target_attempts(&resolves)),
        before_target
    );
    let baseline_reads = reads.lock().unwrap().len();
    let baseline_resolves = resolves.lock().unwrap().len();
    request.case_selection = Some(serde_json::from_value(json!([
        {"id":"captured-peer","kind":"rule","rule":"description","file":"docs/one.md",
         "lines":[1,3],"text_sha256":sunstack_evidence::content_digest(b"---\ndescription: Wrong opening\n---\n")},
        {"id":"unacquired","kind":"rule","rule":"description","file":"docs/two.md",
         "lines":[1,1],"bytes":[0,1000000],"text_sha256":"0".repeat(64)}
    ])).unwrap());
    let selected = run_with_source(
        request,
        CancellationToken::new(),
        None,
        Some(counted_source(root.path(), &reads, &resolves)),
    )
    .await;

    // Assert: no forced bounds read, resolve, guessed stale or fabricated owner.
    assert_eq!(selected.status, LintStatus::Findings, "{selected:?}");
    assert_eq!(selected.usage.checked_files_acquired, 1);
    assert_eq!(
        (target_attempts(&reads), target_attempts(&resolves)),
        before_target
    );
    assert_eq!(
        &reads.lock().unwrap()[baseline_reads..],
        &[PathBuf::from("docs/one.md")]
    );
    assert!(
        resolves.lock().unwrap()[baseline_resolves..]
            .iter()
            .all(|path| path == Path::new("docs/one.md"))
    );
    assert_eq!(
        serde_json::to_value(&selected.results).unwrap(),
        serde_json::to_value(&baseline.results).unwrap()
    );
    let bindings = &selected.case_selection.as_ref().unwrap().bindings;
    assert_eq!(bindings.len(), 2);
    assert!(matches!(
        bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));
    assert_eq!(bindings[1].binding.id, "unacquired");
    assert!(matches!(&bindings[1].binding.subject,
        CaseSubjectBinding::Rule { file, bytes: Some([0, 1000000]), .. }
        if file == Path::new("docs/two.md")));
    assert!(matches!(bindings[1].outcome, CaseBindingStatus::Unmatched));
    assert_eq!(selected.usage.model_requests, 0);
    assert!(selected.requests.is_empty());
}
