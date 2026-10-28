#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::BTreeSet;
use sunstack_evidence::*;
#[test]
fn default_projection_removes_thinking_analysis_and_opaque_reasoning_without_changing_legacy() {
    let text = concat!(
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"channel\":\"analysis\",\"content\":[{\"type\":\"output_text\",\"text\":\"PRIVATE_ANALYSIS\"}]}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"type\":\"reasoning\",\"encrypted_content\":\"OPAQUE_SENTINEL\"}}\n"
    );
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("codex_rollout_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "fixture",
    };
    let events = registry.event_reader(&format).unwrap().lines(&capture);
    let positioned = project_positioned(&SourceInputId::new("fixture").unwrap(), &events).unwrap();
    assert_eq!(positioned.records.len(), 2);
    for (record, kind) in positioned.records.iter().zip([
        PrivateKind::AnalysisChannel,
        PrivateKind::EncryptedReasoning,
    ]) {
        let projected = project_record(record, &BTreeSet::new()).unwrap();
        assert!(projected.record.is_none());
        assert_eq!(projected.privacy.left_out.get(&kind), Some(&1));
    }
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    assert_eq!(legacy.records.len(), 1);
    assert!(
        serde_json::to_string(&legacy.records)
            .unwrap()
            .contains("PRIVATE_ANALYSIS")
    );
    assert_eq!(
        legacy.gaps,
        vec![CoverageGap::UnsupportedRecord {
            position: text.lines().next().unwrap().len() as u64 + 1
        }]
    );
    let granted = project_record(
        &positioned.records[0],
        &BTreeSet::from([PrivateKind::AnalysisChannel]),
    )
    .unwrap();
    assert!(
        serde_json::to_string(&granted.record)
            .unwrap()
            .contains("PRIVATE_ANALYSIS")
    );
}
#[test]
fn credential_text_and_nested_tool_arguments_are_redacted_and_counted() {
    let secret = "sk-PRIVATE_CREDENTIAL_SENTINEL";
    let record:PositionedRecord=serde_json::from_value(serde_json::json!({"record_id":"one","author":"assistant","positions":[{"byte_offset":0,"line":1}],"injection":"not_injected","body":{"kind":"tool_call","call_id":"c1","tool":"exec_command","input":{"nested":{"credential":secret}}}})).unwrap();
    let projected = project_record(&record, &BTreeSet::new()).unwrap();
    assert!(
        !serde_json::to_string(&projected.record)
            .unwrap()
            .contains(secret)
    );
    assert_eq!(
        projected.privacy.left_out.get(&PrivateKind::CredentialLike),
        Some(&1)
    );
    let granted = project_record(&record, &BTreeSet::from([PrivateKind::CredentialLike])).unwrap();
    assert!(
        serde_json::to_string(&granted.record)
            .unwrap()
            .contains(secret)
    );
}

#[test]
fn mixed_message_keeps_visible_blocks_and_only_the_final_channel_survives_by_default() {
    let text = [
        serde_json::json!({"type":"assistant","message":{"id":"mixed","content":[{"type":"text","text":"VISIBLE_TEXT"},{"type":"thinking","thinking":"PRIVATE_THINKING"}]}}),
    ].map(|value| value.to_string()).join("\n")+"\n";
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let events = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: text.as_bytes(),
            source_identity: "fixture",
        });
    let records = project_positioned(&SourceInputId::new("mixed").unwrap(), &events).unwrap();
    let projected = project_record(&records.records[0], &BTreeSet::new()).unwrap();
    let rendered = serde_json::to_string(&projected.record).unwrap();
    assert!(rendered.contains("VISIBLE_TEXT"));
    assert!(!rendered.contains("PRIVATE_THINKING"));
    assert_eq!(
        projected.privacy.left_out.get(&PrivateKind::Thinking),
        Some(&1)
    );
    let codex=["analysis","final"].map(|channel|serde_json::json!({"type":"response_item","payload":{"type":"message","role":"assistant","channel":channel,"content":[{"type":"output_text","text":"same text"}]}}).to_string()).join("\n")+"\n";
    let format = TranscriptFormatName::new("codex_rollout_jsonl").unwrap();
    let events = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: codex.as_bytes(),
            source_identity: "fixture",
        });
    let records = project_positioned(&SourceInputId::new("channels").unwrap(), &events).unwrap();
    assert!(
        project_record(&records.records[0], &BTreeSet::new())
            .unwrap()
            .record
            .is_none()
    );
    assert!(
        project_record(&records.records[1], &BTreeSet::new())
            .unwrap()
            .record
            .is_some()
    );
}
