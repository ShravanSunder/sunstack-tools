#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "model_request_dispatch_test/dispatch_error_contracts.rs"]
mod dispatch_error_contracts;

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct AdmissionCountingBackend {
    inner: JevOpenRouterBackend,
    accounting: TokenAccounting,
    dispatches: Arc<AtomicUsize>,
}
impl DecisionBackend for AdmissionCountingBackend {
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
    fn account_request(&self, _: &EncodedBackendRequest) -> TokenAccounting {
        self.accounting.clone()
    }
    fn dispatch<'a>(
        &'a self,
        request: &'a AdmittedBackendRequest,
        max: usize,
    ) -> BackendAttemptFuture<'a> {
        self.dispatches.fetch_add(1, Ordering::SeqCst);
        self.inner.dispatch(request, max)
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        self.inner.admit_response(request, body)
    }
}

#[tokio::test]
async fn preparation_preserves_invalid_schema_and_unavailable_accounting_without_transport() {
    for invalid_schema in [true, false] {
        let backend_selection = BackendSelection::JevOpenrouter {
            endpoint: "http://127.0.0.1:1/unused".into(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        };
        let counter = Arc::new(AtomicUsize::new(0));
        let backend = Arc::new(AdmissionCountingBackend {
            inner: JevOpenRouterBackend::new(backend_selection.clone(), "SYNTHETIC_UNUSED_KEY")
                .unwrap(),
            accounting: TokenAccounting::Unavailable,
            dispatches: counter.clone(),
        });
        let settings = RulebookRunSettings {
            backend: backend_selection,
            attempts: AttemptSettings::default(),
            request_budget: RequestBudgetSettings {
                max_input_tokens: 100000,
                max_request_bytes: 100000,
                max_questions: 32,
                max_requests: 100,
                safety_margin_tokens: 0,
                accounting_policy: AccountingPolicy::AllowEstimated,
            },
            request_max_chars: 20000,
            max_concurrent_requests: 1,
            max_model_requests: 100,
            max_locate_lines: 80,
        };
        let mut request: StructuredJudgmentRequest = serde_json::from_value(serde_json::json!({
            "state":{"body":"A fixed source."},"questions":{"quality":{"type":"noul","instructions":{"check":"body","task":"Does it comply?"}}}
        })).unwrap();
        if invalid_schema {
            request.questions.clear();
        }
        let mut dispatcher =
            ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new())
                .prepare_only();
        let Err(DispatchRefusal::AdmissionRefused { diagnostics }) =
            dispatcher.submit(request.clone()).await
        else {
            panic!("missing typed admission refusal")
        };
        let value = serde_json::to_value(diagnostics).unwrap();
        assert_eq!(
            value["items"][0]["code"],
            if invalid_schema {
                "invalid_question"
            } else {
                "budget_unverifiable"
            }
        );
        if invalid_schema {
            assert!(value["items"][0].get("admission").is_none());
        } else {
            assert_eq!(
                value["items"][0]["admission"]["accounting"]["kind"],
                "unavailable"
            );
            assert!(value["items"][0]["admission"].get("observed").is_none());
        }
        assert!(dispatcher.prepared.is_empty());
        assert!(dispatcher.drain().await.is_empty());
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
