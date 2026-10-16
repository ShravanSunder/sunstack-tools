#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-decisions/tests/support/mod.rs"]
mod decision_fixtures;
mod support;
use serde_json::json;
use std::{io::Write, path::Path, process::Output, time::Duration};
use support::*;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr},
};

async fn line(stderr: &mut BufReader<ChildStderr>, expected: &str, transcript: &mut Vec<u8>) {
    let mut received = String::new();
    tokio::time::timeout(Duration::from_secs(5), stderr.read_line(&mut received))
        .await
        .unwrap()
        .unwrap();
    transcript.extend_from_slice(received.as_bytes());
    assert_eq!(received, expected);
}
async fn ready(root: &Path) -> (Child, BufReader<ChildStderr>, Vec<u8>) {
    let mut process = child(root);
    let mut stderr = BufReader::new(process.stderr.take().unwrap());
    let mut transcript = vec![];
    loop {
        let mut received = String::new();
        tokio::time::timeout(Duration::from_secs(5), stderr.read_line(&mut received))
            .await
            .unwrap()
            .unwrap();
        transcript.extend_from_slice(received.as_bytes());
        assert!(transcript.len() <= 256);
        if received == "sunstack-rule: ready\n" {
            break;
        }
        // The repository's runner permits this warning and still execs the
        // binary. It is not the product's handler-registration witness.
        assert!(received.starts_with("cargo debug runner: sunstack-rule runs unsigned ("));
    }
    (process, stderr, transcript)
}
async fn signal(process: &Child, signal_name: &str) {
    assert!(
        tokio::process::Command::new("/bin/kill")
            .args([signal_name, &process.id().unwrap().to_string()])
            .status()
            .await
            .unwrap()
            .success()
    );
}
async fn finish(
    process: Child,
    mut stderr: BufReader<ChildStderr>,
    mut transcript: Vec<u8>,
) -> Output {
    let mut output = tokio::time::timeout(Duration::from_secs(5), process.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    stderr.read_to_end(&mut transcript).await.unwrap();
    output.stderr = transcript;
    output
}

#[tokio::test]
async fn both_real_signals_are_observed_before_operation_admission_without_dispatch() {
    let fixture = tempfile::tempdir().unwrap();
    for signal_name in ["-TERM", "-INT"] {
        for operation in ["prepare_evidence", "run_judgments"] {
            let observer = decision_fixtures::TestServer::start(vec![]).await;
            let (mut process, mut stderr, mut transcript) = ready(fixture.path()).await;
            signal(&process, signal_name).await;
            // A successful kill syscall is not observation of cancellation.
            line(
                &mut stderr,
                "sunstack-rule: cancellation observed\n",
                &mut transcript,
            )
            .await;
            let input = if operation == "prepare_evidence" {
                evidence_input(fixture.path())
            } else {
                serde_json::to_value(decision_fixtures::mixed_input(&observer.endpoint)).unwrap()
            };
            let bytes = serde_json::to_vec(
                &json!({"protocol_version":1,"operation":operation,"input":input}),
            )
            .unwrap();
            process
                .stdin
                .take()
                .unwrap()
                .write_all(&bytes)
                .await
                .unwrap();
            let output = finish(process, stderr, transcript).await;
            let connections = observer.finish().await;
            record_http(
                &format!("before-admission-{signal_name}-{operation}"),
                &[],
                connections,
            );
            assert_eq!(connections, 0);
            record_output(fixture.path(), &bytes, &output);
            let wire = response(&output, 4);
            let outcome = &wire["result"]["outcome"];
            assert_eq!(outcome["kind"], "cancelled");
            if operation == "prepare_evidence" {
                assert_eq!(outcome["progress"]["kind"], "before_plan_admission");
                assert!(outcome.get("selections").is_none());
            } else {
                assert_eq!(outcome["attempts"][0]["evidence"]["kind"], "not_dispatched");
            }
        }
    }
}

#[tokio::test]
async fn in_flight_sigterm_keeps_remote_uncertainty_and_starts_no_retry() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
    let mut input = decision_fixtures::mixed_input(&endpoint);
    input.attempts.retry_count = 3;
    let bytes = serde_json::to_vec(
        &json!({"protocol_version":1,"operation":"run_judgments","input":input}),
    )
    .unwrap();
    let (mut process, mut stderr, mut transcript) = ready(&workspace_root()).await;
    process
        .stdin
        .take()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let capture = tokio::time::timeout(
        Duration::from_secs(5),
        decision_fixtures::read_request(&mut socket),
    )
    .await
    .unwrap();
    assert!(!capture.body.is_empty());
    signal(&process, "-TERM").await;
    line(
        &mut stderr,
        "sunstack-rule: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let mut byte = [0];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    let output = finish(process, stderr, transcript).await;
    record_output(&workspace_root(), &bytes, &output);
    record_http("in-flight-cancellation", &capture.body, 1);
    record_observation(
        "in-flight-order",
        json!({"observed_sequence":["ready","full_request_received","signal_consumed","socket_eof","child_joined"],"stdout_before_socket_eof":"not_inferred"}),
    );
    let wire = response(&output, 4);
    assert_eq!(
        wire["result"]["outcome"]["attempts"],
        json!([{"ordinal":1,"evidence":{"kind":"remote_outcome_unknown","failure_class":"caller_aborted"}}])
    );
    // The process has exited, so all its dispatch opportunities are over.
    assert!(
        tokio::time::timeout(Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn evidence_worker_settles_before_terminal_stdout_after_sigterm() {
    let fixture = tempfile::tempdir().unwrap();
    let fifo = fixture.path().join("schema.fifo");
    assert!(
        tokio::process::Command::new("/usr/bin/mkfifo")
            .arg(&fifo)
            .status()
            .await
            .unwrap()
            .success()
    );
    let mut input = evidence_input(fixture.path());
    input["sources"] = json!({});
    input["repositories"] = json!([]);
    input["declarations"] = json!([]);
    input["payload_schemas"] = json!([{"name":"blocked","path":fifo}]);
    let bytes = serde_json::to_vec(
        &json!({"protocol_version":1,"operation":"prepare_evidence","input":input}),
    )
    .unwrap();
    let (mut process, mut stderr, mut transcript) = ready(fixture.path()).await;
    process
        .stdin
        .take()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
    // A nonblocking writer can open only once the real schema reader opens.
    let descriptor = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match rustix::fs::open(
                &fifo,
                rustix::fs::OFlags::WRONLY | rustix::fs::OFlags::NONBLOCK,
                rustix::fs::Mode::empty(),
            ) {
                Ok(descriptor) => break descriptor,
                Err(rustix::io::Errno::NXIO) => tokio::task::yield_now().await,
                Err(error) => panic!("FIFO rendezvous failed: {error}"),
            }
        }
    })
    .await
    .unwrap();
    let mut writer = std::fs::File::from(descriptor);
    signal(&process, "-TERM").await;
    line(
        &mut stderr,
        "sunstack-rule: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let mut stdout = process.stdout.take().unwrap();
    // This is a bounded negative observation after signal consumption, not a
    // sleep or an instantaneous empty-pipe snapshot. Source ownership and the
    // deliberate early-publication control corroborate the ordering.
    let premature = tokio::time::timeout(Duration::from_millis(100), stdout.read_u8()).await;
    assert!(
        premature.is_err(),
        "stdout was published while schema worker was held: {premature:?}"
    );
    writer.write_all(b"{}").unwrap();
    drop(writer);
    let mut output = finish(process, stderr, transcript).await;
    stdout.read_to_end(&mut output.stdout).await.unwrap();
    record_output(fixture.path(), &bytes, &output);
    record_observation(
        "evidence-worker-order",
        json!({"observed_sequence":["ready","schema_reader_open","signal_consumed","no_stdout_with_worker_held","schema_released","terminal_result_after_child_join"],"negative_observation_ms":100}),
    );
    let wire = response(&output, 4);
    assert_eq!(
        wire["result"]["outcome"]["progress"]["kind"],
        "plan_admitted"
    );
}
