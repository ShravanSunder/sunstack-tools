#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;

#[test]
fn an_empty_thinking_line_contributes_a_private_block_and_its_stream_position() {
    // Arrange: a real Claude shape, with an empty thinking update between public updates.
    let lines = [
        serde_json::json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":"first"}]}}),
        serde_json::json!({"type":"assistant","message":{"id":"stream","content":[{"type":"thinking","thinking":""}]}}),
        serde_json::json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":"last"}]}}),
    ].map(|line| line.to_string()+"\n");
    let text = lines.concat();
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "empty-thinking",
    };
    // Act: the positioned path reads the event, and the old consumer folds the same capture.
    let events = registry.event_reader(&format).unwrap().lines(&capture);
    let positioned = project_positioned(&SourceInputId::new("fixture").unwrap(), &events).unwrap();
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    // Assert: position and privacy survive even when there are zero thinking characters.
    assert!(positioned.gaps.is_empty());
    assert_eq!(positioned.records.len(), 1);
    let record = &positioned.records[0];
    assert_eq!(
        record
            .positions
            .iter()
            .map(|position| position.line)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let PositionedBody::Message { blocks } = &record.body else {
        unreachable!()
    };
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[1].private_kind, Some(PrivateKind::Thinking));
    assert_eq!(
        blocks[1].block,
        MessageBlock::Thinking {
            text: String::new()
        }
    );
    let projected = project_record(record, &Default::default()).unwrap();
    assert_eq!(
        projected.privacy.left_out.get(&PrivateKind::Thinking),
        Some(&1)
    );
    let public = projected.record.unwrap();
    assert_eq!(public.positions, record.positions);
    let PositionedBody::Message { blocks } = public.body else {
        unreachable!()
    };
    assert_eq!(blocks.len(), 2);
    let granted = project_record(record, &[PrivateKind::Thinking].into()).unwrap();
    assert_eq!(granted.record.as_ref(), Some(record));
    assert!(granted.privacy.left_out.is_empty());
    // The historical fold still rejects this whole line, preserving its exact old gap and body.
    assert_eq!(
        legacy.gaps,
        vec![CoverageGap::MalformedRecord {
            position: lines[0].len() as u64
        }]
    );
    assert_eq!(legacy.records.len(), 1);
    assert_eq!(
        legacy.records[0].position,
        (lines[0].len() + lines[1].len()) as u64
    );
    assert_eq!(
        legacy.records[0].body,
        ConversationBody::AssistantMessage {
            blocks: vec![
                MessageBlock::Text {
                    text: "first".into()
                },
                MessageBlock::Text {
                    text: "last".into()
                }
            ]
        }
    );
}

#[test]
fn an_empty_thinking_line_keeps_legacy_whole_line_rejection_and_identity_effects() {
    let cases = [
        concat!(
            "{\"type\":\"assistant\",\"message\":{\"id\":\"stream\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"\"},{\"type\":\"tool_use\",\"id\":\"call\",\"name\":\"Read\",\"input\":{}}]}}\n",
            "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"call\",\"content\":\"result\"}]}}\n"
        ),
        concat!(
            "{\"type\":\"assistant\",\"message\":{\"id\":\"stream\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"\"}]}}\n",
            "{\"type\":\"user\",\"origin\":{\"kind\":\"user\"},\"message\":{\"id\":\"stream\",\"content\":\"public user text\"}}\n"
        ),
    ];
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    for (index, text) in cases.into_iter().enumerate() {
        let legacy = registry
            .adapter(&format)
            .unwrap()
            .normalize(&CapturedTranscript {
                bytes: text.as_bytes(),
                source_identity: "empty-thinking-legacy",
            });
        if index == 0 {
            let lines = registry
                .event_reader(&format)
                .unwrap()
                .lines(&CapturedTranscript {
                    bytes: text.as_bytes(),
                    source_identity: "empty-thinking-positioned",
                });
            let positioned =
                project_positioned(&SourceInputId::new("mixed").unwrap(), &lines).unwrap();
            assert!(
                positioned.gaps.is_empty(),
                "the positioned consumer owns the valid call on its private line"
            );
            assert_eq!(positioned.records.len(), 3);
            assert!(matches!(
                positioned.records[1].body,
                PositionedBody::ToolCall { .. }
            ));
            assert!(matches!(
                positioned.records[2].body,
                PositionedBody::ToolResult { .. }
            ));
            assert_eq!(positioned.records[1].positions[0].line, 1);
            assert_eq!(positioned.records[2].positions[0].line, 2);
            assert!(
                legacy.records.is_empty(),
                "the old parser rejected the tool call on the same malformed line"
            );
            assert_eq!(
                legacy.gaps,
                vec![
                    CoverageGap::MalformedRecord { position: 0 },
                    CoverageGap::OrphanResult {
                        call_id: CallId::new("call").unwrap(),
                        position: text.find('\n').unwrap() as u64 + 1
                    }
                ]
            );
        } else {
            assert_eq!(
                legacy.records.len(),
                1,
                "the rejected line never claimed the message id in the old parser"
            );
            assert!(matches!(
                legacy.records[0].body,
                ConversationBody::UserMessage { .. }
            ));
            assert_eq!(
                legacy.gaps,
                vec![CoverageGap::MalformedRecord { position: 0 }]
            );
        }
    }
}

#[test]
fn source_lines_keep_partial_syntax_effects_for_both_consumers() {
    let first = r#"{"type":"assistant","message":{"id":"partial","content":[{"type":"text","text":"kept"},{"type":"tool_use","id":"good","name":"Read","input":{}},{"type":"tool_use","id":"bad","name":"Read"}]}}"#.to_owned()+"\n";
    let text = first.clone()
        + r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"good","content":"result"}]}}"#
        + "\n";
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "partial-lines",
    };
    let lines = registry.event_reader(&format).unwrap().lines(&capture);
    assert_eq!(lines.len(), 2);
    assert!(matches!(lines[0].legacy, LegacyDisposition::Folded));
    assert_eq!(lines[0].events.len(), 3);
    let positioned = project_positioned(&SourceInputId::new("partial").unwrap(), &lines).unwrap();
    assert_eq!(positioned.records.len(), 3);
    assert_eq!(
        positioned.gaps,
        vec![CoverageGap::MalformedRecord { position: 0 }]
    );
    assert_eq!(
        positioned.records[2].positions[0],
        SourcePosition {
            line: 2,
            byte_offset: first.len() as u64
        }
    );
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    assert_eq!(legacy.records.len(), 2);
    assert_eq!(legacy.gaps, positioned.gaps);
}

#[test]
fn rejected_empty_thinking_keeps_prior_syntax_gaps_but_never_claims_a_parent() {
    let values = [
        serde_json::json!({"type":"assistant","parentUuid":"parent","message":{"id":"stream","content":[{"type":"text","text":"first"}]}}),
        serde_json::json!({"type":"assistant","parentUuid":"parent","message":{"id":"stream","content":[{"type":"unknown_block"},{"type":"thinking","thinking":""}]}}),
        serde_json::json!({"type":"assistant","parentUuid":"parent","message":{"id":"stream","content":[{"type":"text","text":"last"}]}}),
    ].map(|value| value.to_string()+"\n");
    let text = values.concat();
    let second = values[0].len() as u64;
    let third = (values[0].len() + values[1].len()) as u64;
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "parent-validity",
    };
    let lines = registry.event_reader(&format).unwrap().lines(&capture);
    let positioned = project_positioned(&SourceInputId::new("parents").unwrap(), &lines).unwrap();
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    assert_eq!(
        legacy.gaps,
        vec![
            CoverageGap::UnsupportedRecord { position: second },
            CoverageGap::MalformedRecord { position: second },
            CoverageGap::AmbiguousTurn {
                position: third,
                reason: AmbiguousReason::UnresolvedBranch
            },
        ]
    );
    assert_eq!(
        positioned.gaps,
        vec![
            CoverageGap::UnsupportedRecord { position: second },
            CoverageGap::AmbiguousTurn {
                position: second,
                reason: AmbiguousReason::UnresolvedBranch
            },
            CoverageGap::AmbiguousTurn {
                position: third,
                reason: AmbiguousReason::UnresolvedBranch
            },
        ]
    );
}

#[test]
fn unsupported_envelopes_never_claim_message_parent_links() {
    let lines = [
        serde_json::json!({"type":"progress","parentUuid":"shared"}),
        serde_json::json!({"type":"assistant","parentUuid":"shared","message":{"id":"one","content":"one"}}),
        serde_json::json!({"type":"assistant","parentUuid":"shared","message":{"id":"two","content":"two"}}),
    ].map(|line| line.to_string()+"\n");
    let text = lines.concat();
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "unsupported-parent",
    };
    let source_lines = registry.event_reader(&format).unwrap().lines(&capture);
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    let positioned =
        project_positioned(&SourceInputId::new("parents").unwrap(), &source_lines).unwrap();
    let expected = vec![
        CoverageGap::UnsupportedRecord { position: 0 },
        CoverageGap::AmbiguousTurn {
            position: (lines[0].len() + lines[1].len()) as u64,
            reason: AmbiguousReason::UnresolvedBranch,
        },
    ];
    assert_eq!(legacy.gaps, expected);
    assert_eq!(positioned.gaps, expected);
}
