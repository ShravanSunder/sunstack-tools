#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_decisions::*;

use std::time::Duration;
use sunstack_decisions::test_support as support;
use support::*;

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
