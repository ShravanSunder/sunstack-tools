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

#[test]
fn outside_fact_bindings_and_explicit_discovery_match_the_published_input() {
    let root = tempfile::tempdir().unwrap();
    let mut input = candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    input["evidence"] = json!({"fact":{"kind":"unavailable","reason":"withheld"}});
    input["sources"][0]["evidence"] =
        json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"public"}}}});
    input["discovery"] = json!({"kind":"bounded","limits":{"max_files":10,"max_file_bytes":65536,"max_total_bytes":262144,"max_matches":100,"include_ignored":false,"max_unit_visits":100,"max_relationship_candidates":100,"max_fan_out":10,"max_history_commits":10,"max_local_time_ms":10000}});
    let schema = reviewed("evaluate-candidates-input.schema.json");
    assert!(schema.is_valid(&input));
    decode_candidate_run(&serde_json::to_vec(&input).unwrap()).unwrap();
    input["evidence"]["fact"]["extra"] = json!(true);
    assert!(!schema.is_valid(&input));
    assert!(decode_candidate_run(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn outside_evidence_use_and_unavailable_reasons_match_the_published_outcome() {
    let root = tempfile::tempdir().unwrap();
    let fixture = candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    let source: sunstack_evidence::SourceInput =
        serde_json::from_value(fixture["sources"][0]["source"].clone()).unwrap();
    let occurrence = json!({"kind":"record","record":"source:0:0","author":"assistant","positions":[{"byte_offset":0,"line":1}],"matched_at":{"byte_offset":0,"line":1},"field":"/body","span":{"byte_start":0,"byte_end":0}});
    let rule = json!({"rule_set":root.path().join("set.json"),"rule_id":"candidate.one"});
    let value = json!({"kind":"candidate_report","report":{"completeness":{"kind":"inconclusive","uncertain":0,"unresolved":1,"not_processed":0},"candidates":[{
        "candidate_id":{"rule":rule,"occurrence":"opaque"},"rule":rule,"declared_severity":"warn","occurrence":occurrence,
        "outcome":{"kind":"unresolved","at":{"kind":"verdict"},"missing":{"kind":"evidence_unavailable","evidence":"fact","reason":"withheld"}},
        "answers":[],"evidence":[{"input":"fact","binding":{"kind":"unavailable","reason":"withheld"},"content_identity":null}],"privacy_view":{"grants":[]}
    }],"dropped_samples":[],"inputs":[{"input_id":source.source_identity().unwrap(),"location":{"kind":"unlocatable","reason":"missing"},"records":0,"unfit":[],"capture":"incomplete","discovery":[]}],"privacy":{"left_out":[],"granted":[]},"usage":{"requests":0,"attempts":0,"input_tokens":0,"output_tokens":0,"cost_usd":null,"wall_time_ms":0}}});
    let schema = reviewed("evaluate-candidates-outcome.schema.json");
    assert!(schema.is_valid(&value));
    serde_json::from_value::<CandidateRunOutcome>(value.clone()).unwrap();
    let mut bad = value;
    bad["report"]["candidates"][0]["evidence"][0]["extra"] = json!(true);
    assert!(!schema.is_valid(&bad));
    assert!(serde_json::from_value::<CandidateRunOutcome>(bad).is_err());
}

#[test]
fn router_and_custom_source_inputs_match_generated_and_published_contracts() {
    let root = tempfile::tempdir().unwrap();
    let mut input = candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    input["definitions"] = json!({"kind":"router","path":"router.json"});
    input["sources"] = json!([{"source":{"kind":"custom_entry","location":"source.json","schema":{"format":"json","record_schema":{"type":"object"},"fields":{"text":"/text"}}}}]);
    let schema = reviewed("evaluate-candidates-input.schema.json");
    assert!(schema.is_valid(&input));
    decode_candidate_run(&serde_json::to_vec(&input).unwrap()).unwrap();
    input["sources"][0]["source"]["schema"]["format"] = json!("unknown");
    assert!(!schema.is_valid(&input));
    assert!(decode_candidate_run(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn narrowing_question_reference_is_published_and_closed() {
    let schema:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas/evaluate-candidates-outcome.schema.json")).unwrap()).unwrap();
    let reference = json!({"$defs":schema["$defs"],"$ref":"#/$defs/QuestionRef"});
    let validator = jsonschema::options().offline().build(&reference).unwrap();
    assert!(validator.is_valid(&json!({"kind":"narrowing"})));
    assert!(!validator.is_valid(&json!({"kind":"narrowing","extra":true})));
    serde_json::from_value::<QuestionRef>(json!({"kind":"narrowing"})).unwrap();
}

#[path = "support/contract_schemas.rs"]
mod native_schema_support;
#[test]
fn narrowing_rule_definition_matches_the_reviewed_authoring_schema() {
    let root = tempfile::tempdir().unwrap();
    candidate_fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    let mut rule: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.jsonl")).unwrap()).unwrap();
    rule["narrowing"] = json!({"question":{"type":"noul","instructions":"Narrow this candidate"},"evidence":rule["evidence"],"threshold":0.8});
    let schema = native_schema_support::reviewed("rule-file.schema.json");
    assert!(native_schema_support::generated::<RuleDefinition>().is_valid(&rule));
    assert!(schema.is_valid(&rule));
    rule["narrowing"]["extra"] = json!(true);
    assert!(!schema.is_valid(&rule));
}
