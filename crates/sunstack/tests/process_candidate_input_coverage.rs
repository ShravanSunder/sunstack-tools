#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
mod support;
use serde_json::{Value, json};

#[tokio::test]
async fn input_reports_distinguish_complete_capture_incomplete_tail_and_undiscovered_input() {
    for (mode, exit, expected_capture, expected_discovery) in [
        ("complete", 0, "complete", "completed"),
        ("tail", 3, "incomplete", "completed"),
        ("missing", 3, "incomplete", "not_started"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut input = candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
        input["sources"].as_array_mut().unwrap().truncate(1);
        let record =
            json!({"type":"assistant","message":{"id":"one","content":"ordinary source record"}})
                .to_string();
        let text = if mode == "tail" {
            format!("{record}\n{record}")
        } else {
            format!("{record}\n")
        };
        if mode != "missing" {
            std::fs::write(root.path().join("first.jsonl"), text).unwrap();
        }
        let output =
            support::run_without_credential(root.path(), "evaluate_candidates", input).await;
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        let source = &report["inputs"][0];
        assert_eq!(source["capture"], expected_capture);
        assert_eq!(source["discovery"].as_array().unwrap().len(), 1);
        assert_eq!(source["discovery"][0]["rule"]["rule_id"], "candidate.one");
        assert_eq!(source["discovery"][0]["status"], expected_discovery);
        assert!(source["unfit"].as_array().unwrap().is_empty());
        assert_eq!(report["usage"]["requests"], 0);
        if mode != "complete" {
            assert_eq!(report["completeness"]["kind"], "partial");
        }
    }
}
