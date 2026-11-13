#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use sunstack_decisions::CancellationToken;
use sunstack_lint::*;
use sunstack_rules::test_support::controlled_http as controlled;

fn copy_fixture(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
fn selection_fixture(root: &Path, endpoint: &str) -> LintRequest {
    copy_fixture(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../sunstack/tests/fixtures/rulebook-lint/selection"),
        root,
    );
    let path = root.join("skill-rules.config.json");
    let mut config: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    config["check"]["files"] = json!(["skills/practices-research/SKILL.md"]);
    config["evidence"]["kinds"]
        .as_object_mut()
        .unwrap()
        .retain(|name, _| name == "workflow_section" || name == "step");
    config["decision_model"]["endpoint"] = json!(endpoint);
    std::fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    LintRequest {
        config_path: Some(path),
        root: None,
        paths: vec![],
        scope: ScopeChoice::All,
        max_warnings: None,
        evidence_listing: None,
    }
}
fn assert_failure_formats(
    report: &LintReport,
    expected: Value,
    text: &str,
    expected_candidates: usize,
) {
    assert_eq!(report.status, LintStatus::Incomplete);
    assert_eq!(report.status.exit_code(), 3);
    let before = serde_json::to_value(report).unwrap();
    let candidates = before["selections"].as_array().unwrap();
    assert_eq!(candidates.len(), expected_candidates);
    let default = render(report, OutputFormat::Default);
    let sarif: Value = serde_json::from_str(&render(report, OutputFormat::Sarif)).unwrap();
    let notifications = sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .unwrap();
    for (entry, candidate) in report.selections.iter().zip(candidates) {
        assert_eq!(
            candidate["outcome"],
            json!({"kind":"incomplete","reason":expected})
        );
        let visible = format!(
            "{}:{} [{}]",
            entry.location.file.display(),
            entry.location.first_line,
            entry.kind.as_str()
        );
        assert!(
            default
                .lines()
                .any(|line| line.starts_with(&visible) && line.contains(text)),
            "candidate reason absent from default output:\n{default}"
        );
        assert!(
            notifications
                .iter()
                .any(|notice| notice["message"]["text"]
                    .as_str()
                    .is_some_and(|message| message
                        .lines()
                        .any(|line| line.starts_with(&visible) && line.contains(text)))),
            "candidate reason absent from SARIF: {sarif}"
        );
    }
    assert_eq!(
        serde_json::from_str::<Value>(&render(report, OutputFormat::Json)).unwrap(),
        before,
        "Formatting changed JSON payload"
    );
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        before,
        "Formatting mutated the report"
    );
}

#[tokio::test]
async fn real_selection_503_default_and_sarif_preserve_the_candidate_reason() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let request = selection_fixture(root.path(), &server.endpoint);
    // Keep both selected kinds for this case: every candidate must carry its own detail.
    let config_path = root.path().join("skill-rules.config.json");
    let mut config: Value = serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
    let original: Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../sunstack/tests/fixtures/rulebook-lint/selection/skill-rules.config.json"),
        )
        .unwrap(),
    )
    .unwrap();
    config["evidence"]["kinds"]["selected_targets"] =
        original["evidence"]["kinds"]["selected_targets"].clone();
    // Its target fields reference z_skill, so use the original selected kinds' supporting definitions too.
    for name in ["z_skill", "plain_heading"] {
        config["evidence"]["kinds"][name] = original["evidence"]["kinds"][name].clone();
    }
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_SELECTION_KEY".into()),
    ));
    for _ in 0..6 {
        server
            .next()
            .await
            .respond(503, json!({"error":"controlled unavailable"}));
    }
    let report = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 6);
    assert_failure_formats(
        &report,
        json!({"kind":"provider_failure","failure_class":"provider_unavailable","http_status":503}),
        "provider failure (provider_unavailable, HTTP 503)",
        2,
    );
}

async fn connection_report(root: &Path) -> LintReport {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/systemone", listener.local_addr().unwrap());
    drop(listener);
    let request = selection_fixture(root, &endpoint);
    tokio::time::timeout(
        Duration::from_secs(5),
        run(
            request,
            CancellationToken::new(),
            Some("SYNTHETIC_SELECTION_KEY".into()),
        ),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn real_selection_connection_default_and_sarif_preserve_the_candidate_reason() {
    let root = tempfile::tempdir().unwrap();
    let report = connection_report(root.path()).await;
    assert_failure_formats(
        &report,
        json!({"kind":"provider_failure","failure_class":"connection"}),
        "provider failure (connection)",
        1,
    );
    let output = render(&report, OutputFormat::Default);
    assert!(
        !output.contains("HTTP"),
        "Connection fabricated HTTP status: {output}"
    );
}
#[tokio::test]
async fn real_selection_invalid_response_default_and_sarif_preserve_the_candidate_reason() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let request = selection_fixture(root.path(), &server.endpoint);
    let operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_SELECTION_KEY".into()),
    ));
    server.next().await.respond(200,json!({"model":"typesafe/controlled","answers":{},"usage":{"input_tokens":1,"output_tokens":1,"cost":0.001}}));
    let report = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 1);
    assert_failure_formats(
        &report,
        json!({"kind":"invalid_response"}),
        "invalid response",
        1,
    );
    assert!(!render(&report, OutputFormat::Default).contains("provider failure"));
}
#[tokio::test]
async fn sdk_invalid_request_reaches_the_existing_lint_formatter_boundary() {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };
    use sunstack_decisions::{BackendSelection, JevOpenRouterBackend, ModelName};
    use sunstack_evidence::*;
    use sunstack_rules::*;
    let root = tempfile::tempdir().unwrap();
    // Normal lint admission makes this SDK-invalid state unreachable. The real
    // incomplete lint report supplies formatting context, not aggregation proof.
    let mut report = connection_report(root.path()).await;
    let config: Value = serde_json::from_slice(
        &std::fs::read(root.path().join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let mut kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> =
        serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    kinds.retain(|kind, _| kind.as_str() == "workflow_section");
    let mut selection = admit_selections(&kinds).unwrap().remove(0);
    selection.question = serde_json::from_value(
        json!({"type":"choice","instructions":"Evaluate the candidate.","criteria":{}}),
    )
    .unwrap();
    let project = load_evidence_project(
        &root.path().join("evidence/sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let file = PathBuf::from("skills/practices-research/SKILL.md");
    let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
    let mut build = EvidenceBuild::start(
        &project,
        &kinds,
        std::slice::from_ref(&file),
        &mut source,
        PieceBuildLimits {
            max_files: 100,
            max_file_bytes: 100000,
            max_total_bytes: 1000000,
            max_matches: 10000,
            evidence_max_chars: 6000,
        },
    )
    .unwrap();
    let BuildStep::NeedsSelection {
        candidates, shared, ..
    } = build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!("Fixture did not reach selection")
    };
    let input = SelectionInput {
        selection,
        candidates,
        shared,
        files: BTreeSet::from([file]),
    };
    let server = controlled::ControlledServer::start().await;
    let limits = EffectiveLimits::default();
    let settings = RulebookRunSettings {
        backend: BackendSelection::JevOpenrouter {
            endpoint: server.endpoint.clone(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        },
        attempts: limits.attempts().unwrap(),
        request_budget: limits.request_budget().unwrap(),
        request_max_chars: 20000,
        max_concurrent_requests: 1,
        max_model_requests: 200,
        max_locate_lines: 80,
    };
    let backend = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_SELECTION_KEY").unwrap(),
    );
    let mut dispatcher =
        ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());
    let outcomes = tokio::time::timeout(
        Duration::from_secs(3),
        select_candidates(&input, &mut dispatcher),
    )
    .await
    .unwrap();
    assert_eq!(server.finish().await, 0);
    assert!(dispatcher.records.is_empty());
    assert_eq!(outcomes.len(), 1);
    report.selections = input
        .candidates
        .into_iter()
        .map(|candidate| SelectionEntry {
            kind: candidate.id.kind.clone(),
            location: candidate.location,
            first_line: candidate.first_line,
            outcome: outcomes[&candidate.id].clone(),
        })
        .collect();
    assert_failure_formats(
        &report,
        json!({"kind":"invalid_request"}),
        "invalid request",
        1,
    );
    assert!(!render(&report, OutputFormat::Default).contains("provider failure"));
}

/// Signals a notifier whenever the run it is polling for is woken.
struct NotifyOnWake(tokio::sync::Notify);
impl std::task::Wake for NotifyOnWake {
    fn wake(self: std::sync::Arc<Self>) {
        self.0.notify_one();
    }
}
fn poll_with<F: std::future::Future + ?Sized>(
    run: &mut std::pin::Pin<Box<F>>,
    wake: &std::sync::Arc<NotifyOnWake>,
) -> std::task::Poll<F::Output> {
    let waker = std::task::Waker::from(wake.clone());
    run.as_mut()
        .poll(&mut std::task::Context::from_waker(&waker))
}

/// A run whose evidence stage cannot join after a real selection answer settled: the
/// CLI keeps its old incomplete report, while the partial report keeps the settled
/// selection request and its usage.
#[test]
fn a_stage_stopped_after_a_settled_selection_keeps_its_request_and_usage() {
    use std::sync::Arc;
    use sunstack_lint::configuration_loading::definition_bundle::acquire_file_definitions;
    let root = tempfile::tempdir().unwrap();
    // The answering server lives on its own runtime, so it outlasts the run's runtime.
    let server_runtime = tokio::runtime::Runtime::new().unwrap();
    let server = server_runtime.block_on(controlled::ControlledServer::start());
    let request = selection_fixture(root.path(), &server.endpoint);
    let location = discover_config(root.path(), request.config_path.as_deref()).unwrap();
    let bundle = acquire_file_definitions(&load_config(&location).unwrap(), &location);
    let (arrived, first_request) = tokio::sync::oneshot::channel();
    let receiving = server_runtime.spawn(async move {
        let mut server = server;
        let _ = arrived.send(server.next().await);
        server
    });

    // The run's runtime: dispatched requests and blocking stages run only while it is driven.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let pool = runtime.handle().clone();
    let mut run = Box::pin(run_definitions(
        bundle,
        root.path().to_path_buf(),
        vec![],
        ScopeChoice::All,
        CancellationToken::new(),
        Some("SYNTHETIC_SELECTION_KEY".into()),
    ));
    #[allow(
        clippy::async_yields_async,
        reason = "the paused run is handed to another runtime, not awaited here"
    )]
    let run = runtime.block_on(async move {
        // Drive the run until its first selection request reaches the server.
        let woken = Arc::new(NotifyOnWake(tokio::sync::Notify::new()));
        let mut first_request = first_request;
        let pending = loop {
            assert!(poll_with(&mut run, &woken).is_pending());
            tokio::select! {
                biased;
                pending = &mut first_request => break pending.unwrap(),
                () = woken.0.notified() => {}
            }
        };
        // Register a fresh waker, answer, and wait only until the answer settles: the run
        // is woken by its finished request but not polled past it.
        let settled = Arc::new(NotifyOnWake(tokio::sync::Notify::new()));
        assert!(poll_with(&mut run, &settled).is_pending());
        pending.noul(0.9);
        settled.0.notified().await;
        run
    });
    // Shut the pool down: the next blocking stage the run spawns is cancelled.
    runtime.shutdown_background();
    let resumed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let outcome = resumed.block_on(async {
        let _original_pool = pool.enter();
        run.await
    });
    let server = server_runtime.block_on(receiving).unwrap();
    assert_eq!(server_runtime.block_on(server.finish()), 1);

    let Err(stopped) = outcome else {
        panic!("the stage after selection cannot join");
    };
    assert_eq!(stopped.stage(), RunStage::EvidenceStage);
    let partial = stopped.clone().into_partial_report();
    let cli = stopped.into_cli_report();
    for report in [&partial, &cli] {
        assert_eq!(report.status, LintStatus::Incomplete);
        assert!(matches!(
            report.incomplete.as_slice(),
            [IncompletePart {
                reason: IncompleteReason::PreparationFailed,
                ..
            }]
        ));
        assert_eq!(report.selections.len(), cli.selections.len());
        assert!(
            !report.selections.is_empty(),
            "the settled selection was recorded"
        );
        assert_eq!(
            report
                .diagnostics
                .last()
                .map(|diagnostic| diagnostic.message.as_str()),
            Some("evidence stage failed")
        );
    }
    // The settled request, its receipt and its reported usage stay in the partial report.
    assert_eq!(partial.usage.model_requests, 1);
    assert_eq!(partial.usage.input_tokens, Some(20));
    assert_eq!(partial.usage.output_tokens, Some(5));
    assert_eq!(partial.usage.cost_reported_requests, 1);
    assert!(partial.usage.checked_files_acquired >= 1);
    assert_settled_selection_request(&partial.requests);
    // The CLI's report is exactly the report as it stood: the partial report without the
    // settled requests and usage the run had not yet recorded.
    let mut as_it_stood = serde_json::to_value(&partial).unwrap();
    as_it_stood["requests"] = json!([]);
    as_it_stood["usage"] = serde_json::to_value(UsageSummary::default()).unwrap();
    assert_eq!(serde_json::to_value(&cli).unwrap(), as_it_stood);
}

/// Exactly one saved selection request, linked to its owner, candidates, models, input
/// digest and completed receipt.
fn assert_settled_selection_request(requests: &[SavedRequest]) {
    let [request] = requests else {
        panic!("one settled selection request: {requests:?}");
    };
    let RequestOwner::Selection {
        selection_kind,
        file,
    } = &request.owner
    else {
        panic!("a selection owner: {:?}", request.owner);
    };
    assert_eq!(selection_kind.as_str(), "workflow_section");
    assert_eq!(file, Path::new("skills/practices-research/SKILL.md"));
    assert_eq!(request.requested_model.as_str(), "jev-latest");
    assert_eq!(
        request.resolved_model.as_ref().map(|model| model.as_str()),
        Some("typesafe/controlled")
    );
    assert!(request.input_sha256.is_some());
    let evidence = request.selection_evidence.as_ref().unwrap();
    assert!(!evidence.candidates.is_empty());
    assert!(matches!(
        &request.outcome,
        SavedRequestOutcome::Run {
            outcome: sunstack_decisions::DecisionRunOutcome::Completed { .. }
        }
    ));
}
