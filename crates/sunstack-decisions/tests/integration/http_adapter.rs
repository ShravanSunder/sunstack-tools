#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_decisions::test_support as support;
use sunstack_decisions::*;
use support::*;

#[tokio::test]
async fn real_http_preserves_structure_receipt_and_exact_admitted_bytes() {
    let mut server = TestServer::start(vec![HttpReply::json(response())]).await;
    let backend = backend(&server.endpoint);
    let input = mixed_input(&server.endpoint);
    let encoded = backend.encode_request(&input.request).unwrap();
    let assessed_bytes = encoded.body().to_vec();
    let admitted = match assess_request_budget(
        encoded.clone(),
        &input.request_budget,
        backend.account_request(&encoded),
    ) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => admitted_request,
        _ => panic!("budget refused"),
    };
    let result = run_admitted_judgments(
        &input.request,
        &backend,
        &admitted,
        &input.attempts,
        &CancellationToken::new(),
    )
    .await;
    let capture = server.captures.recv().await.unwrap();
    assert_eq!(capture.body, assessed_bytes);
    let wire: Value = serde_json::from_slice(&capture.body).unwrap();
    assert_eq!(
        wire["state"],
        serde_json::to_value(&input.request.state).unwrap()
    );
    assert_eq!(
        wire["questions"],
        serde_json::to_value(&input.request.questions).unwrap()
    );
    assert_eq!(
        wire.as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["model", "questions", "state"]
    );
    for policy in ["threshold", "severity", "band", "weight", "route"] {
        assert!(wire.get(policy).is_none());
        assert!(
            wire["questions"]
                .as_object()
                .unwrap()
                .values()
                .all(|question| question.get(policy).is_none())
        );
    }
    assert_eq!(
        String::from_utf8(capture.body)
            .unwrap()
            .matches("unique-state-")
            .count(),
        1
    );
    assert!(capture.headers.contains("Bearer SENTINEL_CREDENTIAL_987"));
    match result {
        DecisionRunOutcome::Completed {
            answers,
            receipt,
            attempts,
        } => {
            assert_eq!(answers.len(), 3);
            assert_eq!(receipt.resolved_model.as_str(), "typesafe/observed-model");
            assert_eq!(receipt.usage.input_tokens, 123);
            assert_eq!(receipt.usage.output_tokens, 9);
            assert_eq!(receipt.usage.cost_usd, Some(0.002));
            assert_eq!(
                attempts,
                vec![InferenceAttemptRecord {
                    ordinal: 1,
                    evidence: AttemptEvidence::Completed { http_status: 200 }
                }]
            );
            assert!(
                !serde_json::to_string(&receipt)
                    .unwrap()
                    .contains("SENTINEL_METADATA")
            );
        }
        _ => panic!("expected completed"),
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn cost_absence_and_additive_metadata_are_not_fabricated_or_leaked() {
    let mut value = response();
    value["usage"].as_object_mut().unwrap().remove("cost");
    let server = TestServer::start(vec![HttpReply::json(value)]).await;
    let outcome = run_judgments(
        &mixed_input(&server.endpoint),
        &backend(&server.endpoint),
        &CancellationToken::new(),
    )
    .await;
    match outcome {
        DecisionRunOutcome::Completed { receipt, .. } => assert_eq!(receipt.usage.cost_usd, None),
        _ => panic!("missing cost refused"),
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn malformed_receipts_refuse_completion() {
    let mut cases = Vec::new();
    for field in ["model", "usage"] {
        let mut value = response();
        value.as_object_mut().unwrap().remove(field);
        cases.push(value);
    }
    for (field, value) in [
        ("input_tokens", json!(-1)),
        ("output_tokens", json!(1.5)),
        ("cost", json!(null)),
        ("cost", json!(-0.2)),
        ("cost", json!("SENTINEL_PROVIDER_BODY")),
    ] {
        let mut body = response();
        body["usage"][field] = value;
        cases.push(body);
    }
    for value in cases {
        let server = TestServer::start(vec![HttpReply::json(value)]).await;
        let result = run_judgments(
            &mixed_input(&server.endpoint),
            &backend(&server.endpoint),
            &CancellationToken::new(),
        )
        .await;
        match result {
            DecisionRunOutcome::InvalidResponse {
                diagnostics,
                attempts,
            } => {
                assert_eq!(
                    diagnostics.items[0].code,
                    DecisionDiagnosticCode::InvalidReceipt
                );
                assert!(matches!(
                    attempts[0].evidence,
                    AttemptEvidence::RemoteOutcomeUnknown {
                        failure_class: AttemptFailureClass::InvalidResponse,
                        ..
                    }
                ));
            }
            _ => panic!("malformed receipt completed"),
        }
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn malformed_answer_fails_combined_response_and_never_salvages_peers() {
    let mut value = response();
    value["answers"]["native_choice"]["probabilities"] = json!({"alpha":0.7,"unknown":0.3});
    let server = TestServer::start(vec![HttpReply::json(value)]).await;
    let result = run_judgments(
        &mixed_input(&server.endpoint),
        &backend(&server.endpoint),
        &CancellationToken::new(),
    )
    .await;
    assert!(matches!(result, DecisionRunOutcome::InvalidResponse { .. }));
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn status_codes_use_closed_safe_diagnostics_without_copying_provider_data() {
    for (status, class) in [
        (400, ProviderFailureClass::ProviderRejectedRequest),
        (401, ProviderFailureClass::Authentication),
        (402, ProviderFailureClass::InsufficientCredits),
        (403, ProviderFailureClass::Authorization),
        (404, ProviderFailureClass::ProviderResourceUnavailable),
        (413, ProviderFailureClass::RequestTooLarge),
        (429, ProviderFailureClass::RateLimit),
        (500, ProviderFailureClass::ProviderUnavailable),
        (524, ProviderFailureClass::Timeout),
        (529, ProviderFailureClass::ProviderUnavailable),
        (418, ProviderFailureClass::HttpError),
    ] {
        let server = TestServer::start(vec![HttpReply {
            status,
            body: b"SENTINEL_PROVIDER_BODY".to_vec(),
            chunked: false,
        }])
        .await;
        let mut input = mixed_input(&server.endpoint);
        input.request.state = SharedEvidenceState::Text("SENTINEL_EVIDENCE".into());
        let result = run_judgments(
            &input,
            &backend(&server.endpoint),
            &CancellationToken::new(),
        )
        .await;
        let serialized = serde_json::to_string(&result).unwrap();
        for sentinel in [
            "SENTINEL_CREDENTIAL_987",
            "SENTINEL_PROVIDER_BODY",
            "SENTINEL_HEADER",
            "SENTINEL_EVIDENCE",
        ] {
            assert!(!serialized.contains(sentinel));
        }
        match result {
            DecisionRunOutcome::ProviderFailure {
                failure_class,
                http_status,
                diagnostics,
                attempts,
            } => {
                assert_eq!(failure_class, class);
                assert_eq!(http_status, Some(status));
                assert_eq!(diagnostics.items.len(), 1);
                assert!(diagnostics.items[0].guidance.len() <= 512);
                assert!(!diagnostics.items[0].guidance.is_empty());
                assert_eq!(diagnostics.items[0], provider_diagnostic(class));
                assert_eq!(attempts.len(), 1);
                assert!(matches!(
                    attempts[0].evidence,
                    AttemptEvidence::RemoteOutcomeUnknown { .. }
                ));
            }
            _ => panic!("wrong status outcome"),
        }
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn response_bytes_bounded_with_and_without_content_length() {
    for chunked in [false, true] {
        let server = TestServer::start(vec![HttpReply {
            status: 200,
            body: vec![b'x'; 100],
            chunked,
        }])
        .await;
        let mut input = mixed_input(&server.endpoint);
        input.attempts.max_response_bytes = 16;
        let result = run_judgments(
            &input,
            &backend(&server.endpoint),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(result, DecisionRunOutcome::InvalidResponse { .. }));
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn retries_are_finite_and_only_for_transient_classes() {
    let server = TestServer::start(vec![
        HttpReply {
            status: 503,
            body: Vec::new(),
            chunked: false,
        },
        HttpReply::json(response()),
    ])
    .await;
    let mut input = mixed_input(&server.endpoint);
    input.attempts.retry_count = 1;
    input.attempts.initial_backoff_ms = 1;
    let result = run_judgments(
        &input,
        &backend(&server.endpoint),
        &CancellationToken::new(),
    )
    .await;
    assert!(matches!(result,DecisionRunOutcome::Completed {ref attempts,..} if attempts.len()==2));
    assert_eq!(server.finish().await, 2);
    let server = TestServer::start(vec![HttpReply {
        status: 401,
        body: Vec::new(),
        chunked: false,
    }])
    .await;
    input.backend = selection(&server.endpoint);
    input.attempts.retry_count = 5;
    let result = run_judgments(
        &input,
        &backend(&server.endpoint),
        &CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(result,DecisionRunOutcome::ProviderFailure {ref attempts,..} if attempts.len()==1)
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn malformed_json_and_answer_shapes_are_invalid_bodies() {
    for body in [b"{broken".to_vec(), b"null".to_vec(), {
        let mut value = response();
        value["answers"]["native_yes"]["noul"] = json!("wrong numeric type");
        serde_json::to_vec(&value).unwrap()
    }] {
        let server = TestServer::start(vec![HttpReply {
            status: 200,
            body,
            chunked: false,
        }])
        .await;
        let result = run_judgments(
            &mixed_input(&server.endpoint),
            &backend(&server.endpoint),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(result, DecisionRunOutcome::InvalidResponse { .. }));
        assert_eq!(server.finish().await, 1);
    }
}
