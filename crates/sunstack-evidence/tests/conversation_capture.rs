#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{fs::OpenOptions, io::Write};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
fn selection() -> ConversationSelection {
    ConversationSelection {
        turns: TurnWindow::SinceCursor,
        user_messages: Inclusion::Included,
        assistant_messages: AssistantSelection::All,
        tool_use: ToolUseSelection::Included {
            tools: ToolFilter::All,
            results: Inclusion::Included,
        },
        hook_injected: Inclusion::Excluded,
    }
}
fn request(path: &std::path::Path) -> ConversationCaptureRequest {
    ConversationCaptureRequest {
        transcript_path: path.into(),
        format: TranscriptFormatName::new("claude_code_jsonl").unwrap(),
        cursor: None,
        final_assistant_text: None,
        max_bytes: 100000,
    }
}
async fn capture(request: ConversationCaptureRequest) -> ConversationCapture {
    capture_conversation(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
fn append(path: &std::path::Path, bytes: &[u8]) {
    OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
#[tokio::test]
async fn real_append_cursor_returns_whole_streamed_message_and_late_result() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("conversation.jsonl");
    let first = b"{\"type\":\"assistant\",\"message\":{\"id\":\"a1\",\"content\":[{\"type\":\"text\",\"text\":\"first\"},{\"type\":\"tool_use\",\"id\":\"c1\",\"name\":\"Read\",\"input\":{\"file_path\":\"/fixture\"}}]}}\n";
    std::fs::write(&path, first).unwrap();
    let initial = capture(request(&path)).await;
    assert_eq!(initial.coverage.cursor_basis, CursorBasis::NoCursorSupplied);
    assert_eq!(
        select_conversation(&initial, &selection())
            .unwrap()
            .records
            .len(),
        2
    );
    assert_eq!(initial.coverage.bytes_read, first.len() as u64);
    assert_eq!(initial.coverage.bytes_parsed, first.len() as u64);
    let mut next_request = request(&path);
    next_request.cursor = Some(initial.cursor.encode().unwrap());
    let unchanged = capture(next_request.clone()).await;
    assert!(
        select_conversation(&unchanged, &selection())
            .unwrap()
            .records
            .is_empty()
    );
    let stream = b"{\"type\":\"assistant\",\"message\":{\"id\":\"a1\",\"content\":[{\"type\":\"text\",\"text\":\"second\"}]}}\n";
    append(&path, stream);
    let streamed = capture(next_request.clone()).await;
    let selected = select_conversation(&streamed, &selection()).unwrap();
    assert_eq!(selected.records.len(), 1);
    assert_eq!(selected.records[0].record_id.as_str(), "a1");
    let ConversationBody::AssistantMessage { blocks } = &selected.records[0].body else {
        unreachable!()
    };
    assert_eq!(blocks.len(), 3); // first text + tool reference + new text, never a last-block projection
    assert_eq!(
        streamed.coverage.bytes_read,
        (first.len() + stream.len()) as u64
    );
    assert_eq!(streamed.coverage.bytes_parsed, streamed.coverage.bytes_read);
    next_request.cursor = Some(streamed.cursor.encode().unwrap());
    let result = b"{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"c1\",\"content\":\"observed\"}]}}\n";
    let split = result.len() / 2;
    append(&path, &result[..split]);
    let partial = capture(next_request.clone()).await;
    assert_eq!(partial.committed_bytes, (first.len() + stream.len()) as u64);
    assert_eq!(
        partial.tail,
        CaptureTail::Incomplete {
            bytes: split as u64
        }
    );
    assert_eq!(
        partial.coverage.bytes_read,
        partial.committed_bytes + split as u64
    );
    assert_eq!(partial.coverage.bytes_parsed, partial.committed_bytes);
    append(&path, &result[split..]);
    let completed = capture(next_request).await;
    let delta = select_conversation(&completed, &selection()).unwrap();
    assert_eq!(delta.records.len(), 1);
    let ConversationBody::ToolUse(tool) = &delta.records[0].body else {
        unreachable!()
    };
    assert_eq!(tool.call_id.as_str(), "c1");
    assert_eq!(tool.outcome, ToolOutcome::Succeeded);
    assert_eq!(
        tool.result,
        ToolResult::FullObserved {
            text: "observed".into()
        }
    );
}
#[tokio::test]
async fn late_supplement_repeats_until_transcript_materializes_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("transcript");
    std::fs::write(&path, b"").unwrap();
    let initial = capture(request(&path)).await;
    let mut next = request(&path);
    next.cursor = Some(initial.cursor.encode().unwrap());
    next.final_assistant_text = Some("final".into());
    for _ in 0..2 {
        let supplemented = capture(next.clone()).await;
        let delta = select_conversation(&supplemented, &selection()).unwrap();
        assert_eq!(delta.records.len(), 1);
        assert_eq!(
            delta.records[0].provenance,
            RecordProvenance::HookSupplement
        );
        assert_eq!(delta.records[0].injection, InjectionOrigin::NotInjected);
        assert!(matches!(
            delta.records[0].body,
            ConversationBody::AssistantMessage { .. }
        ));
        next.cursor = Some(supplemented.cursor.encode().unwrap());
    }
    append(
        &path,
        b"{\"type\":\"assistant\",\"message\":{\"id\":\"final-id\",\"content\":\"final\"}}\n",
    );
    let materialized = capture(next).await;
    assert!(materialized.supplement.is_none());
    assert_eq!(
        select_conversation(&materialized, &selection())
            .unwrap()
            .records
            .len(),
        1
    );
}
#[tokio::test]
async fn cursor_refuses_same_length_rewrite_truncation_and_identical_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("transcript");
    let bytes = b"{\"type\":\"assistant\",\"message\":{\"content\":\"A\"}}\n";
    std::fs::write(&path, bytes).unwrap();
    let original = capture(request(&path)).await;
    let mut next = request(&path);
    next.cursor = Some(original.cursor.encode().unwrap());
    let mut rewritten = bytes.to_vec();
    let index = rewritten.iter().position(|byte| *byte == b'A').unwrap();
    rewritten[index] = b'B';
    for mutation in [rewritten, Vec::new()] {
        std::fs::write(&path, mutation).unwrap();
        let error = capture_conversation(
            next.clone(),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, EvidenceCode::TranscriptDiscontinuous);
    }
    let replacement = root.path().join("replacement");
    std::fs::write(&replacement, bytes).unwrap();
    std::fs::rename(replacement, &path).unwrap();
    assert_eq!(
        capture_conversation(
            next,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new()
        )
        .await
        .unwrap_err()
        .code,
        EvidenceCode::TranscriptDiscontinuous
    );
}
#[tokio::test]
async fn pinned_public_literals_have_whole_message_records_and_explicit_gaps() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("transcript");
    for (fixture, expected) in [
        (
            include_bytes!("fixtures/test_parse_user_message_with_tool_use.jsonl").as_slice(),
            2,
        ),
        (
            include_bytes!("fixtures/test_parse_user_message_with_mixed_content.jsonl").as_slice(),
            2,
        ),
    ] {
        std::fs::write(&path, fixture).unwrap();
        let capture = capture(request(&path)).await;
        assert_eq!(capture.records.len(), expected);
        assert_eq!(capture.records[0].record_id.as_str(), "message:0");
        let ConversationBody::UserMessage { blocks } = &capture.records[0].body else {
            unreachable!()
        };
        if fixture == include_bytes!("fixtures/test_parse_user_message_with_mixed_content.jsonl") {
            assert_eq!(
                blocks,
                &vec![
                    MessageBlock::Text {
                        text: "Here's what I found:".into()
                    },
                    MessageBlock::Text {
                        text: "What do you think?".into()
                    }
                ]
            );
            let ConversationBody::ToolUse(tool) = &capture.records[1].body else {
                unreachable!()
            };
            assert_eq!(tool.call_id.as_str(), "use_1");
            assert_eq!(tool.category, ToolCategory::Search);
            assert_eq!(tool.outcome, ToolOutcome::Succeeded);
        } else {
            let ConversationBody::ToolUse(tool) = &capture.records[1].body else {
                unreachable!()
            };
            assert_eq!(tool.call_id.as_str(), "tool_456");
            assert_eq!(tool.category, ToolCategory::ReadFile);
            assert_eq!(tool.outcome, ToolOutcome::ResultMissing);
            assert_eq!(tool.result, ToolResult::Absent);
        }
    }
    std::fs::write(
        &path,
        include_bytes!("fixtures/test_parse_user_message_with_tool_result.jsonl"),
    )
    .unwrap();
    let orphan = capture(request(&path)).await;
    assert!(orphan.records.is_empty());
    assert!(
        matches!(&orphan.gaps[..],[CoverageGap::OrphanResult{call_id,position:0}] if call_id.as_str()=="tool_789")
    );
    let mut codex = request(&path);
    codex.format = TranscriptFormatName::new("codex_rollout_jsonl").unwrap();
    let fixture = include_bytes!("fixtures/codex-public.jsonl");
    std::fs::write(&path, fixture).unwrap();
    let unsupported = capture(codex.clone()).await;
    assert!(unsupported.records.is_empty());
    assert_eq!(unsupported.gaps.len(), 3);
    let expected_positions = fixture
        .split_inclusive(|byte| *byte == b'\n')
        .scan(0_u64, |offset, line| {
            let position = *offset;
            *offset += line.len() as u64;
            Some(position)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        unsupported.gaps,
        expected_positions
            .into_iter()
            .map(|position| CoverageGap::UnsupportedRecord { position })
            .collect::<Vec<_>>()
    );
    std::fs::write(&path, include_bytes!("fixtures/codex-composed.jsonl")).unwrap();
    let composed = capture(codex).await;
    assert_eq!(composed.records.len(), 1);
    let ConversationBody::ToolUse(tool) = &composed.records[0].body else {
        unreachable!()
    };
    assert_eq!(tool.call_id.as_str(), "first");
    assert_eq!(tool.category, ToolCategory::Mcp);
    assert_eq!(tool.outcome, ToolOutcome::ResultPresentStatusUnknown);
    assert_eq!(
        tool.result,
        ToolResult::FullObserved {
            text: "Sent.".into()
        }
    );
}
#[tokio::test]
async fn generic_projection_windows_precede_filters_and_preserve_unknown_injection() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("transcript");
    let mut bytes = Vec::new();
    for turn in 1..=6 {
        for value in [
            serde_json::json!({"type":"user","uuid":format!("u{turn}"),"message":{"content":format!("question {turn}")}}),
            serde_json::json!({"type":"assistant","message":{"id":format!("a{turn}"),"content":[{"type":"text","text":"first block"},{"type":"text","text":"last block"},{"type":"tool_use","id":format!("call{turn}"),"name":"Read","input":{"file_path":"/fixture"}}]}}),
        ] {
            bytes.extend(serde_json::to_vec(&value).unwrap());
            bytes.push(b'\n');
        }
    }
    std::fs::write(&path, bytes).unwrap();
    let source = capture(request(&path)).await;
    let mut projection = selection();
    projection.turns = TurnWindow::Last {
        count: 5.try_into().unwrap(),
    };
    assert_eq!(
        select_conversation(&source, &projection)
            .unwrap()
            .records
            .len(),
        15
    );
    projection.tool_use = ToolUseSelection::Excluded;
    let without_tools = select_conversation(&source, &projection).unwrap();
    assert_eq!(without_tools.records.len(), 10);
    assert_eq!(without_tools.records[0].record_id.as_str(), "u2");
    projection.turns = TurnWindow::All;
    projection.assistant_messages = AssistantSelection::Excluded;
    let users = select_conversation(&source, &projection).unwrap();
    assert_eq!(users.records.len(), 6);
    assert!(
        users
            .records
            .iter()
            .all(|record| record.injection == InjectionOrigin::Unknown)
    );
    projection.user_messages = Inclusion::Excluded;
    projection.assistant_messages = AssistantSelection::FinalOnly;
    let final_message = select_conversation(&source, &projection).unwrap();
    assert_eq!(final_message.records.len(), 1);
    assert_eq!(final_message.records[0].record_id.as_str(), "a6");
    assert!(
        matches!(&final_message.records[0].body,ConversationBody::AssistantMessage{blocks} if blocks.len()==3)
    );
    projection.assistant_messages = AssistantSelection::Excluded;
    for tools in [
        ToolFilter::Named {
            names: vec!["Read".into()],
        },
        ToolFilter::Categories {
            categories: vec![ToolCategory::ReadFile],
        },
    ] {
        projection.tool_use = ToolUseSelection::Included {
            tools,
            results: Inclusion::Excluded,
        };
        let selected = select_conversation(&source, &projection).unwrap();
        assert_eq!(selected.records.len(), 6);
        assert!(selected.records.iter().all(|record|matches!(&record.body,ConversationBody::ToolUse(tool) if tool.result==ToolResult::Absent)));
    }
    projection.tool_use = ToolUseSelection::Included {
        tools: ToolFilter::Named {
            names: vec!["Search".into()],
        },
        results: Inclusion::Included,
    };
    assert!(
        select_conversation(&source, &projection)
            .unwrap()
            .records
            .is_empty()
    );
    projection.tool_use = ToolUseSelection::Excluded;
    assert_eq!(
        select_conversation(&source, &projection).unwrap_err().code,
        EvidenceCode::EmptyProjection
    );
}
#[tokio::test]
async fn malformed_unsupported_ambiguous_and_explicit_failure_stay_observable() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("transcript");
    let bytes=b"{bad json SENTINEL}\n{\"type\":\"future_record\"}\n{\"type\":\"user\",\"uuid\":\"u1\",\"parentUuid\":\"branch\",\"message\":{\"content\":\"one\"}}\n{\"type\":\"user\",\"uuid\":\"u2\",\"parentUuid\":\"branch\",\"message\":{\"content\":\"two\"}}\n{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"tool_error\",\"name\":\"Bash\",\"input\":{\"command\":\"cat a_file\"}}]}}\n";
    let result = include_bytes!("fixtures/test_parse_user_message_with_tool_result_error.jsonl");
    std::fs::write(&path, bytes).unwrap();
    append(&path, result);
    let capture = capture(request(&path)).await;
    assert!(
        capture
            .gaps
            .iter()
            .any(|gap| matches!(gap, CoverageGap::MalformedRecord { position: 0 }))
    );
    assert!(
        capture
            .gaps
            .iter()
            .any(|gap| matches!(gap, CoverageGap::UnsupportedRecord { .. }))
    );
    for reason in [
        AmbiguousReason::ConsecutiveUserMessages,
        AmbiguousReason::UnresolvedBranch,
    ] {
        assert!(capture.gaps.iter().any(
            |gap| matches!(gap,CoverageGap::AmbiguousTurn{reason:actual,..} if actual==&reason)
        ));
    }
    let tool = capture
        .records
        .iter()
        .find_map(|record| {
            if let ConversationBody::ToolUse(tool) = &record.body {
                Some(tool)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(tool.outcome, ToolOutcome::Failed);
    assert_eq!(tool.category, ToolCategory::Shell);
}

#[tokio::test]
async fn category_and_name_filters_exclude_contrasting_tools_and_remove_present_results() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("mixed-tools.jsonl");
    let messages = [
        serde_json::json!({"type":"assistant","message":{"id":"mixed","content":[
            {"type":"tool_use","id":"read","name":"Read","input":{"file_path":"/fixture"}},
            {"type":"tool_use","id":"shell","name":"Bash","input":{"command":"cat /fixture"}},
            {"type":"tool_use","id":"search","name":"Search","input":{"query":"fixture"}}
        ]}}),
        serde_json::json!({"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"read","content":"read-result"},
            {"type":"tool_result","tool_use_id":"shell","content":"shell-result"},
            {"type":"tool_result","tool_use_id":"search","content":"search-result"}
        ]}}),
    ];
    let bytes = messages
        .iter()
        .map(|value| format!("{}\n", serde_json::to_string(value).unwrap()))
        .collect::<String>();
    std::fs::write(&path, bytes).unwrap();
    let captured = capture(request(&path)).await;
    let mut projection = selection();
    projection.turns = TurnWindow::All;
    projection.user_messages = Inclusion::Excluded;
    projection.assistant_messages = AssistantSelection::Excluded;
    let all = select_conversation(&captured, &projection).unwrap();
    assert_eq!(all.records.len(), 3);
    assert!(all.records.iter().all(|record| matches!(&record.body, ConversationBody::ToolUse(tool) if matches!(tool.result, ToolResult::FullObserved { .. }) && tool.outcome == ToolOutcome::Succeeded)));
    for (filter, expected_id, expected_category) in [
        (
            ToolFilter::Categories {
                categories: vec![ToolCategory::ReadFile],
            },
            "read",
            ToolCategory::ReadFile,
        ),
        (
            ToolFilter::Named {
                names: vec!["Search".into()],
            },
            "search",
            ToolCategory::Search,
        ),
    ] {
        projection.tool_use = ToolUseSelection::Included {
            tools: filter,
            results: Inclusion::Included,
        };
        let selected = select_conversation(&captured, &projection).unwrap();
        assert_eq!(selected.records.len(), 1);
        let ConversationBody::ToolUse(tool) = &selected.records[0].body else {
            unreachable!()
        };
        assert_eq!(tool.call_id.as_str(), expected_id);
        assert_eq!(tool.category, expected_category);
    }
    projection.tool_use = ToolUseSelection::Included {
        tools: ToolFilter::All,
        results: Inclusion::Excluded,
    };
    let excluded = select_conversation(&captured, &projection).unwrap();
    assert_eq!(excluded.records.len(), 3);
    for (original, projected) in all.records.iter().zip(&excluded.records) {
        let (ConversationBody::ToolUse(original), ConversationBody::ToolUse(projected)) =
            (&original.body, &projected.body)
        else {
            unreachable!()
        };
        assert_eq!(projected.call_id, original.call_id);
        assert_eq!(projected.input, original.input);
        assert_eq!(projected.outcome, original.outcome);
        assert_eq!(projected.result, ToolResult::Absent);
    }
}

#[tokio::test]
async fn unknown_format_is_refused_at_capture_entry_before_file_acquisition() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(&root.path().join("does-not-exist"));
    request.format = TranscriptFormatName::new("unregistered-host").unwrap();
    let error = capture_conversation(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, EvidenceCode::UnknownTranscriptFormat);
}
