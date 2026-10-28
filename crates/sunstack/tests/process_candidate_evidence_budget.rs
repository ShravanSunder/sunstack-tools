#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
mod support;
use controlled_http::ControlledServer;
use evidence_fixture::{fixture, report};
use serde_json::json;

#[tokio::test]
async fn captures_share_the_cumulative_limit_and_reuse_one_outside_document() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, true);
    std::fs::write(root.path().join("owner.md"), "SHARED_CAPTURE_FACT\n").unwrap();
    std::fs::copy(
        root.path().join("first.jsonl"),
        root.path().join("second.jsonl"),
    )
    .unwrap();
    input["sources"].as_array_mut().unwrap().push(
        json!({"source":{"kind":"session_pair","provider":"claude_code","session":"second"}}),
    );
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}});
    let transcript_bytes = 2 * std::fs::read(root.path().join("first.jsonl"))
        .unwrap()
        .len();
    let fact_bytes = std::fs::read(root.path().join("owner.md")).unwrap().len();
    // The exact combined byte budget fits only when both candidates reuse the fact capture.
    input["limits"]["max_total_bytes"] = json!(transcript_bytes + fact_bytes);
    let good = input.clone();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", good).await });
    for _ in 0..2 {
        let request = tokio::select! {
            output = &mut run => panic!("shared capture failed: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request = server.next() => request,
        };
        assert!(request.body.to_string().contains("SHARED_CAPTURE_FACT"));
        request.noul(0.9);
    }
    let result = report(&run.await.unwrap(), 0);
    assert!(
        result["report"]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["outcome"]["kind"] == "finding")
    );
    // The transcript captures spent every byte; another session must not reset the budget.
    input["limits"]["max_total_bytes"] = json!(transcript_bytes);
    let result = report(
        &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
        0,
    );
    let entries = result["report"]["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry["outcome"]["kind"], "unresolved");
        assert_eq!(entry["outcome"]["missing"]["diagnostic"], "evidence_limit");
    }
    assert_eq!(result["report"]["usage"]["requests"], 0);
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn old_evaluate_rules_still_refuses_a_missing_declared_fact() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let candidate = fixture(root.path(), &server.endpoint, false);
    let mut input = json!({"definition_root":root.path(),"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"conversation","transcript_path":root.path().join("first.jsonl"),"format":"claude_code_jsonl"}}}]}]},"origin":{"kind":"direct"},"cursors":[],"discovery":{"kind":"none"}});
    for name in [
        "repositories",
        "payload_schemas",
        "execution",
        "limits",
        "backend",
        "attempts",
        "request_budget",
    ] {
        input[name] = candidate[name].clone();
    }
    let outcome = report(
        &support::run_without_credential(root.path(), "evaluate_rules", input).await,
        2,
    );
    assert_eq!(outcome["kind"], "invalid");
    assert!(
        outcome["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "unbound_input")
    );
    assert_eq!(server.finish().await, 0);
}
