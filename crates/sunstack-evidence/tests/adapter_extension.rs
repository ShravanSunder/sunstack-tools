#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
struct FourthHost {
    name: TranscriptFormatName,
}
impl TranscriptAdapter for FourthHost {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        assert_eq!(capture.bytes, b"external\n");
        assert!(!capture.source_identity.is_empty());
        let record_id = RecordId::new("external-message").unwrap();
        AdapterOutput {
            records: vec![ConversationRecord {
                record_id: record_id.clone(),
                position: 0,
                body: ConversationBody::AssistantMessage {
                    blocks: vec![MessageBlock::Text {
                        text: "external".into(),
                    }],
                },
                provenance: RecordProvenance::Transcript,
                injection: InjectionOrigin::NotInjected,
            }],
            turns: vec![ConversationTurn {
                opening: TurnOpening::CaptureStart,
                records: vec![record_id],
                boundary: TurnBoundary::Clear,
            }],
            gaps: vec![],
        }
    }
}
#[tokio::test]
async fn external_consumer_registers_new_format_without_library_edit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source");
    std::fs::write(&path, b"external\n").unwrap();
    let adapter = Arc::new(FourthHost {
        name: TranscriptFormatName::new("fourth-host").unwrap(),
    });
    let registry = AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(adapter)
        .unwrap();
    let capture = capture_conversation(
        ConversationCaptureRequest {
            transcript_path: path,
            format: TranscriptFormatName::new("fourth-host").unwrap(),
            cursor: None,
            final_assistant_text: None,
            max_bytes: 100,
        },
        registry,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(capture.records.len(), 1);
    assert_eq!(capture.records[0].record_id.as_str(), "external-message");
    assert!(
        AdapterRegistry::builtins()
            .unwrap()
            .with_adapter(Arc::new(FourthHost {
                name: TranscriptFormatName::new("claude_code_jsonl").unwrap()
            }))
            .is_err()
    );
}
#[test]
fn adapter_output_validation_rejects_each_structural_contradiction() {
    let adapter = FourthHost {
        name: TranscriptFormatName::new("fourth-host").unwrap(),
    };
    let valid = adapter.normalize(&CapturedTranscript {
        bytes: b"external\n",
        source_identity: "file-object",
    });
    assert!(validate_adapter_output(&valid, 9).is_ok());
    let mut mutations = Vec::new();
    let mut duplicate = valid.clone();
    duplicate.records.push(duplicate.records[0].clone());
    mutations.push(duplicate);
    let mut outside = valid.clone();
    outside.records[0].position = 9;
    mutations.push(outside);
    let mut foreign = valid.clone();
    foreign.turns[0]
        .records
        .push(RecordId::new("foreign").unwrap());
    mutations.push(foreign);
    for outcome in [ToolOutcome::Succeeded, ToolOutcome::Failed] {
        let mut absent = valid.clone();
        absent.records[0].body = ConversationBody::ToolUse(ToolUse {
            call_id: CallId::new("native").unwrap(),
            tool_name: "Read".into(),
            category: ToolCategory::ReadFile,
            input: StructuredJson::Null,
            outcome,
            result: ToolResult::Absent,
        });
        mutations.push(absent);
    }
    let mut clipped = valid.clone();
    clipped.records[0].body = ConversationBody::ToolUse(ToolUse {
        call_id: CallId::new("native").unwrap(),
        tool_name: "Read".into(),
        category: ToolCategory::ReadFile,
        input: StructuredJson::Null,
        outcome: ToolOutcome::Succeeded,
        result: ToolResult::Clipped {
            text: "long result".into(),
            original_bytes: OriginalBytes::Known { bytes: 1 },
        },
    });
    mutations.push(clipped.clone());
    if let ConversationBody::ToolUse(tool) = &mut clipped.records[0].body {
        tool.result = ToolResult::FullObserved {
            text: "observed".into(),
        };
    }
    let mut same_call = clipped.clone();
    let mut second = clipped.records[0].clone();
    second.record_id = RecordId::new("second").unwrap();
    same_call.turns[0].records.push(second.record_id.clone());
    same_call.records.push(second);
    mutations.push(same_call);
    for mutation in mutations {
        assert_eq!(
            validate_adapter_output(&mutation, 9).unwrap_err().code,
            EvidenceCode::AdapterOutputInvalid
        );
    }
    assert!(validate_adapter_output(&clipped, 9).is_ok());
}
#[test]
fn diagnostic_debug_and_json_never_copy_received_content() {
    let sentinel = "SECRET_TRANSCRIPT_OR_PAYLOAD_VALUE";
    let mut output = AdapterOutput {
        records: vec![ConversationRecord {
            record_id: RecordId::new(sentinel).unwrap(),
            position: 99,
            body: ConversationBody::UserMessage {
                blocks: vec![MessageBlock::Text {
                    text: sentinel.into(),
                }],
            },
            provenance: RecordProvenance::Transcript,
            injection: InjectionOrigin::Unknown,
        }],
        turns: vec![],
        gaps: vec![],
    };
    let error = validate_adapter_output(&output, 1).unwrap_err();
    let mut diagnostics = DiagnosticList::default();
    for _ in 0..105 {
        diagnostics.push(Diagnostic::new(error.code));
    }
    assert_eq!(diagnostics.items.len(), 100);
    assert_eq!(diagnostics.omitted_count, 5);
    let serialized = serde_json::to_string(&diagnostics).unwrap();
    assert!(!serialized.contains(sentinel));
    assert!(!format!("{diagnostics:?}").contains(sentinel));
    assert!(
        diagnostics
            .items
            .iter()
            .all(|item| item.guidance.chars().count() <= 512)
    );
    output.records.clear();
    assert!(!format!("{error:?}").contains(sentinel));
}

struct SuppliedOutputAdapter {
    name: TranscriptFormatName,
    output: AdapterOutput,
}
impl TranscriptAdapter for SuppliedOutputAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        assert_eq!(capture.bytes, b"external\n");
        self.output.clone()
    }
}

#[tokio::test]
async fn invalid_external_output_is_refused_by_real_capture_entry() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("external");
    std::fs::write(&path, b"external\n").unwrap();
    let fourth = FourthHost {
        name: TranscriptFormatName::new("invalid-external").unwrap(),
    };
    let mut invalid = fourth.normalize(&CapturedTranscript {
        bytes: b"external\n",
        source_identity: "synthetic",
    });
    invalid.records[0].position = 9; // one past the captured byte positions
    let registry = AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(Arc::new(SuppliedOutputAdapter {
            name: fourth.name.clone(),
            output: invalid,
        }))
        .unwrap();
    let error = capture_conversation(
        ConversationCaptureRequest {
            transcript_path: path,
            format: fourth.name,
            cursor: None,
            final_assistant_text: None,
            max_bytes: 100,
        },
        registry,
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, EvidenceCode::AdapterOutputInvalid);
}

#[tokio::test]
async fn valid_clipped_results_keep_known_and_unknown_sizes_through_capture_and_selection() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("external");
    std::fs::write(&path, b"external\n").unwrap();
    let fourth = FourthHost {
        name: TranscriptFormatName::new("clipped-external").unwrap(),
    };
    for original_bytes in [OriginalBytes::Known { bytes: 40 }, OriginalBytes::Unknown] {
        let mut output = fourth.normalize(&CapturedTranscript {
            bytes: b"external\n",
            source_identity: "synthetic",
        });
        let expected = ToolResult::Clipped {
            text: "retained".into(),
            original_bytes,
        };
        output.records[0].body = ConversationBody::ToolUse(ToolUse {
            call_id: CallId::new("clipped-call").unwrap(),
            tool_name: "external-tool".into(),
            category: ToolCategory::Unknown,
            input: StructuredJson::Null,
            outcome: ToolOutcome::ResultPresentStatusUnknown,
            result: expected.clone(),
        });
        let registry = AdapterRegistry::builtins()
            .unwrap()
            .with_adapter(Arc::new(SuppliedOutputAdapter {
                name: fourth.name.clone(),
                output,
            }))
            .unwrap();
        let capture = capture_conversation(
            ConversationCaptureRequest {
                transcript_path: path.clone(),
                format: fourth.name.clone(),
                cursor: None,
                final_assistant_text: None,
                max_bytes: 100,
            },
            registry,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let selected = select_conversation(
            &capture,
            &ConversationSelection {
                turns: TurnWindow::All {},
                user_messages: Inclusion::Excluded,
                assistant_messages: AssistantSelection::Excluded,
                tool_use: ToolUseSelection::Included {
                    tools: ToolFilter::All {},
                    results: Inclusion::Included,
                },
                hook_injected: Inclusion::Excluded,
            },
        )
        .unwrap();
        assert_eq!(selected.records.len(), 1);
        let ConversationBody::ToolUse(tool) = &selected.records[0].body else {
            unreachable!()
        };
        assert_eq!(tool.call_id.as_str(), "clipped-call");
        assert_eq!(tool.outcome, ToolOutcome::ResultPresentStatusUnknown);
        assert_eq!(tool.result, expected);
    }
}
