#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use sunstack_evidence::*;

#[test]
fn native_projection_mint_and_owner_predicate_keep_source_position_and_ordinal() {
    let source = SourceInputId::new("namespace:with:colons").unwrap();
    let text = [
        json!({"type":"assistant","message":{"id":"first","content":"first"}}),
        json!({"type":"assistant","message":{"id":"second","content":"second"}}),
    ]
    .map(|record| record.to_string() + "\n");
    let bytes = text.concat();
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: bytes.as_bytes(),
            source_identity: source.as_str(),
        });
    let positioned = project_positioned(&source, &lines).unwrap();
    assert_eq!(positioned.records.len(), 2);
    assert_eq!(
        positioned.records[0].record_id.as_str(),
        "namespace:with:colons:0:0"
    );
    assert_eq!(
        positioned.records[1].record_id.as_str(),
        format!("namespace:with:colons:{}:1", text[0].len())
    );
    for (ordinal, record) in positioned.records.iter().enumerate() {
        assert_eq!(
            record.record_id,
            PositionedRecordId::for_source_record(
                &source,
                record.positions[0].byte_offset,
                ordinal
            )
            .unwrap()
        );
        assert!(record.record_id.belongs_to_source(&source));
        assert!(
            !record
                .record_id
                .belongs_to_source(&SourceInputId::new("namespace:with:colons:0").unwrap())
        );
        assert!(
            !record
                .record_id
                .belongs_to_source(&SourceInputId::new("other").unwrap())
        );
    }
}
#[test]
fn owner_predicate_requires_a_whole_canonical_native_identity_not_a_prefix_guess() {
    let source = SourceInputId::new("source").unwrap();
    let valid = PositionedRecordId::for_source_record(&source, 41, 2).unwrap();
    assert_eq!(valid.as_str(), "source:41:2");
    assert!(valid.belongs_to_source(&source));
    for value in [
        "record_1",
        "source_other:41:2",
        "source:41",
        "source:41:2:extra",
        "source:bad:2",
        "source:41:bad",
        "source:041:2",
        "source:41:02",
    ] {
        assert!(
            !PositionedRecordId::new(value)
                .unwrap()
                .belongs_to_source(&source),
            "{value}"
        );
    }
}
