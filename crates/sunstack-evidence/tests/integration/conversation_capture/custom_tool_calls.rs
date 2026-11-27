#![allow(clippy::panic)]

use super::{capture, request};
use serde_json::{Value, json};
use sunstack_evidence::*;

async fn capture_codex_rollout(payloads: &[Value]) -> (ConversationCapture, Vec<u64>) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rollout.jsonl");
    let mut bytes = Vec::new();
    let mut offsets = Vec::new();
    for payload in payloads {
        offsets.push(u64::try_from(bytes.len()).unwrap());
        bytes.extend(
            serde_json::to_vec(&json!({"type":"response_item", "payload":payload})).unwrap(),
        );
        bytes.push(b'\n');
    }
    std::fs::write(&path, bytes).unwrap();
    let mut input = request(&path);
    input.format = TranscriptFormatName::new("codex_rollout_jsonl").unwrap();
    (capture(input).await, offsets)
}

fn observed_call<'capture>(
    capture: &'capture ConversationCapture,
    call_id: &str,
    expected_position: u64,
) -> &'capture ToolUse {
    let record_id = format!("tool:{call_id}");
    let record = capture
        .records
        .iter()
        .find(|record| record.record_id.as_str() == record_id)
        .unwrap_or_else(|| panic!("missing {record_id} with gaps {:?}", capture.gaps));
    assert_eq!(record.provenance, RecordProvenance::Transcript);
    assert_eq!(record.position, expected_position);
    let ConversationBody::ToolUse(tool) = &record.body else {
        panic!("{record_id} was not a tool call");
    };
    assert_eq!(tool.call_id.as_str(), call_id);
    tool
}

#[tokio::test]
async fn codex_custom_calls_pair_out_of_order_and_preserve_original_input_and_status() {
    let shell_input = "  {\"command\":\"echo example\"}\n";
    let function_output = json!([{"type":"output_text","text":"stay as JSON"}]);
    let (captured, positions) = capture_codex_rollout(&[
        json!({"type":"function_call","call_id":"plain","namespace":"mcp__workspace","name":"read","arguments":"{\"path\":\"src/main.rs\"}"}),
        json!({"type":"custom_tool_call","call_id":"shell","name":"exec","input":shell_input}),
        json!({"type":"custom_tool_call","call_id":"patch","name":"apply_patch","input":"original patch"}),
        json!({"type":"custom_tool_call","call_id":"statusless","name":"exec_command","input":"observed command"}),
        json!({"type":"custom_tool_call_output","call_id":"patch","is_error":true,"output":"denied"}),
        json!({"type":"custom_tool_call_output","call_id":"statusless","output":"Process exited with code 0"}),
        json!({"type":"function_call_output","call_id":"plain","output":function_output}),
        json!({"type":"custom_tool_call_output","call_id":"shell","is_error":false,
            "output":[{"type":"input_text","text":"  first\n"},{"type":"text","text":""},{"type":"output_text","text":"second  "}]
        }),
    ])
    .await;

    assert_eq!(captured.records.len(), 4, "{:?}", captured.gaps);
    assert!(captured.gaps.is_empty(), "{:?}", captured.gaps);
    let shell = observed_call(&captured, "shell", positions[7]);
    assert_eq!(shell.tool_name, "exec");
    assert_eq!(shell.category, ToolCategory::Shell);
    assert_eq!(shell.input, StructuredJson::String(shell_input.into()));
    assert_eq!(shell.outcome, ToolOutcome::Succeeded);
    assert_eq!(
        shell.result,
        ToolResult::FullObserved {
            text: "  first\nsecond  ".into()
        }
    );

    let patch = observed_call(&captured, "patch", positions[4]);
    assert_eq!(patch.tool_name, "apply_patch");
    assert_eq!(patch.category, ToolCategory::EditFile);
    assert_eq!(patch.outcome, ToolOutcome::Failed);
    assert_eq!(
        patch.result,
        ToolResult::FullObserved {
            text: "denied".into()
        }
    );

    let statusless = observed_call(&captured, "statusless", positions[5]);
    assert_eq!(statusless.outcome, ToolOutcome::ResultPresentStatusUnknown);
    assert_eq!(
        statusless.result,
        ToolResult::FullObserved {
            text: "Process exited with code 0".into()
        }
    );

    let plain = observed_call(&captured, "plain", positions[6]);
    assert_eq!(plain.tool_name, "mcp__workspace.read");
    assert_eq!(plain.category, ToolCategory::Mcp);
    assert_eq!(
        plain.input,
        serde_json::from_value(json!({"path":"src/main.rs"})).unwrap()
    );
    assert_eq!(plain.outcome, ToolOutcome::ResultPresentStatusUnknown);
    assert_eq!(
        plain.result,
        ToolResult::FullObserved {
            text: function_output.to_string()
        }
    );
}

#[tokio::test]
async fn codex_custom_calls_keep_missing_results_and_positioned_orphans() {
    let (captured, positions) = capture_codex_rollout(&[
        json!({"type":"custom_tool_call","call_id":"pending","name":"custom_other","input":""}),
        json!({"type":"custom_tool_call_output","call_id":"orphan","output":"observed"}),
    ])
    .await;

    assert_eq!(captured.records.len(), 1);
    let pending = observed_call(&captured, "pending", positions[0]);
    assert_eq!(pending.tool_name, "custom_other");
    assert_eq!(pending.category, ToolCategory::Other);
    assert_eq!(pending.input, StructuredJson::String(String::new()));
    assert_eq!(pending.outcome, ToolOutcome::ResultMissing);
    assert_eq!(pending.result, ToolResult::Absent);
    assert_eq!(
        captured.gaps,
        [CoverageGap::OrphanResult {
            call_id: CallId::new("orphan").unwrap(),
            position: positions[1],
        }]
    );
}

#[tokio::test]
async fn codex_custom_results_keep_json_that_cannot_be_projected_without_loss() {
    for output in [
        json!([{"type":"output_text","text":"caption"},{"type":"image","data":"original"}]),
        json!([{"type":"text","text":"observed","metadata":"preserve"}]),
        json!({"exit_code":1,"stdout":"observed"}),
    ] {
        let (captured, positions) = capture_codex_rollout(&[
            json!({"type":"custom_tool_call","call_id":"mixed","name":"exec","input":"fixture"}),
            json!({"type":"custom_tool_call_output","call_id":"mixed","output":output}),
        ])
        .await;

        assert!(captured.gaps.is_empty(), "{:?}", captured.gaps);
        let tool = observed_call(&captured, "mixed", positions[1]);
        assert_eq!(tool.outcome, ToolOutcome::ResultPresentStatusUnknown);
        let ToolResult::FullObserved { text } = &tool.result else {
            panic!("the original output was observed");
        };
        assert_eq!(serde_json::from_str::<Value>(text).unwrap(), output);
    }
}
