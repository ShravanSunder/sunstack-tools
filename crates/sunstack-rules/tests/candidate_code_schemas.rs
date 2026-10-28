#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/candidate_fixture.rs"]
mod fixture;
#[path = "support/contract_schemas.rs"]
mod native_schemas;
use serde_json::{Value, json};
use sunstack_rules::*;
fn candidate_schema(name: &str) -> jsonschema::Validator {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas")
        .join(name);
    let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    jsonschema::options().offline().build(&value).unwrap()
}
#[test]
fn code_and_diff_inputs_and_ast_rule_fields_have_closed_published_shapes() {
    let root = tempfile::tempdir().unwrap();
    let mut input = fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    input["sources"] = json!([{"source":{"kind":"code","repository":"code","revision":{"kind":"worktree"},"paths":[{"kind":"glob","pattern":"src/**"}]}},{"source":{"kind":"diff","repository":"code","scope":{"kind":"revision_diff","base_revision":"base","head":{"kind":"commit","revision":"head"}}}}]);
    let schema = candidate_schema("evaluate-candidates-input.schema.json");
    assert!(schema.is_valid(&input));
    assert!(native_schemas::generated::<CandidateRunInput>().is_valid(&input));
    decode_candidate_run(&serde_json::to_vec(&input).unwrap()).unwrap();
    let mut invalid = input.clone();
    invalid["sources"][0]["source"]["extra"] = json!(true);
    assert!(!schema.is_valid(&invalid));
    assert!(decode_candidate_run(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let mut rule: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.jsonl")).unwrap()).unwrap();
    rule["mechanical_pass"] = json!({"predicate":{"kind":"ast","language":"rust","pattern":"flagged($VALUE)"},"context_parts":["change","touched_unit","related"],"source_sides":"both_available"});
    let published = native_schemas::reviewed("rule-file.schema.json");
    let generated = native_schemas::generated::<RuleDefinition>();
    assert!(published.is_valid(&rule));
    assert!(generated.is_valid(&rule));
    rule["mechanical_pass"]["predicate"]["language"] = json!("unsupported-language");
    assert!(!published.is_valid(&rule));
    assert!(!generated.is_valid(&rule));
    rule["mechanical_pass"] =
        json!({"predicate":{"kind":"regex","field":"/body/blocks/0/block/text","pattern":"MATCH"}});
    assert!(published.is_valid(&rule));
    assert!(generated.is_valid(&rule));
}
