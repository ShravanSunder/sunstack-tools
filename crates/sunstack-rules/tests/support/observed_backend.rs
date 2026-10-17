#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use sunstack_decisions::*;
use tokio::sync::mpsc;
#[derive(Debug)]
pub enum AdapterEvent {
    AttemptFailed,
    ResponseAdmitted,
}
/// Observe the real adapter's completed attempt/validation boundary without replacing either.
pub struct ObservedBackend {
    pub inner: JevOpenRouterBackend,
    pub events: mpsc::UnboundedSender<AdapterEvent>,
}
impl DecisionBackend for ObservedBackend {
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
        request: &'request AdmittedBackendRequest,
        max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        Box::pin(async move {
            let result = self.inner.dispatch(request, max_response_bytes).await;
            if result.is_err() {
                let _ = self.events.send(AdapterEvent::AttemptFailed);
            }
            result
        })
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        let result = self.inner.admit_response(request, body);
        if result.is_ok() {
            let _ = self.events.send(AdapterEvent::ResponseAdmitted);
        }
        result
    }
}
pub async fn next_event(events: &mut mpsc::UnboundedReceiver<AdapterEvent>) -> AdapterEvent {
    tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
        .await
        .expect("adapter did not settle")
        .expect("adapter event stream ended")
}
