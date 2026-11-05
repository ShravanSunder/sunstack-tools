#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::json;
use std::sync::Arc;
use sunstack_decisions::{CancellationToken, DecisionRunOutcome, JevOpenRouterBackend};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::test_support::rule_fixture as fixture;
use sunstack_rules::*;
const ENDPOINT: &str = "https://openrouter.ai/api/v1/systemone";
fn context(input: &RuleInvocationInput) -> RuleExecutionContext {
    let key = std::env::var("OPENROUTER_API_KEY").expect("hosted credential not supplied");
    RuleExecutionContext {
        backend: Arc::new(JevOpenRouterBackend::new(input.backend.clone(), &key).unwrap()),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
fn hosted_settings(input: &mut RuleInvocationInput) {
    input.attempts.attempt_timeout_ms = 60_000;
    input.attempts.retry_count = 0;
    input.request_budget.max_requests = 3;
}
fn receipts(report: &RuleInvocationReport) {
    let mut completed = 0;
    for request in &report.requests {
        if let RequestStatus::Sent {
            outcome:
                DecisionRunOutcome::Completed {
                    receipt, attempts, ..
                },
        } = &request.status
        {
            assert_eq!(attempts.len(), 1);
            assert!(receipt.usage.input_tokens > 0);
            println!(
                "hosted rules receipt: {}",
                serde_json::to_string(receipt).unwrap()
            );
            completed += 1;
        }
    }
    assert!(completed > 0);
}
#[tokio::test]
#[ignore = "hosted Jev applicability requires OPENROUTER_API_KEY"]
async fn hosted_applicability_decision_preserves_native_answer_and_receipt_through_rules() {
    let root = tempfile::tempdir().unwrap();
    let mut gate = fixture::decision("subject");
    gate["question"]["instructions"] =
        json!("Does the supplied source contain the words original content?");
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![gate]);
    let mut input = fixture::input(root.path(), ENDPOINT);
    hosted_settings(&mut input);
    let outcome = evaluate_rules(input.clone(), context(&input)).await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("hosted applicability did not complete: {outcome:?}")
    };
    let EntryResult::Decision { outcome, .. } = &report.groups[0].entries[0].result else {
        panic!("{report:?}")
    };
    let reference = match outcome {
        DecisionEntryOutcome::Passed { answer } | DecisionEntryOutcome::DidNotPass { answer } => {
            answer
        }
        other => panic!("hosted decision did not admit: {other:?}"),
    };
    assert!(matches!(
        report.answer(reference),
        Some(sunstack_decisions::JudgmentAnswer::Noul { .. })
    ));
    receipts(&report);
}
#[tokio::test]
#[ignore = "hosted Jev routing requires OPENROUTER_API_KEY"]
async fn hosted_router_preserves_native_choice_confidence_and_source_linked_receipt() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    std::fs::write(root.path().join("router.json"), serde_json::to_vec(&json!({"schema_version":1,"name":"hosted","description":"real choice admission","inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},"evidence":[fixture::declaration("subject","router_context")],"question":{"type":"choice","instructions":"Select inspect when the supplied text contains original content; otherwise select none.","criteria":{"inspect":"The source says original content.","none":"The source does not say original content."}},"routes":{"inspect":[{"rule_set":"set.json","inputs":{"subject":"subject","other":"other"}}]},"confidence_floor":0.6})).unwrap()).unwrap();
    let mut input = serde_json::to_value(fixture::input(root.path(), ENDPOINT)).unwrap();
    let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
    input["definitions"] = json!({"kind":"router","path":"router.json","binding_groups":[group]});
    let mut input: RuleInvocationInput = serde_json::from_value(input).unwrap();
    hosted_settings(&mut input);
    let outcome = evaluate_rules(input.clone(), context(&input)).await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("hosted routing did not complete: {outcome:?}")
    };
    let RoutingReport::Router { groups } = &report.routing else {
        panic!("{report:?}")
    };
    let reference = match &groups[0].outcome {
        RouterOutcome::Routed { answer, .. }
        | RouterOutcome::UncertainIntent { answer, .. }
        | RouterOutcome::NoRoute { answer, .. } => answer,
        other => panic!("hosted router did not admit: {other:?}"),
    };
    assert!(matches!(
        report.answer(reference),
        Some(sunstack_decisions::JudgmentAnswer::Choice { .. })
    ));
    receipts(&report);
}
