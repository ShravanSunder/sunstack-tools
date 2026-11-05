#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}

#[tokio::test]
async fn full_scope_preparation_and_real_http_report_expose_selection_without_resolving_git() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    let mut server = controlled::ControlledServer::start().await;
    let input = fixture::input(root.path(), &server.endpoint);
    let prepared = prepare_rules(input.clone(), context(&server.endpoint)).await;
    let RulePreparationOutcome::Prepared { preparation } = prepared else {
        panic!("{prepared:?}")
    };
    let prepared = serde_json::to_value(preparation).unwrap();
    assert_eq!(
        prepared["occurrences"][0]["status"],
        json!({"kind":"planned","selection":{"kind":"selected","selected_by":["full_scope"]}})
    );
    assert_eq!(prepared["change_selections"], json!([]));
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let request = server.next().await;
    assert_eq!(
        request.body["state"]["evidence"]["rule:one/target"]["coverage"]["kind"],
        "single_source"
    );
    request.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let report: Value = serde_json::to_value(report).unwrap();
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["full_scope"])
    );
    assert_eq!(report["change_selections"], json!([]));
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn payload_context_keeps_its_authored_input_and_group_in_single_source_coverage() {
    let root = tempfile::tempdir().unwrap();
    let mut rule = fixture::rule("one");
    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"flags","input":"flags","requirement":"required","select":{"kind":"whole_payload"}}));
    fixture::write_set(root.path(), &[rule], vec![]);
    let mut set: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    set["inputs"]["flags"] = json!({"kind":"payload","schema":"flags"});
    std::fs::write(root.path().join("set.json"), set.to_string()).unwrap();
    std::fs::write(root.path().join("flags.schema.json"),r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"],"additionalProperties":false}"#).unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut value = serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
    value["payload_schemas"] = json!([{"name":"flags","path":"flags.schema.json"}]);
    let groups = value["definitions"]["rule_sets"][0]["binding_groups"]
        .as_array_mut()
        .unwrap();
    let mut second = groups[0].clone();
    groups[0]["bindings"]["flags"] = json!({"kind":"payload","value":{"enabled":true}});
    second["bindings"]["flags"] = json!({"kind":"payload","value":{"enabled":false}});
    groups.push(second);
    let task = tokio::spawn(evaluate_rules(
        serde_json::from_value(value).unwrap(),
        context(&server.endpoint),
    ));
    let mut observed = Vec::new();
    for _ in 0..2 {
        let request = server.next().await;
        let attribution = &request.body["state"]["evidence"]["rule:one/flags"];
        assert_eq!(attribution["input"], "flags");
        let capture = &attribution["coverage"]["capture"];
        assert_eq!(attribution["coverage"]["kind"], "single_source");
        assert_eq!(capture["source"]["input"], "flags");
        let content = &request.body["state"]["contents"][attribution["content"].as_str().unwrap()];
        observed.push((
            capture["source"]["group_position"].as_u64().unwrap(),
            content["enabled"].as_bool().unwrap(),
        ));
        request.noul(0.9);
    }
    observed.sort();
    assert_eq!(observed, vec![(0, true), (1, false)]);
    assert!(matches!(
        task.await.unwrap(),
        RuleInvocationOutcome::CompleteReport { .. }
    ));
    assert_eq!(server.finish().await, 2);
}
