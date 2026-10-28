#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

fn schema(format: &str) -> CustomEntrySchema {
    serde_json::from_value(json!({"format":format,"record_schema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]},"fields":{"text":"/text"}})).unwrap()
}

#[tokio::test]
async fn whole_json_without_newline_and_incomplete_document_have_distinct_capture_states() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("records.json");
    let (registry, format) = AdapterRegistry::builtins()
        .unwrap()
        .with_custom_schema(&schema("json"))
        .unwrap();
    for (text, complete, count) in [
        (r#"{"text":"ordinary owner data"}"#, true, 1),
        (r#"[{"text":"first"},{"text":"last"}]"#, true, 2),
        (r#"[{"text":"first"},{"text":"unfinished"#, false, 0),
    ] {
        std::fs::write(&path, text).unwrap();
        let capture = capture_conversation(
            ConversationCaptureRequest {
                transcript_path: path.clone(),
                format: format.clone(),
                cursor: None,
                final_assistant_text: None,
                max_bytes: 65536,
            },
            registry.clone(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(capture.tail == CaptureTail::Complete, complete, "{text}");
        let projected = project_positioned(
            &SourceInputId::new("custom-json").unwrap(),
            capture.transcript_lines().unwrap(),
        )
        .unwrap();
        assert_eq!(projected.records.len(), count, "{text}");
        assert!(
            projected.gaps.is_empty(),
            "a partial document is not a schema mismatch"
        );
        if complete {
            assert_eq!(capture.cursor.committed_bytes, text.len() as u64);
        }
    }
}

#[test]
fn mapped_custom_values_are_ordinary_and_invalid_schema_is_refused_offline() {
    let custom = schema("jsonl");
    let (registry, format) = AdapterRegistry::builtins()
        .unwrap()
        .with_custom_schema(&custom)
        .unwrap();
    let bytes = b"{\"text\":\"thinking is ordinary owner data here\"}\n";
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes,
            source_identity: "custom",
        });
    let projection = project_positioned(&SourceInputId::new("custom").unwrap(), &lines).unwrap();
    let public = project_record(&projection.records[0], &Default::default()).unwrap();
    assert!(
        serde_json::to_string(&public.record)
            .unwrap()
            .contains("thinking is ordinary owner data here")
    );
    assert!(public.privacy.left_out.is_empty());
    let mut invalid = custom;
    invalid.record_schema = serde_json::from_value(json!({"type":"no-such-type"})).unwrap();
    assert!(
        AdapterRegistry::builtins()
            .unwrap()
            .with_custom_schema(&invalid)
            .is_err()
    );
}
