#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Run error paths through the real facade and executor: a backend refusal names its
//! definition, never an invented file, and a run stage whose native task cannot join
//! ends `failed` after dispatch rather than as a domain-incomplete run.
#[path = "../support/blocking_pool_hold.rs"]
mod blocking_pool_hold;

use blocking_pool_hold::{BlockingPoolHold, one_blocking_thread_runtime};
use serde_json::{Value, json};
use std::{future::Future, path::Path, pin::Pin, task::Poll};
use sunstack::sdk_host::{
    self, CredentialSource,
    report_projection::RunOutcome,
    sdk_contracts::{
        AfterDispatchReason, Configuration, LoadOptions, LoadOutcome, RunRequest, Scope,
        SdkFailure, UnknownEffects,
    },
};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::DefinitionOrigin;
use sunstack_lint::{IncompleteReason, LintReport, LintStatus, ScopeChoice};

/// An endpoint the backend refuses while parsing it, before any transport exists.
const UNPARSEABLE_ENDPOINT: &str = "not a url";
/// A literal stand-in: the backend refuses the endpoint before it would use a key.
const PLACEHOLDER_KEY: &str = "placeholder-never-sent";

fn asking_entries() -> Value {
    json!({"K001": {"name": "clear", "evidence": "paragraph", "severity": "error",
        "questions": {"clear": {"type": "noul", "instructions": {"check": "body", "task": "Is it clear?"}}},
        "decision": {"start": "clear", "steps": {"clear": {"ask": "clear",
            "yes_at_or_above": 0.8, "no_at_or_below": 0.3,
            "yes": "pass", "no": {"finding": "Unclear.", "at": "body"}, "uncertain": "uncertain"}}}}})
}

fn mechanical_entries() -> Value {
    json!({"K001": {"name": "body", "evidence": "paragraph", "severity": "error",
        "decision": {"start": "has", "steps": {"has": {
            "mechanical_check": {"tool": "complete", "field": "body"},
            "yes": "pass", "no": {"finding": "Empty."}}}}}})
}

fn inline_configuration(base: &Path, entries: Value) -> Value {
    json!({
        "id": "error-paths",
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
            "id": "error-path-rules",
            "markdown": "Each paragraph is clear. <!-- K001 -->\n",
            "entries": entries
        }]
    })
}

/// A CLI file project with one asking rule, whose decision model uses `endpoint`.
fn write_file_project(directory: &Path, endpoint: &str) {
    std::fs::create_dir_all(directory.join("rules")).unwrap();
    std::fs::write(directory.join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        directory.join("rules/paragraph.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rulebook.md"),
        "Each paragraph is clear. <!-- K001 -->\n",
    )
    .unwrap();
    std::fs::write(directory.join("rules.json"), asking_entries().to_string()).unwrap();
    std::fs::write(
        directory.join("sunstack.lint.json"),
        json!({
            "rulebook": [{"markdown": "rulebook.md", "rules": "rules.json"}],
            "marker_prefix": "K",
            "check": {"files": ["docs/*.md"]},
            "evidence": {"ast_grep": "sgconfig.yml",
                "kinds": {"paragraph": {"rule": "paragraph", "fields": {"body": {"from": "remainder"}}}}},
            "decision_model": {"model": "jev-latest", "endpoint": endpoint}
        })
        .to_string(),
    )
    .unwrap();
}

fn checked_tree() -> tempfile::TempDir {
    let checked = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(checked.path().join("docs")).unwrap();
    std::fs::write(
        checked.path().join("docs/one.md"),
        "# One\n\nA paragraph.\n",
    )
    .unwrap();
    std::fs::write(
        checked.path().join("docs/two.md"),
        "# Two\n\nAnother paragraph.\n",
    )
    .unwrap();
    checked
}

fn decoded(value: &Value) -> Configuration {
    sdk_host::decode_configuration(&value.to_string()).unwrap()
}

async fn loaded_value(directory: &Path) -> Value {
    match sdk_host::load(LoadOptions {
        directory: directory.to_path_buf(),
        config_path: None,
    })
    .await
    {
        LoadOutcome::Loaded { value } => serde_json::to_value(value.configuration).unwrap(),
        other => panic!("the file project loads: {other:?}"),
    }
}

fn all_files(root: &Path) -> RunRequest {
    RunRequest {
        root: root.to_path_buf(),
        scope: Scope::All,
        paths: vec![],
    }
}

/// Run captured definitions with a placeholder key; return the root origin and report.
async fn run_captured(configuration: &Value, root: &Path) -> (DefinitionOrigin, LintReport) {
    let Ok(bundle) = sdk_host::definition_capture::capture_configuration(&decoded(configuration))
    else {
        panic!("the definitions are captured before the run");
    };
    let origin = bundle.root.origin.clone();
    let report = sunstack_lint::run_definitions(
        bundle,
        root.to_path_buf(),
        vec![],
        ScopeChoice::All,
        CancellationToken::new(),
        Some(PLACEHOLDER_KEY.into()),
    )
    .await
    .expect("a refused backend is a domain refusal, not a stopped stage");
    (origin, report)
}

#[tokio::test]
async fn a_refused_backend_names_its_definition_and_never_an_invented_file() {
    let checked = checked_tree();
    let inline_base = tempfile::tempdir().unwrap();
    let mut inline = inline_configuration(inline_base.path(), asking_entries());
    inline["decision_model"]["endpoint"] = json!(UNPARSEABLE_ENDPOINT);
    let refused_project = tempfile::tempdir().unwrap();
    write_file_project(refused_project.path(), UNPARSEABLE_ENDPOINT);
    let file = loaded_value(refused_project.path()).await;
    let valid_project = tempfile::tempdir().unwrap();
    write_file_project(valid_project.path(), "https://example.invalid/api/v1");
    let mut edited = loaded_value(valid_project.path()).await;
    edited["decision_model"]["endpoint"] = json!(UNPARSEABLE_ENDPOINT);

    let cases = [
        ("inline", &inline),
        ("edited", &edited),
        ("unchanged file", &file),
    ];
    let mut observed = vec![];
    let mut expected = vec![];
    for (name, configuration) in cases {
        let (origin, report) = run_captured(configuration, checked.path()).await;
        assert!(
            matches!(report.status, LintStatus::Invalid),
            "{name}: {:?} {:?}",
            report.status,
            report.diagnostics
        );
        assert_eq!(
            report.diagnostics.len(),
            1,
            "{name}: {:?}",
            report.diagnostics
        );
        let diagnostic = &report.diagnostics[0];
        observed.push((
            name,
            serde_json::to_value(&origin).unwrap()["kind"].clone(),
            diagnostic.file.clone(),
            diagnostic.definition.as_ref() == Some(&origin),
        ));
        // The raw engine diagnostic carries the origin's own CLI path: the real file for a
        // file origin, and none (the empty path) for an in-memory one.
        expected.push((
            name,
            serde_json::to_value(&origin).unwrap()["kind"].clone(),
            Some(origin.cli_path()),
            true,
        ));
    }
    assert_eq!(observed, expected);
    let kinds = expected
        .iter()
        .map(|case| case.1.clone())
        .collect::<Vec<_>>();
    assert_eq!(kinds, ["inline", "edited", "file"]);

    // The SDK reports the definition origin and no checked path for the same refusal.
    let outcome = sdk_host::run_with_credential(
        decoded(&inline),
        all_files(checked.path()),
        CancellationToken::new(),
        CredentialSource::Supplied(Some(PLACEHOLDER_KEY.into())),
    )
    .await;
    let RunOutcome::Invalid { diagnostics, .. } = outcome else {
        panic!("the refused backend makes the run invalid: {outcome:?}");
    };
    assert_eq!(
        diagnostics[0].origin,
        Some(DefinitionOrigin::Inline {
            id: "error-paths".into(),
            label: "error-paths".into()
        })
    );
    assert_eq!(diagnostics[0].path, None);
}

/// Where the runtime's blocking pool shuts down, relative to the run's blocking stages.
#[derive(Clone, Copy, Debug)]
enum PoolShutdown {
    /// After definition capture returns, before run admission is spawned.
    BeforeAdmission,
    /// After run admission returns, before target selection is spawned.
    BeforeTargetSelection,
}

fn poll_once<'call, F: Future + ?Sized>(
    call: &'call mut Pin<Box<F>>,
) -> impl Future<Output = bool> + 'call {
    std::future::poll_fn(move |context| Poll::Ready(call.as_mut().poll(context).is_pending()))
}

/// Drive a keyless run with one blocking thread, holding that thread so the run is
/// paused exactly before the chosen stage is spawned. With `shut_down`, the runtime then
/// shuts down and the run resumes elsewhere: Tokio cancels a blocking task spawned on a
/// pool that is shutting down, so the next stage's task cannot join. Without it, the run
/// resumes on its own runtime as the control.
fn run_with_paused_pool(
    configuration: &Value,
    root: &Path,
    pause: PoolShutdown,
    shut_down: bool,
) -> RunOutcome {
    let runtime = one_blocking_thread_runtime();
    let pool = runtime.handle().clone();
    let configuration = decoded(configuration);
    let (call, hold) = runtime.block_on(async {
        // Hold the only blocking thread first: capture queues behind it, so the first
        // poll stops at capture whatever its speed.
        let before_capture = BlockingPoolHold::occupy().await;
        let mut call = Box::pin(sdk_host::run_with_credential(
            configuration,
            all_files(root),
            CancellationToken::new(),
            CredentialSource::Supplied(None),
        ));
        assert!(
            poll_once(&mut call).await,
            "capture is queued behind the hold"
        );
        // Queued behind capture: once it runs, capture has returned.
        let after_capture = BlockingPoolHold::queue();
        before_capture.release().await;
        let after_capture = after_capture.started().await;
        if matches!(pause, PoolShutdown::BeforeAdmission) {
            return (call, after_capture);
        }
        assert!(
            poll_once(&mut call).await,
            "admission is queued behind the hold"
        );
        // Queued behind admission: once it runs, admission has returned.
        let after_admission = BlockingPoolHold::queue();
        after_capture.release().await;
        (call, after_admission.started().await)
    });
    if !shut_down {
        return runtime.block_on(async {
            hold.release().await;
            call.await
        });
    }
    runtime.shutdown_background();
    let resumed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    resumed.block_on(async {
        // Stages spawn their blocking work on the original, shut-down pool.
        let _original_pool = pool.enter();
        let outcome = call.await;
        hold.release().await;
        outcome
    })
}

fn assert_failed_after_dispatch(outcome: &RunOutcome) {
    assert!(
        matches!(
            outcome,
            RunOutcome::Failed {
                failure: SdkFailure::AfterDispatch {
                    effects: UnknownEffects::Unknown,
                    reason: AfterDispatchReason::NativeCallFailed,
                    ..
                },
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// A keyless inline run over two checked files, with its control resumed on a live
/// runtime at the same pause.
fn paused_run(pause: PoolShutdown) -> RunOutcome {
    let base = tempfile::tempdir().unwrap();
    let checked = checked_tree();
    let configuration = inline_configuration(base.path(), mechanical_entries());
    let control = run_with_paused_pool(&configuration, checked.path(), pause, false);
    assert!(
        matches!(control, RunOutcome::Completed { .. }),
        "{pause:?} control: {control:?}"
    );
    run_with_paused_pool(&configuration, checked.path(), pause, true)
}

#[test]
fn an_admission_task_that_cannot_join_ends_failed_without_a_report() {
    let outcome = paused_run(PoolShutdown::BeforeAdmission);
    assert_failed_after_dispatch(&outcome);
    // Admission never ran, so there is no report.
    assert!(
        matches!(outcome, RunOutcome::Failed { report: None, .. }),
        "{outcome:?}"
    );
}

#[test]
fn an_execution_stage_that_cannot_join_ends_failed_with_its_partial_report() {
    let outcome = paused_run(PoolShutdown::BeforeTargetSelection);
    assert_failed_after_dispatch(&outcome);
    let RunOutcome::Failed {
        report: Some(report),
        ..
    } = outcome
    else {
        panic!("a stage after admission keeps its partial report: {outcome:?}");
    };
    // The report as it stood, marked incomplete for the stage that stopped.
    assert!(
        matches!(report.status, LintStatus::Incomplete),
        "{report:?}"
    );
    assert_eq!(report.incomplete.len(), 1);
    assert!(matches!(
        report.incomplete[0].reason,
        IncompleteReason::PreparationFailed
    ));
    assert!(
        report.file_counts.is_empty(),
        "no checked input was selected"
    );
    assert_eq!(
        report.configuration,
        DefinitionOrigin::Inline {
            id: "error-paths".into(),
            label: "error-paths".into()
        }
    );
}

#[tokio::test]
async fn a_domain_incomplete_run_stays_incomplete() {
    let base = tempfile::tempdir().unwrap();
    let checked = checked_tree();
    let mut configuration = inline_configuration(base.path(), mechanical_entries());
    // Two checked files against a one-file limit: the run is incomplete, not failed.
    configuration["limits"] = json!({"max_files": 1});
    let outcome = sdk_host::run_with_credential(
        decoded(&configuration),
        all_files(checked.path()),
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await;
    let RunOutcome::Incomplete { report } = outcome else {
        panic!("the file limit leaves the run incomplete: {outcome:?}");
    };
    assert!(
        report
            .incomplete
            .iter()
            .any(|part| matches!(part.reason, IncompleteReason::SelectionFailed)),
        "{report:?}"
    );
}
