#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Facade admission against the CLI's rules: shared shape refusals, rule refusals that
//! keep their field path, and an abort that lands while admission is running.
use serde_json::{Value, json};
use std::{future::Future, path::Path, task::Poll};
use sunstack::sdk_host::{
    self,
    sdk_contracts::{CheckOutcome, Diagnostic, LoadOptions, LoadOutcome},
};
use sunstack_decisions::CancellationToken;

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
