#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use sunstack_evidence::*;

#[test]
fn mapped_native_private_fields_keep_classification_public_siblings_and_record_identity() {
    for analysis in [false, true] {
        let (format, raw, private_pointer, public_pointer, kind) = if analysis {
            (
                "codex_rollout_jsonl",
                json!({"type":"response_item","public":"PUBLIC_MAPPED","payload":{"type":"message","role":"assistant","channel":"analysis","content":[{"type":"output_text","text":"PRIVATE_MAPPED"}]}}),
                "/payload/content/0/text",
                "/public",
                PrivateKind::AnalysisChannel,
            )
        } else {
            (
                "claude_code_jsonl",
                json!({"type":"assistant","message":{"id":"mixed","content":[{"type":"text","text":"PUBLIC_MAPPED"},{"type":"thinking","thinking":"PRIVATE_MAPPED"}]}}),
                "/message/content/1/thinking",
                "/message/content/0/text",
                PrivateKind::Thinking,
            )
        };
        let mapping = RecordMapping {
            fields: [
                ("public".into(), public_pointer.into()),
                ("private".into(), private_pointer.into()),
            ]
            .into(),
        };
        let (registry, format) = AdapterRegistry::builtins()
            .unwrap()
            .with_record_mapping(&TranscriptFormatName::new(format).unwrap(), &mapping)
            .unwrap();
        let text = raw.to_string() + "\n";
        let lines = registry
            .event_reader(&format)
            .unwrap()
            .lines(&CapturedTranscript {
                bytes: text.as_bytes(),
                source_identity: "mapped",
            });
        let records = project_positioned(&SourceInputId::new("mapped").unwrap(), &lines).unwrap();
        assert_eq!(records.records.len(), 1);
        let record = &records.records[0];
        assert_eq!(record.author, RecordAuthor::Assistant);
        let public = project_record(record, &Default::default()).unwrap();
        assert_eq!(public.privacy.left_out.get(&kind), Some(&1));
        let projected = public.record.unwrap();
        assert!(
            serde_json::to_string(&projected)
                .unwrap()
                .contains("PUBLIC_MAPPED")
        );
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("PRIVATE_MAPPED")
        );
        let granted = project_record(record, &[kind].into()).unwrap();
        assert_eq!(
            granted.record.as_ref().unwrap().record_id,
            projected.record_id
        );
        assert!(
            serde_json::to_string(&granted.record)
                .unwrap()
                .contains("PRIVATE_MAPPED")
        );
        let raw_value = serde_json::to_value(record).unwrap();
        assert_eq!(
            raw_value["body"]["field_privacy"]["private"]["private_kind"],
            json!(kind)
        );
    }
}

#[test]
fn mapping_opaque_bytes_never_materializes_them_even_when_granted() {
    let raw=json!({"type":"response_item","public":"PUBLIC_OPAQUE_SIBLING","payload":{"type":"reasoning","encrypted_content":"OPAQUE_CIPHER_MUST_NEVER_APPEAR","summary":[{"type":"summary_text","text":"READABLE_REASONING_SUMMARY"}]}}).to_string()+"\n";
    let mapping = RecordMapping {
        fields: [
            ("public".into(), "/public".into()),
            ("opaque".into(), "/payload/encrypted_content".into()),
            ("summary".into(), "/payload/summary/0/text".into()),
            ("summary".into(), "/payload/summary/0/text".into()),
        ]
        .into(),
    };
    let (registry, format) = AdapterRegistry::builtins()
        .unwrap()
        .with_record_mapping(
            &TranscriptFormatName::new("codex_rollout_jsonl").unwrap(),
            &mapping,
        )
        .unwrap();
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: raw.as_bytes(),
            source_identity: "opaque",
        });
    assert!(
        !serde_json::to_string(&lines[0].events)
            .unwrap()
            .contains("OPAQUE_CIPHER_MUST_NEVER_APPEAR")
    );
    let projected = project_positioned(&SourceInputId::new("opaque").unwrap(), &lines).unwrap();
    for grants in [Default::default(), [PrivateKind::EncryptedReasoning].into()] {
        let view = project_record(&projected.records[0], &grants).unwrap();
        assert_eq!(
            view.privacy.left_out.get(&PrivateKind::EncryptedReasoning),
            Some(&if grants.contains(&PrivateKind::EncryptedReasoning) {
                1
            } else {
                2
            })
        );
        let view = view.record.unwrap();
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("PUBLIC_OPAQUE_SIBLING")
        );
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("OPAQUE_CIPHER_MUST_NEVER_APPEAR")
        );
        assert_eq!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("READABLE_REASONING_SUMMARY"),
            grants.contains(&PrivateKind::EncryptedReasoning)
        );
        assert_eq!(view.record_id, projected.records[0].record_id);
    }
}
