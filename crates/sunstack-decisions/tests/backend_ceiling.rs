#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_decisions::*;

mod support;
use support::*;

#[test]
fn backend_selection_allows_an_absent_ceiling_but_declared_ceilings_are_positive() {
    let missing = json!({
        "kind": "jev_openrouter",
        "endpoint": "http://127.0.0.1:1/",
        "model": "jev-latest"
    });
    assert!(
        serde_json::from_value::<BackendSelection>(missing.clone()).is_ok(),
        "an unknown provider ceiling must not reject a caller-bounded request"
    );
    for invalid in [json!(null), json!(0), json!(-1), json!(1.5)] {
        let mut selection = missing.clone();
        selection["context_ceiling_tokens"] = invalid;
        assert!(serde_json::from_value::<BackendSelection>(selection).is_err());
    }
    let mut declared = missing;
    declared["context_ceiling_tokens"] = json!(262144);
    assert!(serde_json::from_value::<BackendSelection>(declared).is_ok());
}

#[tokio::test]
async fn caller_declared_ceiling_constrains_a_higher_request_limit_without_dispatch() {
    let server = TestServer::start(vec![HttpReply::json(response())]).await;
    let mut input = mixed_input(&server.endpoint);
    let BackendSelection::JevOpenrouter {
        context_ceiling_tokens,
        ..
    } = &mut input.backend;
    *context_ceiling_tokens = Some(1.try_into().unwrap());
    let backend = JevOpenRouterBackend::new(input.backend.clone(), "synthetic-key").unwrap();
    assert_eq!(input.request_budget.max_input_tokens, 100_000);
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    let count = server.finish().await;
    let DecisionRunOutcome::InvalidRequest { diagnostics } = outcome else {
        panic!("one-token provider ceiling did not refuse the request")
    };
    assert_eq!(
        diagnostics.items[0].code,
        DecisionDiagnosticCode::ContextBudgetExceeded
    );
    assert_eq!(count, 0);
}

#[tokio::test]
async fn selection_is_the_only_ceiling_source_and_exact_policy_stays_unverifiable() {
    let server = TestServer::start(Vec::new()).await;
    let mut input = mixed_input(&server.endpoint);
    let BackendSelection::JevOpenrouter {
        context_ceiling_tokens,
        ..
    } = &mut input.backend;
    *context_ceiling_tokens = Some(1234.try_into().unwrap());
    let backend = match JevOpenRouterBackend::prepare(input.backend.clone(), "synthetic-key") {
        BackendPreparation::Ready { backend } => backend,
        BackendPreparation::Refused { .. } => panic!("valid configuration refused"),
    };
    let encoded = backend.encode_request(&input.request).unwrap();
    assert!(matches!(
        backend.account_request(&encoded),
        TokenAccounting::Estimated {
            provider_ceiling: ProviderContextCeiling::CallerDeclared { tokens },
            ..
        } if tokens.get() == 1234
    ));
    let accounting = backend.account_request(&encoded);
    match assess_request_budget(encoded, &input.request_budget, accounting) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => assert_eq!(
            serde_json::to_value(&admitted_request.counts().basis).unwrap(),
            json!({"kind":"estimated","provider_ceiling":{"kind":"caller_declared","tokens":1234}})
        ),
        _ => panic!("declared ceiling should admit this small request"),
    }
    input.request_budget.accounting_policy = AccountingPolicy::RequireExact;
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    let DecisionRunOutcome::InvalidRequest { diagnostics } = outcome else {
        panic!("byte estimation was accepted as exact")
    };
    assert_eq!(
        diagnostics.items[0].code,
        DecisionDiagnosticCode::BudgetUnverifiable
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn allowed_estimation_reaches_real_http_without_exporting_local_ceiling_policy() {
    let mut server = TestServer::start(vec![HttpReply::json(response())]).await;
    let input = mixed_input(&server.endpoint);
    let backend = JevOpenRouterBackend::new(input.backend.clone(), "synthetic-key").unwrap();
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    assert!(matches!(outcome, DecisionRunOutcome::Completed { .. }));
    let wire: serde_json::Value =
        serde_json::from_slice(&server.captures.recv().await.unwrap().body).unwrap();
    assert!(wire.get("context_ceiling_tokens").is_none());
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn absent_ceiling_keeps_the_caller_limit_and_records_no_provider_claim() {
    let mut server = TestServer::start(vec![
        HttpReply::json(response()),
        HttpReply::json(response()),
    ])
    .await;
    let mut input = mixed_input(&server.endpoint);
    let backend = JevOpenRouterBackend::new(input.backend.clone(), "synthetic-key").unwrap();
    let encoded = backend.encode_request(&input.request).unwrap();
    let accounting = backend.account_request(&encoded);
    let admitted = match assess_request_budget(encoded, &input.request_budget, accounting) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => admitted_request,
        _ => panic!("caller-bounded request without a provider ceiling was refused"),
    };
    assert_eq!(
        serde_json::to_value(&admitted.counts().basis).unwrap(),
        json!({"kind":"estimated","provider_ceiling":{"kind":"not_declared"}})
    );
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    assert!(matches!(outcome, DecisionRunOutcome::Completed { .. }));
    let wire: serde_json::Value =
        serde_json::from_slice(&server.captures.recv().await.unwrap().body).unwrap();
    assert!(wire.get("context_ceiling_tokens").is_none());
    input.request_budget.max_input_tokens = 1;
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    let count = server.finish().await;
    assert!(
        matches!(outcome, DecisionRunOutcome::InvalidRequest { diagnostics } if diagnostics.items[0].code == DecisionDiagnosticCode::ContextBudgetExceeded)
    );
    assert_eq!(count, 1, "the caller limit must refuse the second dispatch");
}
