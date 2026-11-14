#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! What a facade call leaves behind: the tasks it started, and how file and inline
//! matcher definitions agree on defaults, ignored wrapper keys and refusals.
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use sunstack::sdk_host::{
    self, CredentialSource,
    report_projection::RunOutcome,
    sdk_contracts::{CheckOutcome, LoadOptions, LoadOutcome, RunRequest, Scope},
};
use sunstack_decisions::CancellationToken;
use sunstack_rules::test_support::controlled_http::ControlledServer;

fn asking_configuration(base: &Path, endpoint: &str) -> Value {
    json!({
        "id": "settlement",
        "base_directory": base,
        "marker_prefix": "K",
        "check": {"files": ["docs/*.md"]},
        "decision_model": {"model": "jev-latest", "endpoint": endpoint},
        "evidence": {
            "kind": "inline",
            "utilities": {},
            "matchers": {"paragraph": {"language": "Markdown", "rule": {"kind": "paragraph"}}},
            "kinds": {"paragraph": {"rule": "paragraph", "fields": {"body": {"from": "remainder"}}}}
        },
        "rulebooks": [{
            "kind": "inline",
            "id": "settlement-rules",
            "markdown": "Each paragraph is clear. <!-- K001 -->\n",
            "entries": {"K001": {"name": "clear", "evidence": "paragraph", "severity": "error",
                "questions": {"clear": {"type": "noul", "instructions": {"check": "body", "task": "Is it clear?"}}},
                "decision": {"start": "clear", "steps": {"clear": {"ask": "clear", "yes_at_or_above": 0.8,
                    "no_at_or_below": 0.3, "yes": "pass", "no": {"finding": "Unclear.", "at": "body"},
                    "uncertain": "uncertain"}}}}}
        }]
    })
}

/// A held model request, served from its own runtime so the call's runtime counts only the call's tasks.
struct HeldBackend {
    endpoint: String,
    first_request: tokio::sync::oneshot::Receiver<()>,
    release: tokio::sync::oneshot::Sender<()>,
    thread: std::thread::JoinHandle<usize>,
}

fn held_backend() -> HeldBackend {
    let (endpoint_sender, endpoint) = std::sync::mpsc::channel();
    let (seen, first_request) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let mut server = ControlledServer::start().await;
            endpoint_sender.send(server.endpoint.clone()).unwrap();
            let held = server.next().await;
            seen.send(()).unwrap();
            let _ = released.await;
            drop(held);
            server.finish().await
        })
    });
    HeldBackend {
        endpoint: endpoint.recv().unwrap(),
        first_request,
        release,
        thread,
    }
}

// One scheduler thread: a task's completion, including its release from the runtime's
// task list, finishes before the task awaiting it is polled again. So the alive count
// read when the outcome arrives has no bookkeeping lag, and any work still live then
// (even briefly, such as a detached task that ends 500 ms later) is counted.
#[tokio::test(flavor = "current_thread")]
async fn a_cancelled_run_settles_every_task_it_started_before_its_owner_sees_the_outcome() {
    let metrics = tokio::runtime::Handle::current().metrics();
    let baseline = metrics.num_alive_tasks();
    let base = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(checked.path().join("docs")).unwrap();
    for name in ["one", "two", "three"] {
        std::fs::write(
            checked.path().join(format!("docs/{name}.md")),
            format!("# {name}\n\nA paragraph about {name}.\n"),
        )
        .unwrap();
    }
    let backend = held_backend();
    let configuration = sdk_host::decode_configuration(
        &asking_configuration(base.path(), &backend.endpoint).to_string(),
    )
    .unwrap();
    let cancellation = CancellationToken::new();
    // The adapter runs each facade call as one owned task; this is that boundary.
    let call = tokio::spawn(sdk_host::run_with_credential(
        configuration,
        RunRequest {
            root: checked.path().to_path_buf(),
            scope: Scope::All,
            paths: vec![],
        },
        cancellation.clone(),
        CredentialSource::Supplied(Some("synthetic-controlled-key".into())),
    ));
    tokio::time::timeout(Duration::from_secs(20), backend.first_request)
        .await
        .expect("the run reached the model boundary")
        .unwrap();
    assert!(
        metrics.num_alive_tasks() > baseline,
        "the call is live while its request is held"
    );

    cancellation.cancel();
    let outcome = tokio::time::timeout(Duration::from_secs(10), call)
        .await
        .expect("the cancelled call settles")
        .unwrap();
    assert!(
        matches!(outcome, RunOutcome::Cancelled { report: Some(_) }),
        "{outcome:?}"
    );
    // Read at outcome delivery, with no wait: nothing the call started is still alive.
    assert_eq!(
        metrics.num_alive_tasks(),
        baseline,
        "tasks outlived the cancelled call's outcome"
    );
    backend.release.send(()).unwrap();
    // At most one request per document: nothing was replayed after cancellation.
    let sent = backend.thread.join().unwrap();
    assert!(
        (1..=3).contains(&sent),
        "{sent} requests for three documents"
    );
}

fn write_matcher_project(directory: &Path, matcher_yaml: &str) {
    std::fs::create_dir_all(directory.join("evidence/rules")).unwrap();
    std::fs::write(
        directory.join("sunstack.lint.json"),
        json!({
            "rulebook": [{"markdown": "rulebook.md", "rules": "rules.json"}],
            "marker_prefix": "K",
            "check": {"files": ["docs/*.md"]},
            "evidence": {"ast_grep": "evidence/sgconfig.yml",
                "kinds": {"paragraph": {"rule": "paragraph", "fields": {"body": {"from": "remainder"}}}}},
            "decision_model": {"model": "jev-latest"}
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        directory.join("rulebook.md"),
        "Each paragraph has a body. <!-- K001 -->\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rules.json"),
        json!({"K001": {"name": "body", "evidence": "paragraph", "severity": "error", "decision": {"start": "has",
            "steps": {"has": {"mechanical_check": {"tool": "complete", "field": "body"}, "yes": "pass",
                "no": {"finding": "Empty."}}}}}})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        directory.join("evidence/sgconfig.yml"),
        "ruleDirs: [rules]\n",
    )
    .unwrap();
    std::fs::write(directory.join("evidence/rules/paragraph.yml"), matcher_yaml).unwrap();
}

async fn load_value(directory: &Path) -> LoadOutcome {
    sdk_host::load(LoadOptions {
        directory: directory.to_path_buf(),
        config_path: None,
    })
    .await
}

async fn check_value(value: &Value) -> CheckOutcome {
    match sdk_host::decode_configuration(&value.to_string()) {
        Ok(configuration) => sdk_host::check(configuration, CancellationToken::new()).await,
        Err(diagnostics) => CheckOutcome::Invalid { diagnostics },
    }
}

/// The matcher a loaded configuration carries, and the same definition authored inline.
fn inline_twin(loaded: &Value, matcher: Value) -> Value {
    let mut inline = loaded.clone();
    inline["evidence"] = json!({
        "kind": "inline",
        "utilities": {},
        "matchers": {"paragraph": matcher},
        "kinds": loaded["evidence"]["kinds"].clone(),
    });
    inline["id"] = json!("inline-twin");
    inline
}

#[tokio::test]
async fn file_and_inline_matchers_share_defaults_ignored_wrapper_keys_and_refusals() {
    let project = tempfile::tempdir().unwrap();
    // A wrapper-level key upstream does not know is ignored in both forms.
    write_matcher_project(
        project.path(),
        "id: paragraph\nlanguage: md\nrule: {kind: paragraph}\nunrelated: true\n",
    );
    let LoadOutcome::Loaded { value } = load_value(project.path()).await else {
        panic!("the file matcher with an ignored wrapper key loads");
    };
    let loaded = serde_json::to_value(&value.configuration).unwrap();
    let file_matcher = loaded["evidence"]["matchers"]["paragraph"]["value"].clone();
    // Upstream defaults are filled the same way for file and inline definitions.
    assert_eq!(file_matcher["severity"], "hint");
    assert_eq!(file_matcher["message"], "");
    assert_eq!(file_matcher["language"], "Markdown");
    assert!(file_matcher.get("unrelated").is_none());

    let inline = inline_twin(
        &loaded,
        json!({"language": "Markdown", "rule": {"kind": "paragraph"}, "unrelated": true}),
    );
    let CheckOutcome::Valid { sources, .. } = check_value(&inline).await else {
        panic!("the inline twin admits");
    };
    let inline_matcher = sources
        .iter()
        .find(|source| {
            serde_json::to_value(&source.origin).unwrap()["id"]
                .as_str()
                .is_some_and(|id| id.ends_with("/evidence/matchers/paragraph"))
        })
        .unwrap();
    let mut inline_value: Value = serde_json::from_str(&inline_matcher.content).unwrap();
    let mut file_value = file_matcher.clone();
    // Only the file's declared id differs; the inline id is its key.
    file_value.as_object_mut().unwrap().remove("id");
    inline_value.as_object_mut().unwrap().remove("id");
    assert_eq!(inline_value, file_value);

    // A nested unknown rule key is refused in both forms.
    write_matcher_project(
        project.path(),
        "id: paragraph\nlanguage: md\nrule: {kindz: paragraph}\n",
    );
    assert!(
        matches!(
            load_value(project.path()).await,
            LoadOutcome::Invalid { .. }
        ),
        "file nested unknown key"
    );
    let nested = inline_twin(
        &loaded,
        json!({"language": "Markdown", "rule": {"kindz": "paragraph"}}),
    );
    assert!(matches!(
        check_value(&nested).await,
        CheckOutcome::Invalid { .. }
    ));

    // An unsupported language is refused in both forms.
    write_matcher_project(
        project.path(),
        "id: paragraph\nlanguage: cobol\nrule: {kind: paragraph}\n",
    );
    assert!(matches!(
        load_value(project.path()).await,
        LoadOutcome::Invalid { .. }
    ));
    let unsupported = inline_twin(
        &loaded,
        json!({"language": "Cobol", "rule": {"kind": "paragraph"}}),
    );
    assert!(matches!(
        check_value(&unsupported).await,
        CheckOutcome::Invalid { .. }
    ));
}
