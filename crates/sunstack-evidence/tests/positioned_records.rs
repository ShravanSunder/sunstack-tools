#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
#[test]
fn an_orphan_result_remains_positioned_and_reports_one_gap_in_both_consumers() {
    let text = "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"missing-call\",\"output\":\"observed result\"}}\n";
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("codex_rollout_jsonl").unwrap();
    let capture = CapturedTranscript {
        bytes: text.as_bytes(),
        source_identity: "fixture",
    };
    let events = registry.event_reader(&format).unwrap().lines(&capture);
    let positioned = project_positioned(&SourceInputId::new("orphan").unwrap(), &events).unwrap();
    let legacy = registry.adapter(&format).unwrap().normalize(&capture);
    let expected = vec![CoverageGap::OrphanResult {
        call_id: CallId::new("missing-call").unwrap(),
        position: 0,
    }];
    assert_eq!(positioned.records.len(), 1);
    assert_eq!(positioned.gaps, expected);
    assert_eq!(legacy.gaps, expected);
    assert!(legacy.records.is_empty());
}
#[test]
fn three_streamed_lines_keep_all_positions_and_the_third_blocks_actual_origin() {
    let lines = ["first", "second", "MATCH third"].map(|text| serde_json::json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":text}]}}).to_string()+"\n");
    let text = lines.concat();
    let registry = AdapterRegistry::builtins().unwrap();
    let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
    let events = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: text.as_bytes(),
            source_identity: "fixture",
        });
    let records = project_positioned(&SourceInputId::new("fixture").unwrap(), &events).unwrap();
    assert_eq!(records.records.len(), 1);
    assert_eq!(
        records.records[0]
            .positions
            .iter()
            .map(|position| position.line)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let pass: MechanicalPass = serde_json::from_value(serde_json::json!({"predicate":{"kind":"regex","field":"/body/blocks/2/block/text","pattern":"MATCH"}})).unwrap();
    let matches = records
        .records
        .as_slice()
        .discover(Some(&pass), 20)
        .unwrap();
    let SourceOccurrence::Record { matched_at, .. } = matches.matches[0].occurrence;
    assert_eq!(matched_at.line, 3);
    assert_eq!(
        matched_at.byte_offset,
        (lines[0].len() + lines[1].len()) as u64
    );
    assert!(
        serde_json::to_value(&records.records[0])
            .unwrap()
            .get("field_positions")
            .is_none()
    );
}
