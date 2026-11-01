#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../support/mod.rs"]
mod support;
use serde_json::json;
use std::io::Write;
use support::*;

#[tokio::test]
async fn relative_transcripts_and_wire_cursor_paths_follow_each_actual_process_cwd() {
    for tag in ["CWD_A", "CWD_B"] {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("session.jsonl");
        std::fs::write(&path, format!("{}\n", json!({"type":"user","origin":{"kind":"user"},"message":{"id":"u0","content":[{"type":"text","text":tag}]}}))).unwrap();
        let mut input = evidence_input(fixture.path());
        input["repositories"] = json!([]);
        input["sources"] = json!({"skill":{"kind":"conversation","transcript_path":"session.jsonl","format":"claude_code_jsonl"}});
        input["declarations"][0]["select"] = json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"excluded"},"hook_injected":"excluded"});
        let first = response(
            &run(fixture.path(), "prepare_evidence", input.clone()).await,
            0,
        );
        let first = &first["result"]["outcome"];
        assert!(
            first["selections"][0]["readiness"]["content"]
                .to_string()
                .contains(tag)
        );
        assert_eq!(
            first["cursors"][0]["transcript_path"],
            json!(path.canonicalize().unwrap())
        );
        let appended = format!(
            "{}\n",
            json!({"type":"assistant","message":{"id":"a1","content":[{"type":"text","text":"APPENDED_WHOLE_MESSAGE"}]}})
        );
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(appended.as_bytes())
            .unwrap();
        input["cursors"] = json!([{"transcript_path":"session.jsonl","format":"claude_code_jsonl","cursor":first["cursors"][0]["cursor"]}]);
        input["declarations"][0]["select"]["turns"] = json!({"kind":"since_cursor"});
        let resumed = response(&run(fixture.path(), "prepare_evidence", input).await, 0);
        let resumed = &resumed["result"]["outcome"];
        let records = resumed["selections"][0]["readiness"]["content"]["records"]
            .as_array()
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["record_id"], "a1");
        assert!(records[0].to_string().contains("APPENDED_WHOLE_MESSAGE"));
        assert!(!records[0].to_string().contains(tag));
        assert_eq!(
            resumed["captures"][0]["cursor_basis"],
            "continued_from_cursor"
        );
    }
}

#[tokio::test]
async fn invalid_payload_and_malformed_requests_keep_received_values_out_of_diagnostics() {
    let fixture = tempfile::tempdir().unwrap();
    let schema = fixture.path().join("payload.schema.json");
    std::fs::write(&schema, r#"{"type":"object","properties":{"allowed":{"type":"string"}},"additionalProperties":false}"#).unwrap();
    let mut input = evidence_input(fixture.path());
    input["repositories"] = json!([]);
    input["declarations"] = json!([]);
    input["sources"] = json!({"packet":{"kind":"payload","schema":"packet","value":{"AUTHOR_SECRET_KEY":"PAYLOAD_SECRET_SENTINEL"}}});
    input["payload_schemas"] = json!([{"name":"packet","path":schema}]);
    let output = run(fixture.path(), "prepare_evidence", input).await;
    let wire = response(&output, 2);
    assert_eq!(
        wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
        "payload_schema_mismatch"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PAYLOAD_SECRET_SENTINEL"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PAYLOAD_SECRET_SENTINEL"));
    let output = run_raw(
        fixture.path(),
        br#"{"protocol_version":1,"AUTHOR_SECRET_KEY":"PAYLOAD_SECRET_SENTINEL""#,
    )
    .await;
    let wire = response(&output, 2);
    assert_eq!(wire["kind"], "protocol_error");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PAYLOAD_SECRET_SENTINEL"));
}
