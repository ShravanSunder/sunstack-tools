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
