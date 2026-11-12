#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Facade admission against the CLI's rules: shared shape refusals, rule refusals that
//! keep their field path, and an abort that lands while check or run admission is running.
use serde_json::{Value, json};
use std::{future::Future, path::Path, task::Poll};
use sunstack::sdk_host::{
    self,
    sdk_contracts::{CheckOutcome, Diagnostic, LoadOptions, LoadOutcome},
};
use sunstack_decisions::CancellationToken;
use sunstack_lint::{LintReport, LintStatus, ScopeChoice};

fn mechanical_configuration(base: &Path) -> Value {
    json!({
        "id": "admission",
        "base_directory": base,
        "marker_prefix": "K",
        "check": {"files": ["docs/*.md"]},
        "decision_model": {"model": "jev-latest"},
        "evidence": {
            "kind": "inline",
            "utilities": {},
            "matchers": {"paragraph": {"language": "Markdown", "rule": {"kind": "paragraph"}}},
            "kinds": {"paragraph": {"rule": "paragraph", "fields": {"body": {"from": "remainder"}}}}
        },
        "rulebooks": [{
            "kind": "inline",
            "id": "admission-rules",
            "markdown": "Each paragraph has a body. <!-- K001 -->\n",
            "entries": {"K001": {"name": "body", "evidence": "paragraph", "severity": "error",
                "decision": {"start": "has", "steps": {"has": {
                    "mechanical_check": {"tool": "complete", "field": "body"},
                    "yes": "pass", "no": {"finding": "Empty."}}}}}}
        }]
    })
}

async fn check_value(value: &Value) -> CheckOutcome {
    match sdk_host::decode_configuration(&value.to_string()) {
        Ok(configuration) => sdk_host::check(configuration, CancellationToken::new()).await,
        Err(diagnostics) => CheckOutcome::Invalid { diagnostics },
    }
}

fn refusal(outcome: CheckOutcome) -> Diagnostic {
    match outcome {
        CheckOutcome::Invalid { mut diagnostics } => diagnostics.remove(0),
        other => panic!("expected a refusal: {other:?}"),
    }
}

/// The CLI's refusal for the same shape problem in a file configuration.
async fn cli_file_refusal(directory: &Path, config: Value) -> Diagnostic {
    std::fs::write(directory.join("sunstack.lint.json"), config.to_string()).unwrap();
    match sdk_host::load(LoadOptions {
        directory: directory.to_path_buf(),
        config_path: None,
    })
    .await
    {
        LoadOutcome::Invalid { mut diagnostics } => diagnostics.remove(0),
        other => panic!("the file configuration is refused at decode: {other:?}"),
    }
}

#[tokio::test]
async fn inline_configurations_meet_the_cli_shape_checks() {
    let base = tempfile::tempdir().unwrap();
    let valid = mechanical_configuration(base.path());
    assert!(matches!(
        check_value(&valid).await,
        CheckOutcome::Valid { .. }
    ));
    let file_base = json!({
        "rulebook": [{"markdown": "rulebook.md", "rules": "rules.json"}],
        "marker_prefix": "K",
        "check": {"files": ["docs/*.md"]},
        "evidence": {"ast_grep": "sgconfig.yml",
            "kinds": {"paragraph": {"rule": "paragraph", "fields": {"body": {"from": "remainder"}}}}},
        "decision_model": {"model": "jev-latest"}
    });
    let cases: [(&str, Value, Value, &[&str]); 3] = [
        (
            "no rulebooks",
            json!({"rulebooks": []}),
            json!({"rulebook": []}),
            &["rulebooks"],
        ),
        (
            "no checked files",
            json!({"check": {"files": []}}),
            json!({"check": {"files": []}}),
            &["check.files"],
        ),
        (
            "too many locate lines",
            json!({"limits": {"max_locate_lines": 300}}),
            json!({"limits": {"max_locate_lines": 300}}),
            &["limits.max_locate_lines"],
        ),
    ];
    for (name, inline_change, file_change, json_path) in cases {
        let mut inline = valid.clone();
        let mut file = file_base.clone();
        for (target, change) in [(&mut inline, &inline_change), (&mut file, &file_change)] {
            for (key, value) in change.as_object().unwrap() {
                target[key] = value.clone();
            }
        }
        let sdk = refusal(check_value(&inline).await);
        let cli = cli_file_refusal(base.path(), file).await;
        assert_eq!(sdk.json_path, json_path, "{name}");
        // Each surface names its own collection key (`rulebooks` here, `rulebook` in a CLI file).
        assert_eq!(
            sdk.message.replacen("rulebooks", "rulebook", 1),
            cli.message,
            "{name}: same refusal as the CLI"
        );
        assert_eq!(sdk.code, cli.code, "{name}");
    }
}

#[tokio::test]
async fn rule_refusals_keep_the_field_path_inside_their_entries() {
    let base = tempfile::tempdir().unwrap();
    // Rule files are decoded by the strict CLI decoder, which names the refused field.
    std::fs::write(
        base.path().join("rulebook.md"),
        "Each paragraph has a body. <!-- K001 -->\n",
    )
    .unwrap();
    let mut entries = mechanical_configuration(base.path())["rulebooks"][0]["entries"].clone();
    // `only_when` is a field name the strict decoder may disclose; the marker key is not.
    entries["K001"]["only_when"] = json!("not a gate");
    std::fs::write(base.path().join("rules.json"), entries.to_string()).unwrap();
    let mut file_rules = mechanical_configuration(base.path());
    file_rules["rulebooks"] =
        json!([{"kind": "files", "markdown": "rulebook.md", "entries": "rules.json"}]);
    let diagnostic = refusal(check_value(&file_rules).await);
    assert_eq!(
        serde_json::to_value(&diagnostic.origin).unwrap()["kind"],
        "file"
    );
    assert_eq!(diagnostic.json_path, ["only_when"], "{diagnostic:?}");
}

/// Holds the runtime's only blocking thread until released; released on drop as well.
struct BlockingPoolHold {
    release: Option<std::sync::mpsc::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl BlockingPoolHold {
    /// Occupy the single blocking thread and wait until the occupant is running.
    async fn occupy() -> Self {
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (running, started) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::task::spawn_blocking(move || {
            running.send(()).unwrap();
            let _ = released.recv();
        });
        started.await.unwrap();
        Self {
            release: Some(release),
            task: Some(task),
        }
    }

    async fn release(mut self) {
        drop(self.release.take());
        self.task.take().unwrap().await.unwrap();
    }
}

impl Drop for BlockingPoolHold {
    fn drop(&mut self) {
        // A failing assertion still frees the blocking thread so the runtime can shut down.
        drop(self.release.take());
    }
}

#[test]
fn an_abort_during_admission_ends_the_check_cancelled() {
    // One blocking thread, held: admission is queued behind it, so the call is causally
    // inside admission when the abort arrives.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let base = tempfile::tempdir().unwrap();
        let configuration =
            sdk_host::decode_configuration(&mechanical_configuration(base.path()).to_string())
                .unwrap();
        let hold = BlockingPoolHold::occupy().await;
        let cancellation = CancellationToken::new();
        let mut call = Box::pin(sdk_host::check(configuration, cancellation.clone()));
        let first = std::future::poll_fn(|context| Poll::Ready(call.as_mut().poll(context))).await;
        assert!(
            first.is_pending(),
            "admission waits for the held blocking thread"
        );
        cancellation.cancel();
        hold.release().await;
        let outcome = call.await;
        assert!(
            matches!(outcome, CheckOutcome::Cancelled { .. }),
            "{outcome:?}"
        );
    });
}

/// A runtime with one blocking thread, so a held thread queues admission behind it.
fn one_blocking_thread_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap()
}

/// Run captured definitions with run admission queued behind the held blocking thread,
/// aborting the call there when `abort` is set.
fn run_with_admission_held(configuration: &Value, root: &Path, abort: bool) -> LintReport {
    one_blocking_thread_runtime().block_on(async {
        let decoded = sdk_host::decode_configuration(&configuration.to_string()).unwrap();
        let Ok(bundle) = sdk_host::definition_capture::capture_configuration(&decoded) else {
            panic!("the definitions are captured before the run");
        };
        let hold = BlockingPoolHold::occupy().await;
        let cancellation = CancellationToken::new();
        let mut call = Box::pin(sunstack_lint::run_definitions(
            bundle,
            root.to_path_buf(),
            vec![],
            ScopeChoice::All,
            cancellation.clone(),
            None,
        ));
        let first = std::future::poll_fn(|context| Poll::Ready(call.as_mut().poll(context))).await;
        assert!(
            first.is_pending(),
            "run admission waits for the held blocking thread"
        );
        if abort {
            cancellation.cancel();
        }
        hold.release().await;
        call.await
    })
}

#[test]
fn an_abort_during_run_admission_ends_cancelled_before_any_checked_input() {
    let base = tempfile::tempdir().unwrap();
    let readable = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(readable.path().join("docs")).unwrap();
    std::fs::write(
        readable.path().join("docs/one.md"),
        "# One\n\nA paragraph.\n",
    )
    .unwrap();
    // A checked root no one may list: any target selection here ends path_not_found.
    let unlistable = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(unlistable.path().join("docs")).unwrap();
    let valid = mechanical_configuration(base.path());
    // An evidence kind naming no matcher refuses the whole admission.
    let mut refused = mechanical_configuration(base.path());
    refused["evidence"]["kinds"]["paragraph"]["rule"] = json!("missing_matcher");

    // Uncancelled controls: a valid admission executes, an invalid one is refused.
    let completed = run_with_admission_held(&valid, readable.path(), false);
    assert!(
        matches!(completed.status, LintStatus::Clean | LintStatus::Findings),
        "{:?}",
        completed.status
    );
    assert_eq!(completed.file_counts.len(), 1);
    let invalid = run_with_admission_held(&refused, readable.path(), false);
    assert!(
        matches!(invalid.status, LintStatus::Invalid),
        "{:?} {:?}",
        invalid.status,
        invalid.diagnostics
    );
    assert!(!invalid.diagnostics.is_empty());

    std::fs::set_permissions(
        unlistable.path(),
        std::os::unix::fs::PermissionsExt::from_mode(0o000),
    )
    .unwrap();
    let outcomes = [&valid, &refused]
        .map(|configuration| run_with_admission_held(configuration, unlistable.path(), true));
    std::fs::set_permissions(
        unlistable.path(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    for report in outcomes {
        // Cancelled wins over the admission result, and no checked input was touched.
        assert!(
            matches!(report.status, LintStatus::Cancelled),
            "{:?} {:?}",
            report.status,
            report.diagnostics
        );
        assert!(report.diagnostics.is_empty());
        assert!(report.file_counts.is_empty());
    }
}
