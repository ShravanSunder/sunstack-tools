#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn empty_thinking_counts_and_keeps_its_line_without_entering_the_judge_request() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let lines = [
        json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":"public first"}]}}),
        json!({"type":"assistant","message":{"id":"stream","content":[{"type":"thinking","thinking":""}]}}),
        json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":"MATCH public last"}]}}),
    ].map(|line| line.to_string()+"\n");
    std::fs::write(root.path().join("first.jsonl"), lines.concat()).unwrap();
    let rules = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules).unwrap()).unwrap();
    rule["mechanical_pass"]["predicate"]["field"] = json!("/body/blocks/2/block/text");
    std::fs::write(rules, rule.to_string()).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = tokio::select! {
        result = &mut run => panic!("empty-thinking stream did not reach judge: {}", String::from_utf8_lossy(&result.unwrap().stdout)),
        request = server.next() => request,
    };
    let body = request.body.to_string();
    assert!(body.contains("public first"));
    assert!(body.contains("MATCH public last"));
    assert!(
        !body.contains("thinking"),
        "private empty block reached the request: {body}"
    );
    request.noul(0.9);
    let output = run.await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["candidates"][0]["outcome"]["kind"], "finding");
    let occurrence = &report["candidates"][0]["occurrence"];
    assert_eq!(
        occurrence["positions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|position| position["line"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(occurrence["matched_at"]["line"], 3);
    assert_eq!(
        occurrence["matched_at"]["byte_offset"],
        (lines[0].len() + lines[1].len()) as u64
    );
    assert!(report["inputs"][0]["unfit"].as_array().unwrap().is_empty());
    assert!(
        report["privacy"]["left_out"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count == &json!({"route":"records","private_kind":"thinking","count":1}))
    );
    assert_eq!(server.finish().await, 1);
}
