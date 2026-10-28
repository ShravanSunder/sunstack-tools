#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn legacy_capture_and_selection_preserve_hook_user_and_unknown_origins() {
    // The three bodies are identical; only the envelope supplies their origin.
    let transcript = concat!(
        "{\"type\":\"user\",\"uuid\":\"hook\",\"origin\":{\"kind\":\"hook\"},\"message\":{\"content\":\"same text\"}}\n",
        "{\"type\":\"user\",\"uuid\":\"user\",\"origin\":{\"kind\":\"user\"},\"message\":{\"content\":\"same text\"}}\n",
        "{\"type\":\"user\",\"uuid\":\"unknown\",\"message\":{\"content\":\"same text\"}}\n",
    );
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("origins.jsonl");
    std::fs::write(&path, transcript).unwrap();

    let capture = capture_conversation(
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

    assert_eq!(capture.records.len(), 3);
    assert_eq!(
        capture
            .records
            .iter()
            .map(|record| record.injection)
            .collect::<Vec<_>>(),
        vec![
            InjectionOrigin::KnownInjected,
            InjectionOrigin::NotInjected,
            InjectionOrigin::Unknown
        ]
    );
    let selected = select_conversation(
        &capture,
        &ConversationSelection {
            turns: TurnWindow::All {},
            user_messages: Inclusion::Included,
            assistant_messages: AssistantSelection::All,
            tool_use: ToolUseSelection::Excluded {},
            hook_injected: Inclusion::Excluded,
        },
    )
    .unwrap();
    assert_eq!(
        selected
            .records
            .iter()
            .map(|record| record.record_id.as_str())
            .collect::<Vec<_>>(),
        vec!["user", "unknown"]
    );
    assert_eq!(
        capture
            .gaps
            .iter()
            .filter(|gap| matches!(gap, CoverageGap::UnknownInjection { .. }))
            .count(),
        1
    );
}
