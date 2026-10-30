#![allow(clippy::unwrap_used)]
use serde_json::json;
use sunstack_decisions::*;
use sunstack_evidence::FieldName;
use sunstack_rules::*;

fn settings(endpoint: &str) -> RulebookRunSettings {
    RulebookRunSettings {
        backend: BackendSelection::JevOpenrouter {
            endpoint: endpoint.into(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        },
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
    }
}

#[tokio::test]
async fn sdk_callers_propagate_dispatch_and_selection_errors_with_question_mark() {
    fn assess(
        dispatch: &ModelRequestDispatch,
        request: &StructuredJudgmentRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        dispatch.assess_selection(request)?;
        Ok(())
    }
    async fn submit(
        dispatch: &mut ModelRequestDispatch,
        request: StructuredJudgmentRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        dispatch.submit(request).await?;
        Ok(())
    }
    let request: StructuredJudgmentRequest =
        serde_json::from_value(json!({"questions":{},"state":{}})).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut dispatch =
        ModelRequestDispatch::new(settings("http://127.0.0.1:1/unused"), None, cancellation);
    let selection_error = assess(&dispatch, &request).unwrap_err();
    assert!(matches!(
        selection_error.downcast_ref::<SelectionIncompleteReason>(),
        Some(SelectionIncompleteReason::InvalidRequest)
    ));
    let dispatch_error = submit(&mut dispatch, request).await.unwrap_err();
    assert!(matches!(
        dispatch_error.downcast_ref::<DispatchRefusal>(),
        Some(DispatchRefusal::Cancelled)
    ));
}

#[test]
fn selection_error_display_does_not_change_serialized_reason_tags_or_payloads() {
    for (reason, expected) in [
        (
            SelectionIncompleteReason::EvidenceUnavailable {
                field: FieldName::new("body").unwrap(),
            },
            json!({"kind":"evidence_unavailable","field":"body"}),
        ),
        (
            SelectionIncompleteReason::EvidenceTooLarge,
            json!({"kind":"evidence_too_large"}),
        ),
        (
            SelectionIncompleteReason::RequestTooLarge {
                limit: RequestLimit::RequestMaxChars,
            },
            json!({"kind":"request_too_large","limit":"request_max_chars"}),
        ),
        (
            SelectionIncompleteReason::InvalidRequest,
            json!({"kind":"invalid_request"}),
        ),
        (
            SelectionIncompleteReason::ProviderFailure,
            json!({"kind":"provider_failure"}),
        ),
        (
            SelectionIncompleteReason::MalformedAnswer,
            json!({"kind":"malformed_answer"}),
        ),
        (
            SelectionIncompleteReason::RequestLimitReached,
            json!({"kind":"request_limit_reached"}),
        ),
        (
            SelectionIncompleteReason::Cancelled,
            json!({"kind":"cancelled"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(reason).unwrap(), expected);
    }
}

struct ExpandedEncodingBackend {
    inner: JevOpenRouterBackend,
}
impl DecisionBackend for ExpandedEncodingBackend {
    fn identity(&self) -> &BackendIdentity {
        self.inner.identity()
    }
    fn selection(&self) -> &BackendSelection {
        DecisionBackend::selection(&self.inner)
    }
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic> {
        let encoded = self.inner.encode_request(request)?;
        let mut body = encoded.body().to_vec();
        body.extend_from_slice(&[b' '; 256]);
        Ok(EncodedBackendRequest::new(
            self.inner.identity().clone(),
            body,
            request.questions.len(),
        ))
    }
    fn account_request(&self, request: &EncodedBackendRequest) -> TokenAccounting {
        self.inner.account_request(request)
    }
    fn dispatch<'request>(
        &'request self,
        request: &'request AdmittedBackendRequest,
        max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        self.inner.dispatch(request, max_response_bytes)
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        self.inner.admit_response(request, body)
    }
}

#[test]
fn selection_byte_limit_names_the_backend_encoded_measure_that_exceeded_the_budget() {
    let request: StructuredJudgmentRequest = serde_json::from_value(json!({
        "questions":{"candidate":{"type":"noul","instructions":{"check":"body","task":"Does it comply?"}}},
        "state":{"body":"A small judgment body."}
    })).unwrap();
    let mut settings = settings("http://127.0.0.1:1/unused");
    let BackendSelection::JevOpenrouter { model, .. } = &settings.backend;
    let judgment = encode_judgment_body(&request, model).unwrap();
    settings.request_budget.max_request_bytes = judgment.len() + 1;
    let backend = std::sync::Arc::new(ExpandedEncodingBackend {
        inner: JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_ENCODING_KEY")
            .unwrap(),
    });
    let encoded = backend.encode_request(&request).unwrap();
    assert!(judgment.len() < settings.request_budget.max_request_bytes);
    assert!(encoded.body().len() > settings.request_budget.max_request_bytes);
    let encoded_bytes = encoded.body().len();
    let accounting = backend.account_request(&encoded);
    assert!(matches!(
        assess_request_budget(encoded, &settings.request_budget, accounting),
        RequestBudgetAssessment::Exceeded { .. }
    ));
    let mut control_settings = settings.clone();
    control_settings.request_budget.max_request_bytes = encoded_bytes;
    let control = ModelRequestDispatch::new(
        control_settings,
        Some(backend.clone()),
        CancellationToken::new(),
    );
    assert!(control.assess_selection(&request).is_ok());
    let dispatcher = ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());
    assert!(matches!(
        dispatcher.assess_selection(&request),
        Err(SelectionIncompleteReason::RequestTooLarge {
            limit: RequestLimit::RequestBytes
        })
    ));
}
