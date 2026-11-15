#![allow(clippy::unwrap_used)]
use super::super::*;
use crate::test_support::controlled_http::ControlledServer;
use serde_json::json;

fn settings(endpoint: &str) -> RulebookRunSettings {
    RulebookRunSettings {
        backend: BackendSelection::JevOpenrouter {
            endpoint: endpoint.into(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        },
        attempts: AttemptSettings::default(),
        request_budget: RequestBudgetSettings {
            max_input_tokens: 100_000,
            max_request_bytes: 100_000,
            max_questions: 32,
            max_requests: 10,
            safety_margin_tokens: 0,
            accounting_policy: AccountingPolicy::AllowEstimated,
        },
        request_max_chars: 100_000,
        max_concurrent_requests: 1,
        max_model_requests: 10,
        max_locate_lines: 80,
    }
}

fn reservations(dispatcher: &ModelRequestDispatch) -> [usize; 8] {
    [
        dispatcher.requested,
        dispatcher.next_request_id,
        dispatcher.semaphore.available_permits(),
        dispatcher.tasks.len(),
        dispatcher.records.len(),
        dispatcher.prepared.len(),
        dispatcher.selection_requests.len(),
        dispatcher.reused_selections.len(),
    ]
}

#[tokio::test]
async fn unavailable_submission_preserves_every_reservation_after_a_live_keyed_control() {
    let mut server = ControlledServer::start().await;
    let settings = settings(&server.endpoint);
    let backend = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "synthetic-loopback-only").unwrap(),
    );
    let request: StructuredJudgmentRequest = serde_json::from_value(json!({
        "questions": {"quality": {"type":"noul", "instructions":{"check":"body", "task":"Does it comply?"}}},
        "state": {"body":"A real bounded request."}
    })).unwrap();
    let mut keyed = ModelRequestDispatch::new(
        settings.clone(),
        ModelDispatchPolicy::Execution { backend },
        CancellationToken::new(),
    );
    assert_eq!(reservations(&keyed), [0, 0, 1, 0, 0, 0, 0, 0]);
    assert_eq!(keyed.submit(request.clone()).await.unwrap(), 0);
    assert_eq!(reservations(&keyed), [1, 1, 0, 1, 0, 0, 0, 0]);
    let observed = server.next().await;
    assert_eq!(observed.body["state"]["body"], "A real bounded request.");
    assert!(observed.body["questions"].get("quality").is_some());
    observed.noul(0.9);
    let completed = keyed.drain().await;
    assert_eq!(completed.len(), 1);
    assert!(matches!(
        completed[0].outcome,
        DecisionRunOutcome::Completed { .. }
    ));
    assert_eq!(reservations(&keyed), [1, 1, 1, 0, 1, 0, 0, 0]);

    let cancellation = CancellationToken::new();
    let mut unavailable = ModelRequestDispatch::new(
        settings,
        ModelDispatchPolicy::CredentialUnavailable,
        cancellation.clone(),
    );
    let before = reservations(&unavailable);
    assert_eq!(before, [0, 0, 1, 0, 0, 0, 0, 0]);
    assert!(matches!(
        unavailable.check_capability(),
        Err(DispatchRefusal::Unavailable(
            crate::WorkUnavailableReason::CredentialUnavailable {}
        ))
    ));
    assert!(matches!(
        unavailable.submit(request.clone()).await,
        Err(DispatchRefusal::Unavailable(
            crate::WorkUnavailableReason::CredentialUnavailable {}
        ))
    ));
    assert_eq!(reservations(&unavailable), before);
    assert!(unavailable.drain().await.is_empty());
    cancellation.cancel();
    assert!(matches!(
        unavailable.submit(request).await,
        Err(DispatchRefusal::Cancelled)
    ));
    assert_eq!(reservations(&unavailable), before);
    assert_eq!(
        server.finish().await,
        1,
        "only the keyed control reached HTTP"
    );
}
