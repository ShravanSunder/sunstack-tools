#![allow(clippy::unwrap_used)]
use super::*;
use std::io::Write;
use tokio_util::sync::CancellationToken;

fn mapping() -> TranscriptAdapterMapping {
    serde_json::from_str(
        r#"{
      "format":"example_jsonl", "discriminator":"/event",
      "records":{
        "ask":{"kind":"user_message","message_id":"/id","text":"/message~1body"},
        "answer":{"kind":"assistant_message","text":"/text"},
        "started":{"kind":"tool_call","call_id":"/id","tool_name":"/name","input":"/input"},
        "finished":{"kind":"tool_result","call_id":"/id","text":"/text","is_error":"/failed"}
      },
      "schema":{"type":"object","required":["version"],"properties":{"version":{"const":1}}}
    }"#,
    )
    .unwrap()
}

fn registry(mapping: TranscriptAdapterMapping) -> AdapterRegistry {
    AdapterRegistry::builtins()
        .unwrap()
        .with_mapping(mapping)
        .unwrap()
}

#[test]
fn mapping_admission_refuses_collisions_invalid_pointers_and_unavailable_schema_refs() {
    let mut invalid = mapping();
    invalid.format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    assert!(
        matches!(registry(mapping()).with_mapping(invalid), Err(error) if error.code == EvidenceCode::UnknownTranscriptFormat)
    );
    assert!(
        matches!(registry(mapping()).with_mapping(mapping()), Err(error) if error.code == EvidenceCode::UnknownTranscriptFormat)
    );

    let mut invalid = mapping();
    invalid.records.insert(
        "answer".into(),
        TranscriptRecordMapping::AssistantMessage {
            text: "/invalid~2escape".into(),
            message_id: None,
        },
    );
    assert!(
        matches!(registry(mapping()).with_mapping(TranscriptAdapterMapping {
        format: TranscriptFormatName::new("invalid_pointer").unwrap(), ..invalid
    }), Err(error) if error.code == EvidenceCode::InvalidPointer)
    );

    let mut invalid = mapping();
    invalid.schema =
        Some(serde_json::from_str(r#"{"$ref":"https://schemas.invalid/example.json"}"#).unwrap());
    assert!(
        matches!(AdapterRegistry::builtins().unwrap().with_mapping(invalid), Err(error) if error.code == EvidenceCode::PayloadSchemaInvalid)
    );
}

#[tokio::test]
async fn mapped_capture_preserves_byte_positions_coverage_and_definition_bound_cursors() {
    let lines = [
        r#"{"version":1,"event":"ask","id":"native-user","message/body":"Café?"}"#,
        r#"{"version":1,"event":"started","id":"ok","name":"read","input":"{not parsed}"}"#,
        r#"{"version":1,"event":"finished","id":"ok","text":"done","failed":false}"#,
        r#"{"version":1,"event":"started","id":"unknown","name":"read","input":{"exact":9007199254740993}}"#,
        r#"{"version":1,"event":"finished","id":"unknown","text":"unreported"}"#,
        r#"{"version":1,"event":"started","id":"missing","name":"read","input":null}"#,
        r#"{"version":1,"event":"finished","id":"orphan","text":"no call","failed":true}"#,
        r#"{"version":1,"event":"finished","id":"missing","text":"wrong type","failed":"false"}"#,
        r#"{"version":2,"event":"answer","text":"fails schema"}"#,
        r#"{"version":1,"event":"answer"}"#,
        r#"{"event":"unmapped"}"#,
        "not json",
        r#"{"version":1,"event":"answer","text":"Café."}"#,
    ];
    let mut position = 0;
    let positions = lines.map(|line| {
        let offset = position;
        position += line.len() as u64 + 1;
        offset
    });
    let partial =
        r#"{"version":1,"event":"finished","id":"missing","text":"failed","failed":true}"#;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("trace.jsonl");
    std::fs::write(&path, format!("{}\n{partial}", lines.join("\n"))).unwrap();
    let mut request = ConversationCaptureRequest {
        transcript_path: path.clone(),
        format: mapping().format,
        cursor: None,
        final_assistant_text: None,
        max_bytes: 4096,
    };
    let capture = capture_conversation(
        request.clone(),
        registry(mapping()),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(capture.committed_bytes, position);
    assert_eq!(capture.coverage.bytes_read, position + partial.len() as u64);
    assert_eq!(
        capture.tail,
        CaptureTail::Incomplete {
            bytes: partial.len() as u64
        }
    );
    assert_eq!(
        capture
            .records
            .iter()
            .map(|record| record.position)
            .collect::<Vec<_>>(),
        [
            positions[0],
            positions[2],
            positions[4],
            positions[5],
            positions[12]
        ]
    );
    assert_eq!(capture.records[0].record_id.as_str(), "native-user");
    assert_eq!(capture.records[0].injection, InjectionOrigin::Unknown);
    assert_eq!(
        capture.records[4].record_id.as_str(),
        format!("offset:{}", positions[12])
    );
    assert_eq!(capture.turns[0].records.len(), 5);
    assert_eq!(
        capture.gaps,
        [
            CoverageGap::UnknownInjection {
                record_id: RecordId::new("native-user").unwrap()
            },
            CoverageGap::OrphanResult {
                call_id: CallId::new("orphan").unwrap(),
                position: positions[6]
            },
            CoverageGap::MalformedRecord {
                position: positions[7]
            },
            CoverageGap::MalformedRecord {
                position: positions[8]
            },
            CoverageGap::MalformedRecord {
                position: positions[9]
            },
            CoverageGap::UnsupportedRecord {
                position: positions[10]
            },
            CoverageGap::MalformedRecord {
                position: positions[11]
            },
        ]
    );
    let calls: Vec<_> = capture
        .records
        .iter()
        .filter_map(|record| match &record.body {
            ConversationBody::ToolUse(tool) => Some(tool),
            _ => None,
        })
        .collect();
    assert_eq!(calls[0].outcome, ToolOutcome::Succeeded);
    assert_eq!(
        calls[0].input,
        StructuredJson::String("{not parsed}".into())
    );
    assert_eq!(calls[1].outcome, ToolOutcome::ResultPresentStatusUnknown);
    assert_eq!(
        calls[1].input.pointer("/exact"),
        Some(&StructuredJson::Number(9007199254740993_u64.into()))
    );
    assert_eq!(calls[2].outcome, ToolOutcome::ResultMissing);
    assert_eq!(calls[2].result, ToolResult::Absent);

    let mut selection = ConversationSelection {
        turns: TurnWindow::All {},
        user_messages: Inclusion::Included,
        assistant_messages: AssistantSelection::Excluded,
        tool_use: ToolUseSelection::Included {
            tools: ToolFilter::All {},
            results: Inclusion::Included,
        },
        hook_injected: Inclusion::Excluded,
    };
    let all = select_conversation(&capture, &selection).unwrap();
    assert_eq!(
        all.records.first().unwrap().record_id.as_str(),
        "native-user"
    );

    request.cursor = Some(capture.cursor.encode().unwrap());
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let continued = capture_conversation(
        request.clone(),
        registry(mapping()),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    selection.turns = TurnWindow::SinceCursor {};
    let selected = select_conversation(&continued, &selection).unwrap();
    let [amended] = selected.records.as_slice() else {
        unreachable!()
    };
    assert_eq!(amended.position, position);
    assert!(
        matches!(&amended.body, ConversationBody::ToolUse(tool) if tool.call_id.as_str() == "missing" && tool.outcome == ToolOutcome::Failed)
    );

    let mut different_pointer = mapping();
    different_pointer.records.insert(
        "answer".into(),
        TranscriptRecordMapping::AssistantMessage {
            text: "/other".into(),
            message_id: None,
        },
    );
    let mut different_schema = mapping();
    different_schema.schema = Some(TranscriptRecordSchema::Boolean(true));
    for changed in [different_pointer, different_schema] {
        let error =
            capture_conversation(request.clone(), registry(changed), CancellationToken::new())
                .await
                .unwrap_err();
        assert_eq!(error.code, EvidenceCode::TranscriptDiscontinuous);
    }
}

#[tokio::test]
async fn later_mapped_identity_collisions_leave_the_first_record_and_cursor_intact() {
    const FALLBACK: &str = "{\"version\":1,\"event\":\"answer\",\"text\":\"fallback\"}\n";
    const NATIVE_OFFSET: &str =
        "{\"version\":1,\"event\":\"answer_id\",\"id\":\"offset:0\",\"text\":\"native\"}\n";
    const NATIVE_NEXT_OFFSET: &str =
        "{\"version\":1,\"event\":\"answer_id\",\"id\":\"offset:77\",\"text\":\"native\"}\n";
    const CALL: &str =
        "{\"version\":1,\"event\":\"started\",\"id\":\"read\",\"name\":\"read\",\"input\":null}\n";
    const NATIVE_TOOL: &str =
        "{\"version\":1,\"event\":\"answer_id\",\"id\":\"tool:read\",\"text\":\"native\"}\n";
    let mut definition = mapping();
    definition.records.insert(
        "answer_id".into(),
        TranscriptRecordMapping::AssistantMessage {
            text: "/text".into(),
            message_id: Some("/id".into()),
        },
    );
    let registry = registry(definition.clone());
    let root = tempfile::tempdir().unwrap();
    let native_before_fallback = format!("{:<76}\n", NATIVE_NEXT_OFFSET.trim_end());
    for (index, (first, later, expected_id)) in [
        (FALLBACK, NATIVE_OFFSET, "offset:0"),
        (native_before_fallback.as_str(), FALLBACK, "offset:77"),
        (CALL, NATIVE_TOOL, "tool:read"),
        (NATIVE_TOOL, CALL, "tool:read"),
    ]
    .into_iter()
    .enumerate()
    {
        let path = root.path().join(format!("trace-{index}.jsonl"));
        std::fs::write(&path, first).unwrap();
        let mut request = ConversationCaptureRequest {
            transcript_path: path.clone(),
            format: definition.format.clone(),
            cursor: None,
            final_assistant_text: None,
            max_bytes: 4096,
        };
        let before =
            capture_conversation(request.clone(), registry.clone(), CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(before.records[0].record_id.as_str(), expected_id);
        assert_eq!(before.records[0].position, 0);
        request.cursor = Some(before.cursor.encode().unwrap());
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(later.as_bytes())
            .unwrap();

        let continued = capture_conversation(request, registry.clone(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(continued.records, before.records, "order {index}");
        assert_eq!(
            continued.cursor.record_watermarks,
            before.cursor.record_watermarks
        );
        assert_eq!(
            continued.gaps,
            [CoverageGap::MalformedRecord {
                position: first.len() as u64
            }]
        );
        let selected = select_conversation(
            &continued,
            &ConversationSelection {
                turns: TurnWindow::SinceCursor {},
                user_messages: Inclusion::Included,
                assistant_messages: AssistantSelection::All,
                tool_use: ToolUseSelection::Included {
                    tools: ToolFilter::All {},
                    results: Inclusion::Included,
                },
                hook_injected: Inclusion::Excluded,
            },
        )
        .unwrap();
        assert!(selected.records.is_empty());
        assert_eq!(selected.coverage.gaps, continued.gaps);
    }

    let path = root.path().join("repeated-native.jsonl");
    std::fs::write(&path, format!("{NATIVE_OFFSET}{NATIVE_OFFSET}")).unwrap();
    let capture = capture_conversation(
        ConversationCaptureRequest {
            transcript_path: path,
            format: definition.format,
            cursor: None,
            final_assistant_text: None,
            max_bytes: 4096,
        },
        registry,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(capture.gaps.is_empty());
    assert_eq!(capture.records.len(), 1);
    assert_eq!(capture.records[0].record_id.as_str(), "offset:0");
    assert_eq!(capture.records[0].position, NATIVE_OFFSET.len() as u64);
    assert!(
        matches!(&capture.records[0].body, ConversationBody::AssistantMessage { blocks } if blocks.len() == 2)
    );
}
