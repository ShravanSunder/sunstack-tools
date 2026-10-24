#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

fn selection(pattern: &str) -> ConversationSelection {
    serde_json::from_value(json!({
        "turns":{"kind":"all"},"user_messages":"excluded","assistant_messages":"excluded",
        "tool_use":{"kind":"included","tools":{"kind":"all"},"results":"excluded","input_pattern":pattern},
        "hook_injected":"excluded"
    })).unwrap()
}
async fn captured(values: &[Value]) -> ConversationCapture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rollout.jsonl");
    let bytes = values
        .iter()
        .map(|payload| format!("{}\n", json!({"type":"response_item","payload":payload})))
        .collect::<String>();
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
async fn input_pattern_matches_decoded_strings_and_compact_structured_json() {
    let capture = captured(&[
        json!({"type":"custom_tool_call","call_id":"string","name":"exec","input":"say \"quoted\"\nsecond"}),
        json!({"type":"custom_tool_call","call_id":"object","name":"exec","input":{"command":"say \"quoted\"\nsecond"}}),
    ]).await;
    for (pattern, expected_id) in [
        (r#""quoted"\nsecond"#, "tool:string"),
        (r#"\\"quoted\\"\\nsecond"#, "tool:object"),
    ] {
        let selected = select_conversation(&capture, &selection(pattern)).unwrap();
        assert_eq!(selected.records.len(), 1, "{pattern}");
        assert_eq!(selected.records[0].record_id.as_str(), expected_id);
        assert_eq!(
            serde_json::to_value(selected.omissions).unwrap(),
            json!([
                {"reason":"input_pattern","count":1}
            ])
        );
    }
}

#[tokio::test]
async fn input_pattern_omissions_count_only_eligible_calls_after_window_and_projection() {
    let capture = captured(&[
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":"first"}]}),
        json!({"type":"custom_tool_call","call_id":"outside-window","name":"exec","input":"reject"}),
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":"second"}]}),
        json!({"type":"custom_tool_call","call_id":"match","name":"exec","input":"keep"}),
        json!({"type":"custom_tool_call","call_id":"pattern-reject","name":"exec","input":"reject"}),
        json!({"type":"custom_tool_call","call_id":"name-reject","name":"read","input":"reject"}),
    ]).await;
    let mut select = selection("keep");
    select.turns = TurnWindow::Last {
        count: 1.try_into().unwrap(),
    };
    if let ToolUseSelection::Included { tools, .. } = &mut select.tool_use {
        *tools = ToolFilter::Named {
            names: vec!["exec".into()],
        };
    }
    let selected = select_conversation(&capture, &select).unwrap();
    assert_eq!(selected.records.len(), 1);
    assert_eq!(selected.records[0].record_id.as_str(), "tool:match");
    let omissions = serde_json::to_value(selected.omissions).unwrap();
    let omissions = omissions.as_array().unwrap();
    assert_eq!(omissions.len(), 3);
    for expected in [
        json!({"reason":"window","count":2}),
        json!({"reason":"projection","count":2}),
        json!({"reason":"input_pattern","count":1}),
    ] {
        assert!(omissions.contains(&expected), "{omissions:?}");
    }
}

#[tokio::test]
async fn invalid_input_pattern_refuses_before_a_transcript_is_read() {
    let directory = tempfile::tempdir().unwrap();
    let input: EvidencePlanInput = serde_json::from_value(json!({
        "repositories":[],"sources":{"session":{"kind":"conversation","transcript_path":directory.path().join("not-created.jsonl"),"format":"codex_rollout_jsonl"}},
        "payload_schemas":[],"cursors":[],"declarations":[],"native_queries":[],"package_bindings":[],
        "limits":{"max_files":1,"max_file_bytes":4096,"max_total_bytes":8192,"max_matches":8,"max_transcript_bytes":4096,"include_ignored":false},"discovery":{"kind":"none"}
    })).unwrap();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let selected = selection("[");
    let result = session
        .select(&EvidenceDeclaration {
            id: DeclarationId::new("invalid").unwrap(),
            input: SourceName::new("session").unwrap(),
            requirement: EvidenceRequirement::Required,
            select: EvidenceSelection::Conversation {
                turns: selected.turns,
                user_messages: selected.user_messages,
                assistant_messages: selected.assistant_messages,
                tool_use: selected.tool_use,
                hook_injected: selected.hook_injected,
            },
            max_items: None,
            max_bytes: None,
        })
        .await;
    assert!(
        matches!(result.readiness,SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code==EvidenceCode::InvalidSelector)
    );
    assert_eq!(session.close().measurements.transcript_bytes_read, 0);
}
