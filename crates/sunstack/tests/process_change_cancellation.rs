#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/rule_transport.rs"]
mod rule_transport;
#[path = "../../sunstack-rules/tests/support/scope_fixture.rs"]
mod scope_fixture;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
use scope_fixture::{GATE_TEXT, ScopeFixture};
use serde_json::json;
use signal_process::*;
use support::*;
use tokio::io::AsyncWriteExt;
#[tokio::test]
async fn consumed_sigterm_at_a_received_change_scoped_gate_retains_pending_and_resolved_identity() {
    let fixture = ScopeFixture::new();
    let mut server = rule_transport::controlled::ControlledServer::start().await;
    let mut input = serde_json::to_value(fixture.add_gate(&server.endpoint)).unwrap();
    input["execution"]["max_concurrent_requests"] = json!(1);
    input["attempts"]["retry_count"] = json!(3);
    fixture.repository.write(
        "source.md",
        "# Prefix\n\nU001: original\n\nU002: neighbor\n",
    );
    let bytes = serde_json::to_vec(
        &json!({"protocol_version":1,"operation":"evaluate_rules","input":input}),
    )
    .unwrap();
    let (mut child, mut stderr, mut transcript) =
        ready_process(fixture.repository.root.path()).await;
    child.stdin.take().unwrap().write_all(&bytes).await.unwrap();
    let gate = server.next().await;
    assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
    assert_eq!(
        gate.body["questions"]["q000001"]["instructions"]["owner"],
        "rule_set_decision:0"
    );
    assert_eq!(gate.body["state"]["evidence"].as_object().unwrap().len(), 1);
    send_signal(&child, "-TERM").await;
    expect_line(
        &mut stderr,
        "sunstack: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let output = collect_output(child, stderr, transcript).await;
    record_output(fixture.repository.root.path(), &bytes, &output);
    let count = server.finish().await;
    let wire = response(&output, 4);
    assert_eq!(wire["result"]["outcome"]["kind"], "cancelled");
    let progress = &wire["result"]["outcome"]["progress"];
    assert_eq!(progress["kind"], "report_admitted");
    let report = &progress["partial_report"];
    assert_eq!(report["change_selections"].as_array().unwrap().len(), 1);
    assert_eq!(report["change_selections"][0]["repository"], "repo");
    let after = &report["change_selections"][0]["members"][0]["changed_paths"][0]["after"];
    assert_eq!(after["path"], "source.md");
    assert_eq!(after["revision"]["side"], "after");
    assert!(
        report["captures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|capture| &capture["source"] == after)
    );
    for occurrence in report["occurrences"].as_array().unwrap() {
        assert!(
            matches!(
                occurrence["kind"].as_str(),
                Some("cancelled" | "not_started")
            ),
            "{occurrence}"
        );
        assert_eq!(
            occurrence["selection"],
            json!({"kind":"comparison_pending"})
        );
        assert!(occurrence.get("judgment").is_none());
        assert!(occurrence.get("selected_by").is_none());
    }
    assert_eq!(report["measurements"]["bytes_scanned"], GATE_TEXT.len());
    assert_eq!(report["measurements"]["model_requests"], 1);
    assert_eq!(report["measurements"]["inference_attempts"], 1);
    let sent = report["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|request| request["status"]["kind"] == "sent")
        .collect::<Vec<_>>();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["status"]["outcome"]["kind"], "cancelled");
    assert_eq!(
        sent[0]["status"]["outcome"]["attempts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(count, 1);
    record_observation(
        "m2-scoped-cancellation",
        json!({"sequence":["ready","full_gate_body","signal_consumed","child_joined","server_joined"],"http_requests":count,"wire":wire,"gate_body":gate.body}),
    );
}
