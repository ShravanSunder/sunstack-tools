#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use support::*;

#[tokio::test]
async fn raw_protocol_errors_never_fabricate_an_operation_result() {
    for (bytes, code) in [
        (b"".as_slice(), "malformed_json"),
        (b"{".as_slice(), "malformed_json"),
        (b"[]".as_slice(), "invalid_framing"),
        (b"{} {}".as_slice(), "invalid_framing"),
        (br#"{"protocol_version":2,"operation":"prepare_evidence","input":{}}"#.as_slice(), "unsupported_protocol_version"),
        (br#"{"protocol_version":1,"operation":"not_an_operation","input":{}}"#.as_slice(), "unsupported_operation"),
        (br#"{"protocol_version":1,"protocol_version":1,"operation":"prepare_evidence","input":{}}"#.as_slice(), "duplicate_field"),
        (br#"{"protocol_version":1,"operation":"prepare_evidence","input":{},"AUTHOR_SECRET_KEY":3}"#.as_slice(), "unknown_field"),
    ] {
        let output = run_raw(&workspace_root(), bytes).await;
        let wire = response(&output, 2);
        assert_eq!(wire["kind"], "protocol_error");
        assert_eq!(wire["protocol_version"], 1);
        assert_eq!(wire["diagnostics"]["items"][0]["code"], code);
        assert!(wire.get("result").is_none());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("AUTHOR_SECRET_KEY"));
    }
}

#[tokio::test]
async fn evidence_success_and_required_optional_refusals_use_real_selections_and_exits() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(
        fixture.path().join("skill.md"),
        "U001: SELECTED_PROCESS_SENTINEL\n\nU002: PRIVATE_UNSELECTED_SENTINEL\n",
    )
    .unwrap();
    let input = evidence_input(fixture.path());
    let output = run(fixture.path(), "prepare_evidence", input.clone()).await;
    let wire = response(&output, 0);
    assert_eq!(wire["kind"], "operation_result");
    assert_eq!(wire["result"]["operation"], "prepare_evidence");
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    assert_eq!(wire["result"]["outcome"]["change_selections"], json!([]));
    assert_eq!(
        wire["result"]["outcome"]["selections"][0]["readiness"]["content"]["text"],
        "U001: SELECTED_PROCESS_SENTINEL"
    );
    assert_eq!(wire["result"]["outcome"]["measurements"]["files_read"], 1);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_UNSELECTED_SENTINEL"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("SELECTED_PROCESS_SENTINEL"));
    for (requirement, exit, kind) in [("required", 3, "incomplete"), ("optional", 0, "prepared")] {
        let mut missing = input.clone();
        missing["declarations"][0]["requirement"] = json!(requirement);
        missing["declarations"][0]["select"]["target_id"] = json!("U999");
        let wire = response(
            &run(fixture.path(), "prepare_evidence", missing).await,
            exit,
        );
        assert_eq!(wire["result"]["outcome"]["kind"], kind);
        assert_eq!(
            wire["result"]["outcome"]["selections"][0]["readiness"]["kind"],
            "refused"
        );
    }
    let mut invalid = input;
    invalid["sources"]["skill"]["path"] = json!("../outside.md");
    let wire = response(&run(fixture.path(), "prepare_evidence", invalid).await, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "refused");
}
