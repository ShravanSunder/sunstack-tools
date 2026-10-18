#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_source_fixture.rs"]
mod source_fixture;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use serde_json::{Value, json};
use support::*;
use transport::{AnswerServer, ReplyMode, RouterReply};

fn assert_empty_diagnostic(outcome: &Value, root: &std::path::Path, file: &str) {
    let diagnostic = &outcome["diagnostics"]["items"][0];
    assert_eq!(diagnostic["code"], "empty_rule_file");
    assert_eq!(
        diagnostic["location"]["file"],
        json!(root.canonicalize().unwrap().join(file))
    );
    assert_eq!(diagnostic["location"]["line"], 1);
    assert_eq!(diagnostic["location"]["column"], 1);
}

#[tokio::test]
async fn empty_markdown_pair_is_check_refusal_and_runtime_source_skip_without_losing_valid_verdict()
{
    let (directory, input) = source_fixture::source_fixture();
    let root = directory.path();
    std::fs::write(
        root.join("missing-reviewed.md"),
        "U001: selected evidence\n",
    )
    .unwrap();
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let runtime = source_fixture::runtime_input(input.clone(), &server.endpoint);
    let baseline = response(&run(root, "evaluate_rules", runtime.clone()).await, 0);
    std::fs::write(root.join("empty.md"), "# No marked standards\n").unwrap();
    std::fs::write(root.join("empty.rules.json"), "{}\n").unwrap();
    let path = root.join("set.json");
    let mut header: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let empty_pair = json!({"markdown":"empty.md","rules":"empty.rules.json"});
    header["rule_sources"]
        .as_array_mut()
        .unwrap()
        .push(empty_pair.clone());
    std::fs::write(&path, serde_json::to_vec(&header).unwrap()).unwrap();
    let checked = response(
        &run_without_credential(root, "check_rules", input.clone()).await,
        2,
    );
    assert_eq!(checked["result"]["outcome"]["kind"], "refused");
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 1);
    assert_empty_diagnostic(&checked["result"]["outcome"], root, "empty.rules.json");
    let partial = response(&run(root, "evaluate_rules", runtime.clone()).await, 0);
    let report = &partial["result"]["outcome"]["report"];
    assert_eq!(report["rule_coverage"], "partial");
    assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
    assert_eq!(report["skipped_rules"][0]["kind"], "source");
    assert_eq!(
        report["skipped_rules"][0]["source_file"],
        json!(root.canonicalize().unwrap().join("empty.rules.json"))
    );
    assert_empty_diagnostic(&report["skipped_rules"][0], root, "empty.rules.json");
    assert_eq!(
        report["occurrences"],
        baseline["result"]["outcome"]["report"]["occurrences"]
    );
    assert_eq!(report["measurements"]["skipped_rule_sources"], 1);
    assert_eq!(report["measurements"]["invalid_rule_entries"], 0);
    header["rule_sources"] = json!([empty_pair]);
    std::fs::write(path, serde_json::to_vec(&header).unwrap()).unwrap();
    let checked = response(&run_without_credential(root, "check_rules", input).await, 2);
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 0);
    assert_empty_diagnostic(&checked["result"]["outcome"], root, "empty.rules.json");
    let empty = response(&run(root, "evaluate_rules", runtime).await, 3);
    assert_eq!(empty["result"]["outcome"]["kind"], "incomplete_report");
    let report = &empty["result"]["outcome"]["report"];
    assert_eq!(report["rule_coverage"], "none");
    assert_eq!(report["occurrences"], json!([]));
    assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
    assert_eq!(report["skipped_rules"][0]["kind"], "source");
    assert_empty_diagnostic(&report["skipped_rules"][0], root, "empty.rules.json");
    assert_eq!(
        server.finish().await.0,
        2,
        "only baseline and retained valid rule can dispatch"
    );
}

#[tokio::test]
async fn empty_jsonl_remains_a_strict_file_refusal_in_check_and_runtime() {
    let (directory, input) = source_fixture::source_fixture();
    let root = directory.path();
    let path = root.join("set.json");
    let mut header: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    header["rule_paths"] = json!(["empty.jsonl"]);
    std::fs::write(path, serde_json::to_vec(&header).unwrap()).unwrap();
    std::fs::write(root.join("empty.jsonl"), " \n").unwrap();
    let checked = response(
        &run_without_credential(root, "check_rules", input.clone()).await,
        2,
    );
    assert_eq!(checked["result"]["outcome"]["kind"], "refused");
    assert_empty_diagnostic(&checked["result"]["outcome"], root, "empty.jsonl");
    let runtime = source_fixture::runtime_input(input, "http://127.0.0.1:9/v1/systemone");
    let evaluated = response(&run(root, "evaluate_rules", runtime).await, 2);
    assert_eq!(evaluated["result"]["outcome"]["kind"], "invalid");
    assert_empty_diagnostic(&evaluated["result"]["outcome"], root, "empty.jsonl");
}
