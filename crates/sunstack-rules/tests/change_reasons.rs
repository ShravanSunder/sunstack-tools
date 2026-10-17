#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/scope_fixture.rs"]
mod scope_fixture;
#[path = "support/decision_http.rs"]
mod transport;
use scope_fixture::{ScopeFixture, fixture};
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;
fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
#[tokio::test]
async fn context_content_change_selects_the_unchanged_target_with_only_its_context_reason() {
    let fixture = ScopeFixture::new();
    let mut rule = fixture::rule("changed");
    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"subject","requirement":"required","select":{"kind":"paragraph","target_id":"U002"}}));
    fixture.repository.write("changed.jsonl", &rule.to_string());
    fixture.repository.stage("changed.jsonl");
    fixture.repository.commit();
    fixture
        .repository
        .write("source.md", "U001: original\n\nU002: new context\n");
    let mut server = controlled::ControlledServer::start().await;
    let task = tokio::spawn(evaluate_rules(
        fixture.input(&server.endpoint),
        context(&server.endpoint),
    ));
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 2);
    let state = &request.body["state"];
    let attribution = &state["evidence"]["rule:changed/context"];
    assert_eq!(
        state["contents"][attribution["content"].as_str().unwrap()],
        "U002: new context"
    );
    request.noul(0.9);
    let RuleInvocationOutcome::CompleteReport { report } = task.await.unwrap() else {
        panic!("report")
    };
    let report: Value = serde_json::to_value(report).unwrap();
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["context_changed"])
    );
    assert_eq!(server.finish().await, 1);
}
#[tokio::test]
async fn unknown_before_sides_conservatively_select_and_judge_the_current_side() {
    for history in [false, true] {
        let fixture = ScopeFixture::new();
        fixture
            .repository
            .write("source.md", "U001: current\n\nU002: neighbor\n");
        fixture.repository.stage("source.md");
        fixture.repository.commit();
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture.input(&server.endpoint);
        if history {
            input.repositories[0].change_scope = ChangeScope::RecentCommits {
                reference: "HEAD".into(),
                window: TimeWindow::Absolute {
                    since_utc: chrono::DateTime::from_timestamp(0, 0).unwrap(),
                    until_utc: chrono::DateTime::from_timestamp(2000, 0).unwrap(),
                },
                parent_policy: CommitParentPolicy::FirstParent,
            };
            let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
                panic!("discovery")
            };
            limits.max_history_commits = 1.try_into().unwrap();
        } else {
            input.repositories[0].change_scope = ChangeScope::RevisionDiff {
                base_revision: "missing-baseline".into(),
                head: RevisionHead::Worktree {},
            };
        }
        let prepared = prepare_rules(input.clone(), context(&server.endpoint)).await;
        let RulePreparationOutcome::Prepared { preparation } = prepared else {
            panic!("{prepared:?}")
        };
        let value = serde_json::to_value(preparation).unwrap();
        let expected = if history {
            "before_analysis_unknown"
        } else {
            "baseline_unavailable"
        };
        assert_eq!(
            value["occurrences"][0]["status"]["selection"]["selected_by"],
            json!([expected])
        );
        let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
        let request = server.next().await;
        assert_eq!(request.body["questions"].as_object().unwrap().len(), 2);
        request.noul(0.9);
        let outcome = task.await.unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{history}: {outcome:?}")
        };
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["occurrences"][0]["selected_by"], json!([expected]));
        assert_eq!(server.finish().await, 1);
    }
}
