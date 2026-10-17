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
    for paths in [json!([]), json!(["legacy.jsonl"])] {
        let mut header = source_header();
        header["rule_paths"] = paths;
        let result = decode_rule_set(&serde_json::to_vec(&header).unwrap(), "set.json".as_ref());
        assert!(result.is_ok(), "{result:?}");
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
        assert!(decode_rule_set(&serde_json::to_vec(&header).unwrap(), "set.json".as_ref()).is_err());
    }
    let mut empty = source_header();
    empty["rule_sources"] = json!([]);
    assert!(decode_rule_set(&serde_json::to_vec(&empty).unwrap(), "set.json".as_ref()).is_err());
}
