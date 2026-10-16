#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_decisions::*;

#[tokio::test]
async fn pre_aborted_request_records_caller_abort_without_dispatch() {
    let selection = BackendSelection::JevOpenrouter {
        endpoint: "http://127.0.0.1:1/".into(),
        model: ModelName::new("jev-latest").unwrap(),
    };
    let backend = JevOpenRouterBackend::new(
        selection.clone(),
        "sentinel-key",
        JevAccountingProfile::EstimatedBytes {
            context_ceiling_tokens: 262_144,
        },
    )
    .unwrap();
    let input: RunJudgmentsInput = serde_json::from_value(json!({
        "request":{"questions":{"check":{"type":"noul","instructions":"test"}},"state":"state"},
        "backend":selection,"attempts":AttemptSettings::default(),
        "request_budget":{"max_input_tokens":10000,"max_request_bytes":10000,"max_questions":10,"max_requests":1,"safety_margin_tokens":10,"accounting_policy":"allow_estimated"}
    })).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = run_judgments(&input, &backend, &cancellation).await;
    let wire = serde_json::to_value(result).unwrap();
    assert_eq!(wire["kind"], "cancelled");
    assert_eq!(
        wire["attempts"][0]["evidence"],
        json!({"kind":"not_dispatched","failure_class":"caller_aborted"})
    );
}

mod support;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use support::*;
use tokio::sync::Notify;

enum Behavior {
    Block,
    Fail,
    Success,
}
struct ControlledBackend {
    inner: JevOpenRouterBackend,
    calls: AtomicUsize,
    active: Arc<AtomicUsize>,
    started: Notify,
    behavior: Behavior,
    cancel_on_admission: Option<CancellationToken>,
}
struct ActiveGuard(Arc<AtomicUsize>);
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl ControlledBackend {
    fn new(behavior: Behavior) -> Self {
        Self {
            inner: backend("http://localhost/"),
            calls: AtomicUsize::new(0),
            active: Arc::new(AtomicUsize::new(0)),
            started: Notify::new(),
            behavior,
            cancel_on_admission: None,
        }
    }
}
impl DecisionBackend for ControlledBackend {
    fn identity(&self) -> &BackendIdentity {
        self.inner.identity()
    }
    fn selection(&self) -> &BackendSelection {
        self.inner.selection()
    }
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic> {
        self.inner.encode_request(request)
    }
    fn account_request(&self, encoded: &EncodedBackendRequest) -> TokenAccounting {
        self.inner.account_request(encoded)
    }
    fn dispatch<'request>(
        &'request self,
        _request: &'request AdmittedBackendRequest,
        _max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.active.fetch_add(1, Ordering::SeqCst);
            let _guard = ActiveGuard(self.active.clone());
            self.started.notify_one();
            match self.behavior {
                Behavior::Block => std::future::pending().await,
                Behavior::Fail => Err(BackendAttemptFailure::unknown(
                    ProviderFailureClass::Connection,
                    None,
                )),
                Behavior::Success => Ok(BackendResponse {
                    http_status: 200,
                    body: serde_json::to_vec(&response()).unwrap(),
                }),
            }
        })
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        let outcome = self.inner.admit_response(request, body);
        if let Some(cancellation) = &self.cancel_on_admission {
            cancellation.cancel();
        }
        outcome
    }
}

#[tokio::test]
async fn cancellation_settles_owned_future_and_stops_new_attempts() {
    let backend = Arc::new(ControlledBackend::new(Behavior::Block));
    let mut input = mixed_input("http://localhost/");
    input.attempts.retry_count = 5;
    let cancellation = CancellationToken::new();
    let task_backend = backend.clone();
    let task_cancellation = cancellation.clone();
    let task = tokio::spawn(async move {
        run_judgments(&input, task_backend.as_ref(), &task_cancellation).await
    });
    tokio::time::timeout(Duration::from_secs(2), backend.started.notified())
        .await
        .unwrap();
    assert_eq!(backend.active.load(Ordering::SeqCst), 1);
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(backend.active.load(Ordering::SeqCst), 0);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    match result {
        DecisionRunOutcome::Cancelled { attempts, .. } => assert_eq!(
            attempts,
            vec![InferenceAttemptRecord {
                ordinal: 1,
                evidence: AttemptEvidence::RemoteOutcomeUnknown {
                    failure_class: AttemptFailureClass::CallerAborted,
                    http_status: None
                }
            }]
        ),
        _ => panic!("cancellation lost"),
    }
}

#[tokio::test(start_paused = true)]
async fn local_deadlines_are_finite_and_release_each_owned_attempt() {
    let backend = ControlledBackend::new(Behavior::Block);
    let mut input = mixed_input("http://localhost/");
    input.attempts.retry_count = 2;
    input.attempts.attempt_timeout_ms = 10;
    let outcome = tokio::time::timeout(
        Duration::from_secs(10),
        run_judgments(&input, &backend, &CancellationToken::new()),
    )
    .await
    .unwrap();
    match outcome {
        DecisionRunOutcome::ProviderFailure {
            failure_class,
            attempts,
            ..
        } => {
            assert_eq!(failure_class, ProviderFailureClass::Timeout);
            assert_eq!(attempts.len(), 3);
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
        _ => panic!("deadline not enforced"),
    }
    assert_eq!(backend.calls.load(Ordering::SeqCst), 3);
    assert_eq!(backend.active.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn transient_failure_retries_and_pre_abort_perform_exact_finite_calls() {
    let backend = ControlledBackend::new(Behavior::Fail);
    let mut input = mixed_input("http://localhost/");
    input.attempts.retry_count = 2;
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(matches!(
        run_judgments(&input, &backend, &cancellation).await,
        DecisionRunOutcome::Cancelled { .. }
    ));
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
    let result = run_judgments(&input, &backend, &CancellationToken::new()).await;
    assert!(
        matches!(result,DecisionRunOutcome::ProviderFailure {ref attempts,..} if attempts.len()==3)
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn cancellation_before_completed_admission_wins_but_later_abort_cannot_rewrite_result() {
    let cancellation = CancellationToken::new();
    let mut backend = ControlledBackend::new(Behavior::Success);
    backend.cancel_on_admission = Some(cancellation.clone());
    let input = mixed_input("http://localhost/");
    let outcome = run_judgments(&input, &backend, &cancellation).await;
    match outcome {
        DecisionRunOutcome::Cancelled { attempts, .. } => assert!(matches!(
            attempts[0].evidence,
            AttemptEvidence::RemoteOutcomeUnknown {
                failure_class: AttemptFailureClass::CallerAborted,
                http_status: Some(200)
            }
        )),
        _ => panic!("successful response bypassed observed cancellation"),
    }
    let backend = ControlledBackend::new(Behavior::Success);
    let cancellation = CancellationToken::new();
    let result = run_judgments(&input, &backend, &cancellation).await;
    let before = serde_json::to_value(&result).unwrap();
    cancellation.cancel();
    assert!(matches!(result, DecisionRunOutcome::Completed { .. }));
    assert_eq!(serde_json::to_value(result).unwrap(), before);
}

#[tokio::test]
async fn cancellation_after_real_http_dispatch_keeps_remote_uncertainty() {
    use tokio::{io::AsyncReadExt, net::TcpListener, sync::oneshot};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
    let (dispatched, received) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        let _capture = read_request(&mut connection).await;
        dispatched.send(()).unwrap();
        let mut byte = [0];
        tokio::time::timeout(Duration::from_secs(2), connection.read(&mut byte))
            .await
            .unwrap()
            .unwrap()
    });
    let input = mixed_input(&endpoint);
    let backend = backend(&endpoint);
    let cancellation = CancellationToken::new();
    let task_cancel = cancellation.clone();
    let task = tokio::spawn(async move { run_judgments(&input, &backend, &task_cancel).await });
    tokio::time::timeout(Duration::from_secs(2), received)
        .await
        .unwrap()
        .unwrap();
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(result,DecisionRunOutcome::Cancelled {ref attempts,..} if matches!(attempts[0].evidence,AttemptEvidence::RemoteOutcomeUnknown {failure_class:AttemptFailureClass::CallerAborted,..}))
    );
    assert_eq!(server.await.unwrap(), 0);
}
