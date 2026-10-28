#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use sunstack_rules::{CandidateEntry, EvidenceBinding};

#[test]
fn evidence_binding_contract_is_closed_without_enabling_any_admission_behavior() {
    let schema = serde_json::to_value(schemars::schema_for!(EvidenceBinding)).unwrap();
    let schema = jsonschema::options().offline().build(&schema).unwrap();
    for value in [
        json!({"kind":"bound","binding":{"kind":"payload","value":{"owner_fact":true}}}),
        json!({"kind":"unavailable","reason":"not_recorded"}),
        json!({"kind":"unavailable","reason":"withheld"}),
        json!({"kind":"unavailable","reason":"unknown"}),
    ] {
        assert!(schema.is_valid(&value));
        serde_json::from_value::<EvidenceBinding>(value).unwrap();
    }
    let invalid = json!({"kind":"unavailable","reason":"unknown","extra":true});
    assert!(!schema.is_valid(&invalid));
    assert!(serde_json::from_value::<EvidenceBinding>(invalid).is_err());
}

#[test]
fn timing_contract_distinguishes_a_recorded_duration_from_an_unmeasured_candidate() {
    let value = json!({
        "candidate_id":{"rule":{"rule_set":"set.json","rule_id":"one"},"occurrence":"occurrence"},
        "rule":{"rule_set":"set.json","rule_id":"one"},
        "occurrence":{"kind":"record","record":"record","author":"assistant","positions":[{"byte_offset":0,"line":1}],"matched_at":{"byte_offset":0,"line":1},"field":"/body","span":{"byte_start":0,"byte_end":0}},
        "outcome":{"kind":"not_processed","reason":"run_stopped"},"answers":[],"privacy_view":{"grants":[]}
    });
    let mut entry: CandidateEntry = serde_json::from_value(value).unwrap();
    assert_eq!(entry.elapsed_ms, None);
    assert!(
        serde_json::to_value(&entry)
            .unwrap()
            .get("elapsed_ms")
            .is_none()
    );
    entry.elapsed_ms = Some(23);
    let wire = serde_json::to_value(&entry).unwrap();
    assert_eq!(wire["elapsed_ms"], 23);
    assert_eq!(
        serde_json::from_value::<CandidateEntry>(wire)
            .unwrap()
            .elapsed_ms,
        Some(23)
    );
}
