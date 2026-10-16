#![allow(clippy::unwrap_used, clippy::panic)]
mod support;
use sunstack_decisions::*;
use support::*;

fn encoded() -> EncodedBackendRequest {
    EncodedBackendRequest::new(
        BackendIdentity::new(),
        b"{\"state\":\"\\\"\\n\"}".to_vec(),
        3,
    )
}
#[test]
fn exact_estimated_unavailable_and_provider_ceiling_admission() {
    let mut settings = budget();
    settings.max_input_tokens = 100;
    settings.safety_margin_tokens = 7;
    let accounting = TokenAccounting::Exact {
        input_tokens: 93,
        context_ceiling_tokens: 100,
    };
    assert!(matches!(
        assess_request_budget(encoded(), &settings, accounting),
        RequestBudgetAssessment::WithinConfiguredBudget { .. }
    ));
    for accounting in [
        TokenAccounting::Exact {
            input_tokens: 94,
            context_ceiling_tokens: 100,
        },
        TokenAccounting::Exact {
            input_tokens: 93,
            context_ceiling_tokens: 99,
        },
        TokenAccounting::Exact {
            input_tokens: u64::MAX,
            context_ceiling_tokens: u64::MAX,
        },
    ] {
        assert!(matches!(
            assess_request_budget(encoded(), &settings, accounting),
            RequestBudgetAssessment::Exceeded { .. }
        ));
    }
    settings.accounting_policy = AccountingPolicy::RequireExact;
    for accounting in [
        TokenAccounting::Estimated {
            input_tokens: 1,
            context_ceiling_tokens: 100,
        },
        TokenAccounting::Unavailable,
        TokenAccounting::Exact {
            input_tokens: 1,
            context_ceiling_tokens: 0,
        },
    ] {
        assert!(matches!(
            assess_request_budget(encoded(), &settings, accounting),
            RequestBudgetAssessment::Unverifiable { .. }
        ));
    }
    settings.accounting_policy = AccountingPolicy::AllowEstimated;
    match assess_request_budget(
        encoded(),
        &settings,
        TokenAccounting::Estimated {
            input_tokens: 12,
            context_ceiling_tokens: 100,
        },
    ) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => {
            assert_eq!(admitted_request.counts().input_tokens, 19);
            assert_eq!(admitted_request.counts().basis, AccountingBasis::Estimated);
        }
        _ => panic!("explicit estimated allowance refused"),
    }
}

#[tokio::test]
async fn each_budget_refusal_makes_zero_http_calls() {
    for case in 0..7 {
        let server = TestServer::start(Vec::new()).await;
        let mut input = mixed_input(&server.endpoint);
        let backend = if case == 5 {
            JevOpenRouterBackend::new(
                selection(&server.endpoint),
                "key",
                JevAccountingProfile::Unavailable,
            )
            .unwrap()
        } else {
            backend(&server.endpoint)
        };
        match case {
            0 => input.request_budget.max_request_bytes = 1,
            1 => input.request_budget.max_input_tokens = 1,
            2 => input.request_budget.max_questions = 1,
            3 => input.request_budget.max_requests = 0,
            4 => input.request_budget.accounting_policy = AccountingPolicy::RequireExact,
            5 => {}
            6 => input.request_budget.safety_margin_tokens = u64::MAX,
            _ => panic!("unknown case"),
        }
        let result = run_judgments(&input, &backend, &CancellationToken::new()).await;
        assert!(matches!(result, DecisionRunOutcome::InvalidRequest { .. }));
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn synthetic_exact_accounting_binds_real_body_bytes_and_backend_identity() {
    let mut server = TestServer::start(vec![HttpReply::json(response())]).await;
    let backend = backend(&server.endpoint);
    let input = mixed_input(&server.endpoint);
    let encoded = backend.encode_request(&input.request).unwrap();
    let expected = encoded.body().to_vec();
    let admitted = match assess_request_budget(
        encoded,
        &input.request_budget,
        TokenAccounting::Exact {
            input_tokens: 15,
            context_ceiling_tokens: 1000,
        },
    ) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => admitted_request,
        _ => panic!("refused synthetic count"),
    };
    let other = JevOpenRouterBackend::new(
        selection(&server.endpoint),
        "key",
        JevAccountingProfile::Unavailable,
    )
    .unwrap();
    let refused = run_admitted_judgments(
        &input.request,
        &other,
        &admitted,
        &input.attempts,
        &CancellationToken::new(),
    )
    .await;
    assert!(matches!(refused, DecisionRunOutcome::InvalidRequest { .. }));
    let mut changed = input.request.clone();
    changed.state = SharedEvidenceState::Text("different".into());
    assert!(matches!(
        run_admitted_judgments(
            &changed,
            &backend,
            &admitted,
            &input.attempts,
            &CancellationToken::new()
        )
        .await,
        DecisionRunOutcome::InvalidRequest { .. }
    ));
    let result = run_admitted_judgments(
        &input.request,
        &backend,
        &admitted,
        &input.attempts,
        &CancellationToken::new(),
    )
    .await;
    assert!(matches!(result, DecisionRunOutcome::Completed { .. }));
    assert_eq!(server.captures.recv().await.unwrap().body, expected);
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn encoded_envelope_escaping_and_all_questions_contribute_to_limits() {
    let server = TestServer::start(Vec::new()).await;
    let backend = backend(&server.endpoint);
    let mut input = mixed_input(&server.endpoint);
    let encoded = backend.encode_request(&input.request).unwrap();
    let state_bytes = serde_json::to_vec(&input.request.state).unwrap().len();
    assert!(encoded.body().len() > state_bytes);
    input.request_budget.max_request_bytes = encoded.body().len() - 1;
    assert!(matches!(
        run_judgments(&input, &backend, &CancellationToken::new()).await,
        DecisionRunOutcome::InvalidRequest { .. }
    ));
    assert_eq!(server.finish().await, 0);
}
