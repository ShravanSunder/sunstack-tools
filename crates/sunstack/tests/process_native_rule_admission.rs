#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/change_fixture.rs"]
mod changes;
#[path = "../../sunstack-decisions/tests/support/mod.rs"]
mod decision_fixture;
#[path = "../../sunstack-rules/tests/support/rule_fixture.rs"]
mod rule_fixture;
#[path = "support/rule_transport.rs"]
mod rule_transport;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use support::*;

fn input(root: &std::path::Path, endpoint: &str) -> Value {
    serde_json::to_value(rule_fixture::input(root, endpoint)).unwrap()
}
fn check_input(mut input: Value) -> Value {
    for key in [
        "package_bindings",
        "backend",
        "attempts",
        "request_budget",
        "execution",
        "origin",
        "cursors",
    ] {
        input.as_object_mut().unwrap().remove(key);
    }
    input["output"] = json!("assembled_rules");
    input
}
#[tokio::test]
async fn invalid_connected_jsonl_selector_refuses_prepare_and_evaluate_without_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    response(
        &run(
            root.path(),
            "run_judgments",
            serde_json::to_value(decision_fixture::mixed_input(&server.endpoint)).unwrap(),
        )
        .await,
        0,
    );
    assert_eq!(server.count(), 1);
    let mut rule = rule_fixture::rule("bad.jsonl");
    rule["evidence"][0]["input"] = json!("changes");
    rule["evidence"][0]["select"] = json!({"kind":"changed_units","languages":[],"unit_kinds":["function"],"source_sides":"after_only"});
    rule_fixture::write_set(root.path(), &[rule], vec![]);
    let mut header: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    header["inputs"]["changes"] = json!({"kind":"change_set"});
    std::fs::write(
        root.path().join("set.json"),
        serde_json::to_vec(&header).unwrap(),
    )
    .unwrap();
    let mut input = input(root.path(), &server.endpoint);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["changes"] =
        json!({"kind":"change_set","repository":"repo"});
    input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
    for operation in ["prepare_rules", "evaluate_rules"] {
        let before = server.count();
        let refused = response(&run(root.path(), operation, input.clone()).await, 2);
        assert_eq!(refused["result"]["outcome"]["kind"], "invalid");
        assert_eq!(server.count(), before);
    }
    assert_eq!(server.finish().await.0, 1);
}
#[tokio::test]
async fn connected_native_check_requires_discovery_without_acquiring_bound_documents() {
    let root = tempfile::tempdir().unwrap();
    let mut rule = rule_fixture::rule("connected.valid");
    rule["evidence"][0]["input"] = json!("changes");
    rule["evidence"][0]["select"] = json!({"kind":"changed_units","languages":["typescript"],"unit_kinds":["function"],"source_sides":"after_only"});
    rule_fixture::write_set(root.path(), &[rule], vec![]);
    let mut header: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    header["inputs"]["changes"] = json!({"kind":"change_set"});
    std::fs::write(
        root.path().join("set.json"),
        serde_json::to_vec(&header).unwrap(),
    )
    .unwrap();
    let mut input = check_input(input(root.path(), "not-a-url"));
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["subject"]["path"] =
        json!("missing-reviewed.md");
    input["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"**"}]);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["changes"] =
        json!({"kind":"change_set","repository":"repo"});
    input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
    assert!(!root.path().join("missing-reviewed.md").exists());
    assert!(!root.path().join(".git").exists());
    let output = run_without_credential(root.path(), "check_rules", input.clone()).await;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let checked = response(&output, 0);
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 1);
    assert_eq!(
        checked["result"]["outcome"]["printed_rules"][0]["definition"]["evidence"][0]["select"]["kind"],
        "changed_units"
    );
    input.as_object_mut().unwrap().remove("discovery");
    let refused = response(
        &run_without_credential(root.path(), "check_rules", input).await,
        2,
    );
    let diagnostic = &refused["result"]["outcome"]["diagnostics"]["items"][0];
    assert_eq!(diagnostic["code"], "discovery_not_enabled");
    assert_eq!(
        diagnostic["location"]["file"],
        json!(root.path().canonicalize().unwrap().join("rules.jsonl"))
    );
    assert_eq!(diagnostic["location"]["line"], 1);
}
#[tokio::test]
async fn malformed_and_empty_jsonl_keep_strict_file_refusals() {
    for (content, code) in [("{\n", "malformed_json"), (" \n", "empty_rule_file")] {
        let root = tempfile::tempdir().unwrap();
        rule_fixture::write_set(root.path(), &[rule_fixture::rule("one")], vec![]);
        std::fs::write(root.path().join("rules.jsonl"), content).unwrap();
        let runtime = input(root.path(), "http://127.0.0.1:9/v1/systemone");
        for (operation, input, kind) in [
            ("check_rules", check_input(runtime.clone()), "refused"),
            ("evaluate_rules", runtime, "invalid"),
        ] {
            let refused = response(
                &run_without_credential(root.path(), operation, input).await,
                2,
            );
            assert_eq!(refused["result"]["outcome"]["kind"], kind);
            assert!(
                refused["result"]["outcome"]["diagnostics"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["code"] == code
                        && item["location"]["file"]
                            .as_str()
                            .is_some_and(|path| path.ends_with("rules.jsonl")))
            );
        }
    }
}
#[tokio::test]
async fn native_check_cancels_after_observed_signal_without_a_credential() {
    use signal_process::{
        collect_output, expect_line, ready_process_without_credential, send_signal,
    };
    use tokio::io::AsyncWriteExt;
    let directory = tempfile::tempdir().unwrap();
    rule_fixture::write_set(directory.path(), &[rule_fixture::rule("one")], vec![]);
    let input = check_input(input(directory.path(), "not-a-url"));
    let (mut process, mut stderr, mut transcript) =
        ready_process_without_credential(directory.path()).await;
    send_signal(&process, "-TERM").await;
    expect_line(
        &mut stderr,
        "sunstack: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let bytes =
        serde_json::to_vec(&json!({"protocol_version":1,"operation":"check_rules","input":input}))
            .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    stdin.write_all(&bytes).await.unwrap();
    drop(stdin);
    let output = collect_output(process, stderr, transcript).await;
    record_output(directory.path(), &bytes, &output);
    let wire = response(&output, 4);
    assert_eq!(wire["result"]["outcome"]["kind"], "cancelled");
    assert_eq!(
        wire["result"]["outcome"]["measurements"]["model_requests"],
        0
    );
}
