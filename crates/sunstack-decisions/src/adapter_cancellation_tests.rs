#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::*;
use std::{sync::Arc, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpListener, sync::Notify};
#[path = "../tests/support/mod.rs"]
mod support;
use support::*;

#[tokio::test]
async fn real_adapter_pre_dispatch_abort_has_no_network_call() {
    let server = TestServer::start(Vec::new()).await;
    let input = mixed_input(&server.endpoint);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = run_judgments(&input, &backend(&server.endpoint), &cancellation).await;
    match result {
        DecisionRunOutcome::Cancelled { attempts, .. } => assert_eq!(
            attempts,
            vec![InferenceAttemptRecord {
                ordinal: 1,
                evidence: AttemptEvidence::NotDispatched {
                    failure_class: AttemptFailureClass::CallerAborted
                }
            }]
        ),
        _ => panic!("pre-dispatch abort lost"),
    }
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn real_adapter_received_body_barrier_then_cancel_prevents_completed_admission() {
    let server = TestServer::start(vec![HttpReply::json(response())]).await;
    let received = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    let mut backend = backend(&server.endpoint);
    backend.test_observation = Some(AdapterTestObservation::BodyReceived {
        received: received.clone(),
        resume,
    });
    let mut input = mixed_input(&server.endpoint);
    input.attempts.retry_count = 2;
    let cancellation = CancellationToken::new();
    let task_cancel = cancellation.clone();
    let task = tokio::spawn(async move { run_judgments(&input, &backend, &task_cancel).await });
    tokio::time::timeout(Duration::from_secs(2), received.notified())
        .await
        .unwrap();
    // This signal is emitted by the real adapter after its bounded body loop,
    // before the owned IO future returns to the controller's response admission.
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    match result {
        DecisionRunOutcome::Cancelled { attempts, .. } => {
            assert_eq!(attempts.len(), 1);
            assert!(matches!(
                attempts[0].evidence,
                AttemptEvidence::RemoteOutcomeUnknown {
                    failure_class: AttemptFailureClass::CallerAborted,
                    ..
                }
            ));
        }
        _ => panic!("body bypassed cancellation"),
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn real_adapter_deadline_is_timeout_and_retries_finitely() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
    let server = tokio::spawn(stall_connections_until_peer_close(listener, 2));
    let mut input = mixed_input(&endpoint);
    input.attempts.attempt_timeout_ms = 100;
    input.attempts.retry_count = 1;
    input.attempts.initial_backoff_ms = 1;
    let result = tokio::time::timeout(
        Duration::from_secs(4),
        run_judgments(&input, &backend(&endpoint), &CancellationToken::new()),
    )
    .await
    .unwrap();
    match result {
        DecisionRunOutcome::ProviderFailure {
            failure_class,
            attempts,
            ..
        } => {
            assert_eq!(failure_class, ProviderFailureClass::Timeout);
            assert_eq!(attempts.len(), 2);
            for attempt in attempts {
                assert!(matches!(
                    attempt.evidence,
                    AttemptEvidence::RemoteOutcomeUnknown {
                        failure_class: AttemptFailureClass::Timeout,
                        ..
                    }
                ));
            }
        }
        _ => panic!("real deadline misclassified"),
    }
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn real_adapter_cancel_during_backoff_prevents_next_dispatch() {
    let server = TestServer::start(vec![HttpReply {
        status: 503,
        body: Vec::new(),
        chunked: false,
    }])
    .await;
    let observed = Arc::new(Notify::new());
    let mut backend = backend(&server.endpoint);
    backend.test_observation = Some(AdapterTestObservation::HttpFailure {
        observed: observed.clone(),
    });
    let mut input = mixed_input(&server.endpoint);
    input.attempts.retry_count = 3;
    input.attempts.initial_backoff_ms = 60_000;
    input.attempts.max_backoff_ms = 60_000;
    let cancellation = CancellationToken::new();
    let task_cancel = cancellation.clone();
    let task = tokio::spawn(async move { run_judgments(&input, &backend, &task_cancel).await });
    tokio::time::timeout(Duration::from_secs(2), observed.notified())
        .await
        .unwrap();
    // Current-thread runtime: after notifying, dispatch returns immediately,
    // and the controller has no await before its backoff select. This task
    // resumes only once that poll has yielded at backoff.
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    match result {
        DecisionRunOutcome::Cancelled { attempts, .. } => {
            assert_eq!(attempts.len(), 2);
            assert!(matches!(
                attempts[0].evidence,
                AttemptEvidence::RemoteOutcomeUnknown {
                    failure_class: AttemptFailureClass::ProviderUnavailable,
                    http_status: Some(503)
                }
            ));
            assert!(matches!(
                attempts[1].evidence,
                AttemptEvidence::NotDispatched {
                    failure_class: AttemptFailureClass::CallerAborted
                }
            ));
        }
        _ => panic!("backoff ignored abort"),
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn real_adapter_abort_after_return_does_not_rewrite_completion() {
    let server = TestServer::start(vec![HttpReply::json(response())]).await;
    let input = mixed_input(&server.endpoint);
    let cancellation = CancellationToken::new();
    let result = run_judgments(&input, &backend(&server.endpoint), &cancellation).await;
    let snapshot = serde_json::to_value(&result).unwrap();
    cancellation.cancel();
    assert!(matches!(result, DecisionRunOutcome::Completed { .. }));
    assert_eq!(serde_json::to_value(result).unwrap(), snapshot);
    assert_eq!(server.finish().await, 1);
}

async fn stall_connections_until_peer_close(
    listener: TcpListener,
    expected_connections: usize,
) -> Vec<usize> {
    let mut received_bytes = Vec::new();
    for _ in 0..expected_connections {
        let (mut connection, _) = listener.accept().await.unwrap();
        // A deadline can close transport before the HTTP request is written.
        // Keep the listener alive for the next attempt even after an empty EOF.
        let mut request_bytes = Vec::new();
        connection.read_to_end(&mut request_bytes).await.unwrap();
        received_bytes.push(request_bytes.len());
    }
    received_bytes
}

#[tokio::test]
async fn deadline_fixture_accepts_peer_close_before_request_bytes() {
    use tokio::{io::AsyncWriteExt, net::TcpStream};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(stall_connections_until_peer_close(listener, 2));
    let first = TcpStream::connect(address).await.unwrap();
    drop(first);
    let mut second = TcpStream::connect(address).await.unwrap();
    let request = b"POST /v1/systemone HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
    second.write_all(request).await.unwrap();
    second.shutdown().await.unwrap();
    let captures = tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(captures, vec![0, request.len()]);
}
