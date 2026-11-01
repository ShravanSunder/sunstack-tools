#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_decisions::test_support as decision_fixture;
use sunstack_rules::test_support::rule_fixture;
#[path = "../support/rule_transport.rs"]
mod rule_transport;
#[path = "../support/mod.rs"]
mod support;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::json;
use support::*;

#[tokio::test]
async fn prepare_rules_is_a_real_operation_with_prepared_native_evidence_and_no_dispatch() {
    let root = tempfile::tempdir().unwrap();
    rule_fixture::write_set(root.path(), &[rule_fixture::rule("one")], vec![]);
    let server = decision_fixture::TestServer::start(vec![]).await;
    let input = serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
    let output = run(root.path(), "prepare_rules", input).await;
    let count = server.finish().await;
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["operation"], "prepare_rules");
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    assert_eq!(
        wire["result"]["outcome"]["preparation"]["planned_requests"][0]["readiness"]["kind"],
        "prepared"
    );
    assert_eq!(
        wire["result"]["outcome"]["preparation"]["change_selections"],
        json!([])
    );
    assert_eq!(
        wire["result"]["outcome"]["preparation"]["occurrences"][0]["status"],
        json!({"kind":"planned","selection":{"kind":"selected","selected_by":["full_scope"]}})
    );
    assert_eq!(count, 0);
}

#[tokio::test]
async fn prepared_rule_requests_do_not_dispatch_and_completed_error_failures_exit_zero() {
    let root = tempfile::tempdir().unwrap();
    let mut rule = rule_fixture::rule("error-rule");
    rule["severity"] = json!("error");
    rule["verdict"]["error_at_or_below"] = json!(0.3);
    rule_fixture::write_set(root.path(), &[rule], vec![]);
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.1,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let positive = run(
        root.path(),
        "run_judgments",
        serde_json::to_value(decision_fixture::mixed_input(&server.endpoint)).unwrap(),
    )
    .await;
    response(&positive, 0);
    assert_eq!(server.count(), 1);
    let input = serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
    let prepared = response(&run(root.path(), "prepare_rules", input.clone()).await, 0);
    assert_eq!(
        server.count(),
        1,
        "preparation dispatched despite a working observer"
    );
    assert_eq!(
        prepared["result"]["outcome"]["preparation"]["measurements"]["model_requests"],
        0
    );
    let evaluated = response(&run(root.path(), "evaluate_rules", input).await, 0);
    assert_eq!(evaluated["result"]["outcome"]["kind"], "complete_report");
    assert_eq!(
        evaluated["result"]["outcome"]["report"]["change_selections"],
        json!([])
    );
    assert_eq!(
        evaluated["result"]["outcome"]["report"]["occurrences"][0]["selected_by"],
        json!(["full_scope"])
    );
    let judgment = &evaluated["result"]["outcome"]["report"]["occurrences"][0]["judgment"];
    assert_eq!(judgment["declared_severity"], "error");
    assert_eq!(
        judgment["verdict"]["result"],
        json!({"kind":"fail","activation":{"kind":"error"}})
    );
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 2);
    record_observation(
        "rules-error-failure",
        json!({"http_requests":count,"bodies":bodies,"report":evaluated}),
    );
}

#[tokio::test]
async fn rules_operations_preserve_native_invalid_and_selection_failure_outcomes() {
    let root = tempfile::tempdir().unwrap();
    let mut missing = rule_fixture::rule("missing");
    missing["evidence"][0]["select"]["target_id"] = json!("U999");
    rule_fixture::write_set(root.path(), &[missing], vec![]);
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let input = serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
    for (operation, failed) in [
        ("prepare_rules", "prepared_with_failures"),
        ("evaluate_rules", "incomplete_report"),
    ] {
        let wire = response(&run(root.path(), operation, input.clone()).await, 3);
        assert_eq!(wire["result"]["outcome"]["kind"], failed);
        let malformed = response(
            &run(root.path(), operation, json!({"AUTHOR_SECRET_KEY":true})).await,
            2,
        );
        assert_eq!(malformed["kind"], "operation_result");
        assert_eq!(malformed["result"]["operation"], operation);
        assert_eq!(malformed["result"]["outcome"]["kind"], "invalid");
        assert!(!malformed.to_string().contains("AUTHOR_SECRET_KEY"));
    }
    assert_eq!(server.finish().await.0, 0);
}

#[tokio::test]
async fn evaluation_provider_failures_and_invalid_answers_never_create_verdicts() {
    let root = tempfile::tempdir().unwrap();
    rule_fixture::write_set(root.path(), &[rule_fixture::rule("one")], vec![]);
    for mode in [
        ReplyMode::ProviderFailure { status: 401 },
        ReplyMode::InvalidResponse,
    ] {
        let server = AnswerServer::start(mode).await;
        let input =
            serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
        let wire = response(&run(root.path(), "evaluate_rules", input).await, 3);
        assert_eq!(server.finish().await.0, 1);
        assert_eq!(wire["result"]["outcome"]["kind"], "incomplete_report");
        let occurrence = &wire["result"]["outcome"]["report"]["occurrences"][0];
        assert_eq!(occurrence["kind"], "evaluation_failed");
        assert_eq!(
            occurrence["selection"],
            json!({"kind":"selected","selected_by":["full_scope"]})
        );
        assert_eq!(
            wire["result"]["outcome"]["report"]["change_selections"],
            json!([])
        );
        assert!(occurrence.get("judgment").is_none());
    }
}
