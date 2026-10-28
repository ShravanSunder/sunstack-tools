#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn sent_candidates_have_explicit_elapsed_time_and_local_only_candidates_do_not() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    std::fs::write(
        root.path().join("first.jsonl"),
        format!(
            "{}\n",
            json!({"type":"assistant","message":{"content":"MATCH timing"}})
        ),
    )
    .unwrap();
    let directory = root.path().to_owned();
    let model_input = input.clone();
    let run =
        tokio::spawn(
            async move { support::run(&directory, "evaluate_candidates", model_input).await },
        );
    server.next().await.noul(0.9);
    let output = run.await.unwrap();
    assert_eq!(output.status.code(), Some(0));
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        wire["result"]["outcome"]["report"]["candidates"][0]["elapsed_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(server.finish().await, 1);
    let rules = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules).unwrap()).unwrap();
    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"NEVER_IN_THE_CANDIDATE"},"max_scan_bytes":100000}}]);
    std::fs::write(rules, rule.to_string()).unwrap();
    let output = support::run_without_credential(root.path(), "evaluate_candidates", input).await;
    assert_eq!(output.status.code(), Some(0));
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let candidate = &wire["result"]["outcome"]["report"]["candidates"][0];
    assert_eq!(candidate["outcome"]["kind"], "dropped");
    assert!(candidate.get("elapsed_ms").is_none());
}
