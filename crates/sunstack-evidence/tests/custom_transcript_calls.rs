#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

async fn capture_records(records: &[Value]) -> ConversationCapture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rollout.jsonl");
    let mut bytes = Vec::new();
    for payload in records {
        bytes.extend(
            serde_json::to_vec(&json!({"type":"response_item","payload":payload})).unwrap(),
        );
        bytes.push(b'\n');
    }
    std::fs::write(&path, bytes).unwrap();
    capture_conversation(
        ConversationCaptureRequest {
            transcript_path: path,
            format: TranscriptFormatName::new("codex_rollout_jsonl").unwrap(),
            cursor: None,
            final_assistant_text: None,
            max_bytes: 100_000,
        },
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn custom_call_category_uses_only_the_name_and_preserves_recorded_input() {
    let patch =
        "*** Begin Patch\n*** Add File: example.ts\n+ctx.storage.put('key', 1);\n*** End Patch";
    for (name, category) in [
        ("exec", ToolCategory::Shell),
        ("exec_command", ToolCategory::Shell),
        ("shell_command", ToolCategory::Shell),
        ("Bash", ToolCategory::Shell),
        ("apply_patch", ToolCategory::EditFile),
        ("future_tool", ToolCategory::Other),
    ] {
        let captured = capture_records(&[
            json!({"type":"custom_tool_call","call_id":"native-id","name":name,"input":patch}),
        ])
        .await;
        assert!(captured.gaps.is_empty(), "{name}: {:?}", captured.gaps);
        assert_eq!(captured.records.len(), 1, "{name}");
        let record = &captured.records[0];
        assert_eq!(record.record_id.as_str(), "tool:native-id");
        let ConversationBody::ToolUse(tool) = &record.body else {
            panic!("custom call must be a tool use");
        };
        assert_eq!(tool.call_id.as_str(), "native-id");
        assert_eq!(tool.tool_name, name);
        assert_eq!(tool.category, category);
        assert_eq!(tool.input, StructuredJson::String(patch.into()));
        assert_eq!(tool.outcome, ToolOutcome::ResultMissing);
    }
    let captured = capture_records(&[
        json!({"type":"custom_tool_call","call_id":"json-text","name":"exec","input":"{\"command\":\"echo quoted\"}"}),
    ])
    .await;
    let ConversationBody::ToolUse(tool) = &captured.records[0].body else {
        panic!("custom call must be a tool use");
    };
    assert_eq!(
        tool.input,
        StructuredJson::String("{\"command\":\"echo quoted\"}".into())
    );
}

#[tokio::test]
async fn custom_outputs_pair_by_call_id_without_guessing_status_from_text() {
    let captured = capture_records(&[
        json!({"type":"custom_tool_call","call_id":"first","name":"exec","input":"one"}),
        json!({"type":"custom_tool_call","call_id":"second","name":"apply_patch","input":"two"}),
        json!({"type":"custom_tool_call_output","call_id":"second","output":"success"}),
        json!({"type":"custom_tool_call_output","call_id":"first","output":"failed is merely quoted output"}),
    ])
    .await;
    assert!(captured.gaps.is_empty());
    assert_eq!(captured.records.len(), 2);
    for (record, call_id, text) in [
        (
            &captured.records[0],
            "first",
            "failed is merely quoted output",
        ),
        (&captured.records[1], "second", "success"),
    ] {
        let ConversationBody::ToolUse(tool) = &record.body else {
            panic!("custom call must be a tool use");
        };
        assert_eq!(tool.call_id.as_str(), call_id);
        assert_eq!(tool.result, ToolResult::FullObserved { text: text.into() });
        assert_eq!(tool.outcome, ToolOutcome::ResultPresentStatusUnknown);
    }
    assert!(captured.records[0].position > captured.records[1].position);
}

#[tokio::test]
async fn custom_output_without_its_call_remains_a_positioned_orphan_gap() {
    let captured = capture_records(&[
        json!({"type":"custom_tool_call_output","call_id":"orphan","output":"observed"}),
    ])
    .await;
    assert!(captured.records.is_empty());
    assert_eq!(
        captured.gaps,
        vec![CoverageGap::OrphanResult {
            call_id: CallId::new("orphan").unwrap(),
            position: 0,
        }]
    );
}

#[tokio::test]
async fn custom_outcome_uses_a_native_status_field_and_ignores_nested_exit_codes() {
    for (native_status, output, expected) in [
        (Some(false), json!("observed"), ToolOutcome::Succeeded),
        (Some(true), json!("observed"), ToolOutcome::Failed),
        (
            None,
            json!({"exit_code":0}),
            ToolOutcome::ResultPresentStatusUnknown,
        ),
        (
            None,
            json!([{"type":"input_text","text":"{\"exit_code\":1}"}]),
            ToolOutcome::ResultPresentStatusUnknown,
        ),
    ] {
        let mut result =
            json!({"type":"custom_tool_call_output","call_id":"native","output":output});
        if let Some(is_error) = native_status {
            result["is_error"] = json!(is_error);
        }
        let captured = capture_records(&[
            json!({"type":"custom_tool_call","call_id":"native","name":"exec","input":"recorded"}),
            result,
        ])
        .await;
        assert!(captured.gaps.is_empty());
        let ConversationBody::ToolUse(tool) = &captured.records[0].body else {
            panic!("custom call must be a tool use");
        };
        assert_eq!(tool.outcome, expected);
    }
}
