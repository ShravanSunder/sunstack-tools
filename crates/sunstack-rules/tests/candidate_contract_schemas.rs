#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/candidate_fixture.rs"]
mod candidate_fixture;
use serde_json::{Value, json};
use sunstack_rules::*;

fn reviewed(name: &str) -> jsonschema::Validator {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas");
    let schema: Value = serde_json::from_slice(&std::fs::read(root.join(name)).unwrap()).unwrap();
    jsonschema::options().offline().build(&schema).unwrap()
}
#[test]
fn strict_input_and_reviewed_schema_agree_on_valid_fields_and_rejections() {
    let root = tempfile::tempdir().unwrap();
    let valid = candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    let published = reviewed("evaluate-candidates-input.schema.json");
    let generated = serde_json::to_value(schemars::schema_for!(CandidateRunInput)).unwrap();
    let generated = jsonschema::options().offline().build(&generated).unwrap();
    assert!(published.is_valid(&valid));
    assert!(generated.is_valid(&valid));
    decode_candidate_run(&serde_json::to_vec(&valid).unwrap()).unwrap();
    let mut invalid = valid.clone();
    invalid["sources"][0]["source"]["misspelt"] = json!(true);
    assert!(!published.is_valid(&invalid));
    assert!(!generated.is_valid(&invalid));
    assert!(decode_candidate_run(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let duplicate = br#"{"sources":[],"sources":[]}"#;
    assert!(decode_candidate_run(duplicate).is_err());
}
#[test]
fn outcome_schema_and_decoder_reject_fields_outside_the_closed_variants() {
    let schema = reviewed("evaluate-candidates-outcome.schema.json");
    let report = CandidateRunReport {
        completeness: RunCompleteness::NothingFound {},
        candidates: vec![],
        dropped_samples: vec![],
        inputs: vec![],
        privacy: PrivacyReport::default(),
        usage: RunUsage::default(),
    };
    let valid = serde_json::to_value(CandidateRunOutcome::CandidateReport { report }).unwrap();
    assert!(schema.is_valid(&valid));
    serde_json::from_value::<CandidateRunOutcome>(valid.clone()).unwrap();
    let mut invalid = valid;
    invalid["report"]["completeness"]["unexpected"] = json!(true);
    assert!(!schema.is_valid(&invalid));
    assert!(serde_json::from_value::<CandidateRunOutcome>(invalid).is_err());
}
