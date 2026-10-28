#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use sunstack_evidence::*;

#[test]
fn idless_cursor_messages_and_tool_blocks_keep_source_positions_without_invented_results() {
    let source=[json!({"role":"user","message":{"content":[{"type":"text","text":"user request"}]}}),json!({"role":"assistant","message":{"content":[{"type":"text","text":"MATCH cursor reply"},{"type":"tool_use","name":"Read","input":{"path":"public.rs"}},{"type":"tool_use","name":"Bash","input":{"command":"pwd"}}]}}),json!({"role":"assistant","message":{"content":[{"type":"thinking","thinking":"PRIVATE_CURSOR"},{"type":"text","text":"after tools"}]}})].map(|value|value.to_string()+"\n");
    let text = source.concat();
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("cursor_export_jsonl").unwrap();
    let reader = registry.event_reader(&format).unwrap();
    let lines = reader.lines(&CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "cursor-fixture",
    });
    let projection =
        project_positioned(&SourceInputId::new("cursor-fixture").unwrap(), &lines).unwrap();
    assert!(projection.gaps.is_empty(), "{:?}", projection.gaps);
    assert_eq!(projection.records.len(), 5);
    assert_eq!(projection.records[0].author, RecordAuthor::User);
    assert_eq!(projection.records[1].author, RecordAuthor::Assistant);
    let calls = projection
        .records
        .iter()
        .filter_map(|record| match &record.body {
            PositionedBody::ToolCall {
                call_id,
                tool,
                input,
            } => Some((record, call_id, tool, input)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0].1, calls[1].1);
    assert_eq!(calls[0].2.as_str(), "Read");
    assert_eq!(calls[1].2.as_str(), "Bash");
    assert_eq!(
        serde_json::to_value(calls[0].3).unwrap(),
        json!({"path":"public.rs"})
    );
    assert_eq!(
        serde_json::to_value(calls[1].3).unwrap(),
        json!({"command":"pwd"})
    );
    assert_eq!(
        projection
            .records
            .iter()
            .map(|record| record.record_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );
    for (index, record) in projection.records.iter().enumerate() {
        let (line, offset) = match index {
            0 => (1, 0),
            1..=3 => (2, source[0].len()),
            _ => (3, source[0].len() + source[1].len()),
        };
        assert_eq!(
            record.positions,
            [SourcePosition {
                line,
                byte_offset: offset as u64
            }]
        );
    }
    for (record, _, _, _) in calls {
        assert_eq!(record.author, RecordAuthor::Assistant);
        assert_eq!(
            record.positions,
            [SourcePosition {
                line: 2,
                byte_offset: source[0].len() as u64
            }]
        );
    }
    assert!(
        !projection
            .records
            .iter()
            .any(|record| matches!(record.body, PositionedBody::ToolResult { .. }))
    );
    let private = &projection.records[4];
    let public = project_record(private, &Default::default()).unwrap();
    assert_eq!(
        public.privacy.left_out.get(&PrivateKind::Thinking),
        Some(&1)
    );
    assert!(
        !serde_json::to_string(&public.record)
            .unwrap()
            .contains("PRIVATE_CURSOR")
    );
    assert!(
        serde_json::to_string(&public.record)
            .unwrap()
            .contains("after tools")
    );
    let granted = project_record(private, &[PrivateKind::Thinking].into()).unwrap();
    assert!(
        serde_json::to_string(&granted.record)
            .unwrap()
            .contains("PRIVATE_CURSOR")
    );
}

#[test]
fn tool_only_cursor_envelope_keeps_one_message_with_references_and_separate_calls() {
    let text=json!({"role":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"path":"first.rs"}},{"type":"tool_use","name":"Bash","input":{"command":"pwd"}}]}}).to_string()+"\n";
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("cursor_export_jsonl").unwrap();
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: text.as_bytes(),
            source_identity: "tool-only-cursor",
        });
    let projected =
        project_positioned(&SourceInputId::new("tool-only-cursor").unwrap(), &lines).unwrap();
    assert!(projected.gaps.is_empty());
    assert_eq!(
        projected.records.len(),
        3,
        "message envelope plus its two calls"
    );
    let message = projected
        .records
        .iter()
        .find(|record| matches!(record.body, PositionedBody::Message { .. }))
        .unwrap();
    assert_eq!(message.author, RecordAuthor::Assistant);
    assert_eq!(
        message.positions,
        [SourcePosition {
            line: 1,
            byte_offset: 0
        }]
    );
    let PositionedBody::Message { blocks } = &message.body else {
        unreachable!()
    };
    assert_eq!(blocks.len(), 2);
    let calls = projected
        .records
        .iter()
        .filter_map(|record| match &record.body {
            PositionedBody::ToolCall {
                call_id,
                tool,
                input,
            } => Some((record, call_id, tool, input)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    for (block, (record, call_id, _, _)) in blocks.iter().zip(&calls) {
        assert_eq!(
            block.block,
            MessageBlock::ToolCall {
                call_id: (*call_id).clone()
            }
        );
        assert_eq!(block.private_kind, None);
        assert_eq!(record.positions, message.positions);
        assert_eq!(record.author, RecordAuthor::Assistant);
    }
    assert_eq!(calls[0].2.as_str(), "Read");
    assert_eq!(calls[1].2.as_str(), "Bash");
    assert_eq!(
        serde_json::to_value(calls[0].3).unwrap(),
        json!({"path":"first.rs"})
    );
    assert_eq!(
        serde_json::to_value(calls[1].3).unwrap(),
        json!({"command":"pwd"})
    );
    assert!(
        project_record(message, &Default::default())
            .unwrap()
            .record
            .is_some()
    );
}
