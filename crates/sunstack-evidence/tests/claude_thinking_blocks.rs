#![allow(clippy::unwrap_used)]
use serde_json::json;
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn claude_thinking_accepts_empty_strings_and_keeps_missing_or_wrong_types_malformed() {
    for (block, expected) in [
        (
            json!({"type":"thinking","thinking":"","signature":"fixture"}),
            Some(""),
        ),
        (
            json!({"type":"thinking","thinking":"observed"}),
            Some("observed"),
        ),
        (json!({"type":"thinking"}), None),
        (json!({"type":"thinking","thinking":null}), None),
        (json!({"type":"thinking","thinking":12}), None),
    ] {
        // Arrange a complete real-format transcript; no adapter output is substituted.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("conversation.jsonl");
        std::fs::write(
            &path,
            format!(
                "{}\n",
                json!({"type":"assistant","message":{"id":"thinking-message","content":[block]}})
            ),
        )
        .unwrap();
        // Act through the built-in capture and normalization path.
        let captured = capture_conversation(
            ConversationCaptureRequest {
                transcript_path: path,
                format: TranscriptFormatName::new("claude_code_jsonl").unwrap(),
                cursor: None,
                final_assistant_text: None,
                max_bytes: 100_000,
            },
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        // Assert content and gaps independently of the adapter's result.
        assert_eq!(captured.tail, CaptureTail::Complete);
        if let Some(text) = expected {
            assert!(captured.gaps.is_empty(), "{:?}", captured.gaps);
            assert_eq!(captured.records.len(), 1);
            assert_eq!(captured.records[0].position, 0);
            assert_eq!(
                captured.records[0].body,
                ConversationBody::AssistantMessage {
                    blocks: vec![MessageBlock::Thinking { text: text.into() }],
                }
            );
        } else {
            assert!(captured.records.is_empty());
            assert_eq!(
                captured.gaps,
                vec![CoverageGap::MalformedRecord { position: 0 }]
            );
        }
    }
}
