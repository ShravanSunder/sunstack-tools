#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
#[test]
fn invalid_patterns_are_refused_even_when_the_source_has_no_records() {
    let pass: MechanicalPass = serde_json::from_value(
        serde_json::json!({"predicate":{"kind":"regex","field":"/body/input/cmd","pattern":"["}}),
    )
    .unwrap();
    let records: Vec<PositionedRecord> = vec![];
    assert!(records.as_slice().discover(Some(&pass), 20).is_err());
}
#[test]
fn decoded_tool_fields_match_and_rejected_samples_are_twenty_distinct_real_drops() {
    let mut records = Vec::new();
    for index in 0..51 {
        records.push(serde_json::from_value::<PositionedRecord>(serde_json::json!({"record_id":format!("record-{index}"),"author":"assistant","positions":[{"byte_offset":index*100,"line":index+1}],"injection":"not_injected","body":{"kind":"tool_call","call_id":format!("c{index}"),"tool":"exec_command","input":{"cmd":if index==50 {"MATCH tool argument".to_owned()} else {format!("drop {index}")}}}})).unwrap());
    }
    let pass:MechanicalPass=serde_json::from_value(serde_json::json!({"predicate":{"kind":"regex","field":"/body/input/cmd","pattern":"MATCH"}})).unwrap();
    let first = records.as_slice().discover(Some(&pass), 20).unwrap();
    let second = records.as_slice().discover(Some(&pass), 20).unwrap();
    assert_eq!(first.matches.len(), 1);
    assert_eq!(first.matches[0].record.record_id.as_str(), "record-50");
    assert_eq!(first.rejected_sample.len(), 20);
    assert_eq!(
        first
            .rejected_sample
            .iter()
            .map(|matched| &matched.occurrence)
            .collect::<Vec<_>>(),
        second
            .rejected_sample
            .iter()
            .map(|matched| &matched.occurrence)
            .collect::<Vec<_>>()
    );
    assert!(
        first
            .rejected_sample
            .iter()
            .all(|matched| matched.record.record_id.as_str() != "record-50")
    );
}
