#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;

fn events(format: &str, bytes: &[u8]) -> Vec<TranscriptEvent> {
    AdapterRegistry::builtins()
        .unwrap()
        .event_reader(&TranscriptFormatName::new(format).unwrap())
        .unwrap()
        .events(&CapturedTranscript {
            bytes,
            source_identity: "public fixture",
        })
}

#[test]
fn tool_call_and_result_keep_their_own_source_positions_and_decoded_arguments() {
    let call = "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"exec_command\",\"call_id\":\"c1\",\"arguments\":\"{\\\"cmd\\\":\\\"read fixture\\\"}\"}}\n";
    let result = "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"c1\",\"output\":\"observed\"}}\n";
    let transcript = format!("{call}{result}");

    let actual = events("codex_rollout_jsonl", transcript.as_bytes());

    assert_eq!(actual.len(), 2);
    let TranscriptEvent::ToolCall {
        call_id,
        tool,
        category,
        input,
        position,
    } = &actual[0]
    else {
        unreachable!("call was lost or merged")
    };
    assert_eq!(call_id.as_str(), "c1");
    assert_eq!(tool.as_str(), "exec_command");
    assert_eq!(*category, ToolCategory::Shell);
    assert_eq!(
        serde_json::to_value(input).unwrap(),
        serde_json::json!({"cmd": "read fixture"})
    );
    assert_eq!(
        *position,
        SourcePosition {
            byte_offset: 0,
            line: 1
        }
    );
    let TranscriptEvent::ToolResult {
        call_id,
        outcome,
        result,
        position,
    } = &actual[1]
    else {
        unreachable!("result was lost or merged")
    };
    assert_eq!(call_id.as_str(), "c1");
    assert_eq!(*outcome, ToolOutcome::ResultPresentStatusUnknown);
    assert_eq!(
        *result,
        ToolResult::FullObserved {
            text: "observed".into()
        }
    );
    assert_eq!(
        *position,
        SourcePosition {
            byte_offset: call.len() as u64,
            line: 2
        }
    );
}

#[test]
fn streamed_parts_preserve_every_position_author_and_injection_before_legacy_merge() {
    let first = "{\"type\":\"user\",\"origin\":{\"kind\":\"hook\"},\"message\":{\"id\":\"stream\",\"content\":\"α\"}}\n";
    let second = "{\"type\":\"user\",\"origin\":{\"kind\":\"hook\"},\"message\":{\"id\":\"stream\",\"content\":\"second\"}}\n";
    let transcript = format!("{first}{second}");
    let capture = CapturedTranscript {
        bytes: transcript.as_bytes(),
        source_identity: "public fixture",
    };
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();

    let actual = registry.event_reader(&format).unwrap().events(&capture);
    let merged = registry.adapter(&format).unwrap().normalize(&capture);

    assert_eq!(actual.len(), 2);
    for (index, event) in actual.iter().enumerate() {
        let TranscriptEvent::MessagePart {
            message_id,
            author,
            position,
            blocks,
            injection,
            ..
        } = event
        else {
            unreachable!("message part missing")
        };
        assert_eq!(message_id.as_str(), "stream");
        assert_eq!(*author, RecordAuthor::User);
        assert_eq!(*injection, InjectionOrigin::KnownInjected);
        assert_eq!(position.line, index as u64 + 1);
        assert_eq!(
            position.byte_offset,
            if index == 0 { 0 } else { first.len() as u64 }
        );
        assert_eq!(
            blocks,
            &[MessageBlock::Text {
                text: if index == 0 { "α" } else { "second" }.into()
            }]
        );
    }
    assert_eq!(merged.records.len(), 1);
    assert_eq!(merged.records[0].position, first.len() as u64);
    assert_eq!(merged.records[0].injection, InjectionOrigin::KnownInjected);
    assert_eq!(
        merged.records[0].body,
        ConversationBody::UserMessage {
            blocks: vec![
                MessageBlock::Text { text: "α".into() },
                MessageBlock::Text {
                    text: "second".into()
                }
            ]
        }
    );
}
