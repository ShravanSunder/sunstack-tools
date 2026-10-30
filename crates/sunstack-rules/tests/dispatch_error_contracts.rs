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
