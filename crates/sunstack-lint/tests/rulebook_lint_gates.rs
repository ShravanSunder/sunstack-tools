#![allow(clippy::unwrap_used)]
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::{CheckedFileSource, CheckedReadError, FilesystemCheckedSource};
use sunstack_lint::*;
struct RecordingSource {
    inner: FilesystemCheckedSource,
    reads: Arc<Mutex<Vec<PathBuf>>>,
}
impl CheckedFileSource for RecordingSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf> {
        self.inner.resolve_file(path)
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        self.reads.lock().unwrap().push(path.into());
        self.inner.read_file(path, max_bytes)
    }
    fn checked_files_acquired(&self) -> usize {
        self.inner.checked_files_acquired()
    }
}
fn source(root: &Path) -> (Box<dyn CheckedFileSource + Send>, Arc<Mutex<Vec<PathBuf>>>) {
    let reads = Arc::new(Mutex::new(vec![]));
    (
        Box::new(RecordingSource {
            inner: FilesystemCheckedSource::new(root).unwrap(),
            reads: reads.clone(),
        }),
        reads,
    )
}
#[tokio::test]
async fn missing_key_and_malformed_config_never_reach_the_checked_source_or_git_capture() {
    for malformed in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
        if malformed {
            std::fs::write(root.path().join("sunstack.lint.json"), "{ broken").unwrap();
        }
        let (source, reads) = source(root.path());
        let mut request = fixture::request(root.path());
        request.scope = ScopeChoice::Default;
        let report = run_with_source(request, CancellationToken::new(), None, Some(source)).await;
        assert_eq!(report.status, LintStatus::Invalid);
        assert_eq!(
            report.diagnostics[0].code,
            if malformed {
                LintInvalidCode::Syntax
            } else {
                LintInvalidCode::CredentialMissing
            }
        );
        assert!(reads.lock().unwrap().is_empty());
        assert_eq!(report.usage.checked_files_acquired, 0);
        assert!(!root.path().join(".git").exists());
    }
}
#[tokio::test]
async fn an_invalid_entry_skips_locally_without_changing_peer_findings_or_acquisition_order() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let (reader, baseline_reads) = source(root.path());
    let baseline = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
        Some(reader),
    )
    .await;
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K002"] = entries["K001"].clone();
    entries["K002"]["name"] = json!("bad");
    entries["K002"]["evidence"] = json!("undefined");
    fixture::entry_file(root.path(), &entries);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Description must begin with Use when. <!-- K001 -->\n\nBad standard. <!-- K002 -->\n",
    )
    .unwrap();
    let (reader, reads) = source(root.path());
    let partial = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
        Some(reader),
    )
    .await;
    assert_eq!(partial.status, LintStatus::Incomplete);
    assert_eq!(partial.status.exit_code(), 3);
    assert_eq!(partial.skipped_rules.len(), 1);
    assert_eq!(partial.skipped_rules[0].key, "K002");
    assert_eq!(
        serde_json::to_value(&partial.findings).unwrap(),
        serde_json::to_value(&baseline.findings).unwrap()
    );
    assert_eq!(
        *baseline_reads.lock().unwrap(),
        vec![PathBuf::from("docs/one.md")]
    );
    assert_eq!(*reads.lock().unwrap(), vec![PathBuf::from("docs/one.md")]);
    assert_eq!(partial.usage.checked_files_acquired, 1);
}
#[tokio::test]
async fn changed_file_uses_the_frozen_capture_and_has_one_logical_acquisition_without_a_fallback_read()
 {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    fixture::commit(root.path(), "baseline");
    std::fs::write(
        root.path().join("docs/one.md"),
        "---\ndescription: Wrong changed opening\n---\n",
    )
    .unwrap();
    let (reader, reads) = source(root.path());
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;
    assert_eq!(report.status, LintStatus::Findings, "{report:?}");
    assert_eq!(report.usage.checked_files_acquired, 1);
    assert!(
        reads.lock().unwrap().is_empty(),
        "captured after text was reopened through the fallback source"
    );
    assert_eq!(report.findings[0].file, Path::new("docs/one.md"));
    assert_eq!(report.findings[0].line, 2);
}

#[test]
fn contained_absolute_target_reuses_the_same_frozen_content_as_its_relative_path() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let (fallback, reads) = source(root.path());
    let mut reader = sunstack_evidence::FrozenCheckedSource::new(
        std::collections::BTreeMap::from([(
            PathBuf::from("docs/one.md"),
            "Frozen snapshot text".into(),
        )]),
        fallback,
    );
    assert_eq!(
        reader
            .read_file(&root.path().join("docs/one.md"), 1000)
            .unwrap(),
        "Frozen snapshot text"
    );
    assert_eq!(
        reader.read_file(Path::new("docs/one.md"), 1000).unwrap(),
        "Frozen snapshot text"
    );
    assert_eq!(reader.checked_files_acquired(), 1);
    assert!(reads.lock().unwrap().is_empty());
}
