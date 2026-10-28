#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;

#[test]
fn a_partial_tool_error_keeps_prior_message_and_call_before_the_malformed_gap() {
    // The old parser records the message and first call before the second call's missing input fails.
    let first = concat!(
        "{\"type\":\"assistant\",\"message\":{\"id\":\"partial\",\"content\":[",
        "{\"type\":\"text\",\"text\":\"kept\"},",
        "{\"type\":\"tool_use\",\"id\":\"good\",\"name\":\"Read\",\"input\":{}},",
        "{\"type\":\"tool_use\",\"id\":\"bad\",\"name\":\"Read\"}]}}\n"
    );
    let text = format!(
        "{first}{}\n",
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"good","content":"result"}]}}"#
    );
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let legacy = registry
        .adapter(&format)
        .unwrap()
        .normalize(&CapturedTranscript {
            bytes: text.as_bytes(),
            source_identity: "partial-line",
        });
    assert_eq!(
        legacy.gaps,
        vec![CoverageGap::MalformedRecord { position: 0 }]
    );
    assert_eq!(legacy.records.len(), 2);
    assert_eq!(legacy.records[0].position, 0);
    assert_eq!(
        legacy.records[0].body,
        ConversationBody::AssistantMessage {
            blocks: vec![
                MessageBlock::Text {
                    text: "kept".into()
                },
                MessageBlock::ToolCall {
                    call_id: CallId::new("good").unwrap()
                },
                MessageBlock::ToolCall {
                    call_id: CallId::new("bad").unwrap()
                },
            ]
        }
    );
    let ConversationBody::ToolUse(tool) = &legacy.records[1].body else {
        unreachable!()
    };
    assert_eq!(tool.call_id.as_str(), "good");
    assert_eq!(
        tool.result,
        ToolResult::FullObserved {
            text: "result".into()
        }
    );
    assert_eq!(tool.outcome, ToolOutcome::Succeeded);
    assert_eq!(legacy.records[1].position, first.len() as u64);
}
