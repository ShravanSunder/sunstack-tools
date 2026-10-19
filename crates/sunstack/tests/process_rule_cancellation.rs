#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/rule_fixture.rs"]
mod rule_fixture;
#[path = "support/rule_transport.rs"]
mod rule_transport;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
use serde_json::json;
use signal_process::*;
use support::*;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn rules_cancellation_before_admission_never_fabricates_preparation_or_report() {
    let root = tempfile::tempdir().unwrap();
    rule_fixture::write_set(root.path(), &[rule_fixture::rule("one")], vec![]);
    for (operation, progress) in [
        ("prepare_rules", "before_preparation_admission"),
        ("evaluate_rules", "before_report_admission"),
    ] {
        let server = rule_transport::AnswerServer::start(rule_transport::ReplyMode::Answers {
            probability: 0.9,
            router: rule_transport::RouterReply::ReviewHandback,
        })
        .await;
        let input =
            serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
        let bytes =
            serde_json::to_vec(&json!({"protocol_version":1,"operation":operation,"input":input}))
                .unwrap();
        let (mut process, mut stderr, mut transcript) = ready_process(root.path()).await;
        send_signal(&process, "-TERM").await;
        expect_line(
            &mut stderr,
            "sunstack-rule: cancellation observed\n",
            &mut transcript,
        )
        .await;
        process
            .stdin
            .take()
            .unwrap()
            .write_all(&bytes)
            .await
            .unwrap();
        let output = collect_output(process, stderr, transcript).await;
        record_output(root.path(), &bytes, &output);
        let wire = response(&output, 4);
        assert_eq!(wire["result"]["outcome"]["kind"], "cancelled");
        assert_eq!(wire["result"]["outcome"]["progress"]["kind"], progress);
        assert!(wire["result"]["outcome"].get("report").is_none());
        assert!(wire["result"]["outcome"].get("preparation").is_none());
        assert_eq!(server.finish().await.0, 0);
    }
}

#[tokio::test]
async fn sigterm_after_causal_peer_admission_retains_its_verdict_receipt_and_queued_work() {
    let root = tempfile::tempdir().unwrap();
    rule_fixture::write_set(root.path(), &[rule_fixture::rule("one")], vec![]);
    for group in 0..3 {
        std::fs::write(
            root.path().join(format!("group{group}.md")),
            format!("U001: GROUP_{group}\n"),
        )
        .unwrap();
    }
    let mut server = rule_transport::controlled::ControlledServer::start().await;
    let mut input =
        serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
    let base = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
    input["definitions"]["rule_sets"][0]["binding_groups"] = json!(
        (0..3)
            .map(|group| {
                let mut binding = base.clone();
                binding["bindings"]["subject"]["path"] = json!(format!("group{group}.md"));
                binding
            })
            .collect::<Vec<_>>()
    );
    input["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"*.md"}]);
    input["execution"]["max_concurrent_requests"] = json!(1);
    input["attempts"]["retry_count"] = json!(3);
    let bytes = serde_json::to_vec(
        &json!({"protocol_version":1,"operation":"evaluate_rules","input":input}),
    )
    .unwrap();
    let (mut process, mut stderr, mut transcript) = ready_process(root.path()).await;
    process
        .stdin
        .take()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
    let first = server.next().await;
    assert!(first.body["state"].to_string().contains("GROUP_0"));
    first.noul(0.9);
    // One permit means the next full body is causal evidence that the first
    // native completion was admitted and its owned request task settled.
    let second = server.next().await;
    assert!(second.body["state"].to_string().contains("GROUP_1"));
    send_signal(&process, "-TERM").await;
    expect_line(
        &mut stderr,
        "sunstack-rule: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let output = collect_output(process, stderr, transcript).await;
    record_output(root.path(), &bytes, &output);
    let count = server.finish().await;
    let wire = response(&output, 4);
    let progress = &wire["result"]["outcome"]["progress"];
    assert_eq!(progress["kind"], "report_admitted");
    let report = &progress["partial_report"];
    assert_eq!(report["measurements"]["files_read"], 2);
    assert_eq!(report["measurements"]["model_requests"], 2);
    assert_eq!(report["occurrences"][0]["kind"], "judged");
    assert_eq!(report["change_selections"], json!([]));
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["full_scope"])
    );
    for occurrence in &report["occurrences"].as_array().unwrap()[1..] {
        assert_eq!(
            occurrence["selection"],
            json!({"kind":"selected","selected_by":["full_scope"]})
        );
    }
    assert_eq!(report["occurrences"][1]["kind"], "cancelled");
    assert_eq!(report["occurrences"][2]["kind"], "not_started");
    assert_eq!(report["occurrences"][2]["reason"], "caller_cancelled");
    assert_eq!(
        report["requests"][0]["status"]["outcome"]["kind"],
        "completed"
    );
    assert_eq!(
        report["requests"][0]["status"]["outcome"]["receipt"]["usage"]["cost_usd"],
        0.001
    );
    assert_eq!(
        report["requests"][1]["status"]["outcome"]["kind"],
        "cancelled"
    );
    assert_eq!(
        report["requests"][1]["status"]["outcome"]["attempts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(count, 2);
    record_observation(
        "rules-causal-cancellation",
        json!({"observed_sequence":["ready","first_full_request","second_full_request_after_native_completion","signal_consumed","child_joined"],"http_requests":count,"report":report}),
    );
}
