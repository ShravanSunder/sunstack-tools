#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_rules::*;

fn source_header() -> serde_json::Value {
    json!({
        "schema_version":1,
        "name":"marked standards",
        "description":"source assembly contract",
        "inputs":{"skill":{"kind":"document"}},
        "applicability":[],
        "common_ruleset_instructions":"Apply each marked standard.",
        "rule_paths":[],
        "rule_sources":[{"markdown":"teaching.md","rules":"teaching.rules.json"}],
        "marker_prefix":"K"
    })
}

#[test]
fn rule_set_header_accepts_markdown_sources_alone_or_beside_jsonl() {
    for paths in [None, Some(json!([])), Some(json!(["legacy.jsonl"]))] {
        let mut header = source_header();
        if let Some(paths) = paths {
            header["rule_paths"] = paths;
        } else {
            header.as_object_mut().unwrap().remove("rule_paths");
        }
        let result = decode_rule_set(&serde_json::to_vec(&header).unwrap(), "set.json".as_ref());
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(result.unwrap().max_question_words, 400);
    }
}

#[test]
fn generated_rule_source_header_schema_matches_native_source_and_prefix_requirements() {
    let schema = serde_json::to_value(schemars::schema_for!(RuleSetDefinition)).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    let mut cases = vec![(source_header(), true)];
    let mut no_paths = source_header();
    no_paths.as_object_mut().unwrap().remove("rule_paths");
    cases.push((no_paths, true));
    let mut legacy = source_header();
    legacy["rule_paths"] = json!(["legacy.jsonl"]);
    legacy.as_object_mut().unwrap().remove("rule_sources");
    legacy.as_object_mut().unwrap().remove("marker_prefix");
    cases.push((legacy, true));
    for prefix in [None, Some(json!(null)), Some(json!("k"))] {
        let mut missing = source_header();
        missing["rule_paths"] = json!(["legacy.jsonl"]);
        if let Some(prefix) = prefix {
            missing["marker_prefix"] = prefix;
        } else {
            missing.as_object_mut().unwrap().remove("marker_prefix");
        }
        cases.push((missing, false));
    }
    let mut empty = source_header();
    empty["rule_sources"] = json!([]);
    cases.push((empty, false));
    let mut bad_path = source_header();
    bad_path["rule_sources"][0]["markdown"] = json!("");
    cases.push((bad_path, false));
    for (header, accepted) in cases {
        assert_eq!(validator.is_valid(&header), accepted, "schema: {header}");
        assert_eq!(
            decode_rule_set(&serde_json::to_vec(&header).unwrap(), "set.json".as_ref()).is_ok(),
            accepted,
            "native: {header}"
        );
    }
}

#[test]
fn rule_source_header_requires_a_valid_prefix_and_at_least_one_source() {
    for prefix in [None, Some(json!("")), Some(json!("k")), Some(json!("KK"))] {
        let mut header = source_header();
        if let Some(prefix) = prefix {
            header["marker_prefix"] = prefix;
        } else {
            header.as_object_mut().unwrap().remove("marker_prefix");
        }
        assert!(
            decode_rule_set(&serde_json::to_vec(&header).unwrap(), "set.json".as_ref()).is_err()
        );
    }
    let mut empty = source_header();
    empty["rule_sources"] = json!([]);
    assert!(decode_rule_set(&serde_json::to_vec(&empty).unwrap(), "set.json".as_ref()).is_err());
}
