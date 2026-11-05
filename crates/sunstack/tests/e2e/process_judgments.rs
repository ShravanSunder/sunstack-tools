#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_decisions::test_support as decision_fixtures;
#[path = "../support/mod.rs"]
mod support;
use decision_fixtures::{HttpReply, TestServer, mixed_input};
use serde_json::json;
use support::*;

#[tokio::test]
async fn malformed_judgment_input_keeps_the_native_code_and_safe_schema_path() {
    let mut input = serde_json::to_value(mixed_input("http://127.0.0.1:1/")).unwrap();
    input["request"]["questions"] =
        json!({"PRIVATE_DYNAMIC_QUESTION":{"type":"noul","instructions":true}});
    let output = run(&workspace_root(), "run_judgments", input).await;
    let wire = response(&output, 2);
    let diagnostic = &wire["result"]["outcome"]["diagnostics"]["items"][0];
    assert_eq!(diagnostic["code"], "invalid_question");
    assert_eq!(
        diagnostic["location"],
        json!({"kind":"field","field_path":["request","questions"]})
    );
    assert!(!diagnostic.to_string().contains("PRIVATE_DYNAMIC_QUESTION"));
}

#[tokio::test]
async fn real_binary_runs_mixed_judgments_with_admitted_receipt_and_attempts() {
    let mut server = TestServer::start(vec![HttpReply::json(decision_fixtures::response())]).await;
    let input = serde_json::to_value(mixed_input(&server.endpoint)).unwrap();
    let output = run(&workspace_root(), "run_judgments", input).await;
    let capture = server.captures.try_recv();
    let count = server.finish().await;
    let capture = capture.unwrap();
    record_http("mixed-completion", &capture.body, count);
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["operation"], "run_judgments");
    let outcome = &wire["result"]["outcome"];
    assert_eq!(outcome["kind"], "completed");
    assert_eq!(outcome["answers"]["native_yes"]["yes_probability"], 0.75);
    assert_eq!(
        outcome["answers"]["native_choice"]["selected_label"],
        "alpha"
    );
    assert_eq!(outcome["answers"]["native_score"]["score"], 0.35);
    assert_eq!(
        outcome["receipt"]["resolved_model"],
        "typesafe/observed-model"
    );
    assert_eq!(outcome["receipt"]["usage"]["input_tokens"], 123);
    assert_eq!(outcome["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(count, 1);
    let provider: serde_json::Value = serde_json::from_slice(&capture.body).unwrap();
    assert!(provider.get("request_budget").is_none());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SYNTHETIC_PROCESS_CREDENTIAL"));
}

#[tokio::test]
async fn real_binary_distinguishes_invalid_request_provider_failure_and_invalid_response() {
    for (status, body, kind) in [
        (401, b"PROVIDER_ERROR_SENTINEL".to_vec(), "provider_failure"),
        (
            200,
            b"PROVIDER_BAD_JSON_SENTINEL".to_vec(),
            "invalid_response",
        ),
    ] {
        let mut server = TestServer::start(vec![HttpReply {
            status,
            body,
            chunked: false,
        }])
        .await;
        let input = serde_json::to_value(mixed_input(&server.endpoint)).unwrap();
        let output = run(&workspace_root(), "run_judgments", input).await;
        let capture = server.captures.try_recv().unwrap();
        let count = server.finish().await;
        record_http(kind, &capture.body, count);
        let wire = response(&output, 3);
        assert_eq!(wire["result"]["outcome"]["kind"], kind);
        assert!(wire["result"]["outcome"].get("answers").is_none());
        assert_eq!(count, 1);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("SENTINEL"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("SENTINEL"));
    }
    let server = TestServer::start(vec![]).await;
    let mut input = serde_json::to_value(mixed_input(&server.endpoint)).unwrap();
    input["request_budget"]["max_input_tokens"] = json!(1);
    let output = run(&workspace_root(), "run_judgments", input).await;
    assert_eq!(server.finish().await, 0);
    record_http("budget-refusal", &[], 0);
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid_request");
    assert!(wire["result"]["outcome"].get("attempts").is_none());
    for (operation, kind) in [
        ("prepare_evidence", "refused"),
        ("run_judgments", "invalid_request"),
    ] {
        let wire = response(&run(&workspace_root(), operation, json!({})).await, 2);
        assert_eq!(wire["kind"], "operation_result");
        assert_eq!(wire["result"]["operation"], operation);
        assert_eq!(wire["result"]["outcome"]["kind"], kind);
    }
}

#[tokio::test]
async fn real_binary_keeps_configured_provider_attempts_finite() {
    let mut server = TestServer::start(
        (0..3)
            .map(|_| HttpReply {
                status: 503,
                body: b"ERROR_SENTINEL".to_vec(),
                chunked: false,
            })
            .collect(),
    )
    .await;
    let mut input = mixed_input(&server.endpoint);
    input.attempts.retry_count = 2;
    input.attempts.initial_backoff_ms = 1;
    input.attempts.max_backoff_ms = 1;
    let output = run(
        &workspace_root(),
        "run_judgments",
        serde_json::to_value(input).unwrap(),
    )
    .await;
    let mut captures = Vec::new();
    while let Ok(capture) = server.captures.try_recv() {
        captures.push(capture);
    }
    let count = server.finish().await;
    for (index, capture) in captures.iter().enumerate() {
        record_http(&format!("finite-attempt-{index}"), &capture.body, count);
    }
    assert_eq!(captures.len(), count);
    let wire = response(&output, 3);
    assert_eq!(
        wire["result"]["outcome"]["attempts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(count, 3);
}
