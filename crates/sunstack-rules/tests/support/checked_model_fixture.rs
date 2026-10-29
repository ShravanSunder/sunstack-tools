#![allow(dead_code, clippy::panic, clippy::unwrap_used)]

use serde_json::{Value, json};
use std::path::Path;

pub const MARKED_RULE_ID: &str = "marked.schema.parity";
pub const FULL_RULE_ID: &str = "full.schema.parity";

fn markdown_paragraph_selection() -> Value {
    json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]})
}

fn markdown_evidence(name: &str) -> Value {
    json!({"id":name,"input":"skill","select":markdown_paragraph_selection()})
}

pub fn write_checked_model_files(root: &Path) -> Value {
    let marked_entry = json!({
        "name": MARKED_RULE_ID,
        "target": {
            "input": "skill",
            "select": markdown_paragraph_selection()
        },
        "mechanical_pass": {
            "predicate": {"kind":"literal","field":"/text","text":"A marked"}
        },
        "narrowing": {
            "question": {"type":"noul","instructions":"Is this standard relevant?"},
            "evidence": [markdown_evidence("narrowing")],
            "threshold": 0.8,
            "uncertain_below": 0.6
        },
        "gates": [{
            "kind":"decision",
            "question": {"type":"noul","instructions":"Does the marked standard apply?"},
            "evidence": [markdown_evidence("gate")],
            "threshold": 0.7,
            "uncertain_below": 0.65
        }],
        "questions": {},
        "threshold": 0.85
    });
    let full_rule = json!({
        "rule_id": FULL_RULE_ID,
        "evidence": [markdown_evidence("target")],
        "target": "target",
        "applicability": [{
            "kind":"decision",
            "question": {"type":"noul","instructions":"Does the full model gate apply?"},
            "evidence": [markdown_evidence("full_gate")],
            "threshold": 0.72,
            "uncertain_below": 0.62
        }],
        "questions": {},
        "verdict": {
            "kind":"noul",
            "question": {"type":"noul","instructions":"Full definition verdict question."},
            "threshold": 0.8,
            "uncertain_below": 0.7
        },
        "mechanical_pass": {
            "predicate": {"kind":"literal","field":"/text","text":"A full"}
        },
        "narrowing": {
            "question": {"type":"noul","instructions":"Is the full model relevant?"},
            "evidence": [markdown_evidence("full_narrowing")],
            "threshold": 0.75,
            "uncertain_below": 0.55
        },
        "private_access": ["thinking"]
    });
    let ruleset = json!({
        "schema_version": 1,
        "name": "Checker output schema parity",
        "description": "A marked rule and a full native definition exercise checker output.",
        "inputs": {"skill":{"kind":"document"}},
        "applicability": [],
        "common_ruleset_instructions": null,
        "rule_paths": ["full-rules.jsonl"],
        "rule_sources": [{"markdown":"standards.md","rules":"standards.rules.json"}],
        "marker_prefix": "K"
    });
    let marker_rules = json!({"K014": marked_entry});

    std::fs::write(
        root.join("ruleset.json"),
        serde_json::to_vec(&ruleset).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("full-rules.jsonl"),
        format!("{}\n", serde_json::to_string(&full_rule).unwrap()),
    )
    .unwrap();
    std::fs::write(
        root.join("standards.md"),
        "## Model<!-- K014 -->\n\nA marked standard has a visible criterion.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("standards.rules.json"),
        serde_json::to_vec(&marker_rules).unwrap(),
    )
    .unwrap();

    check_input(root)
}

pub fn check_input(definition_root: &Path) -> Value {
    json!({
        "definition_root": definition_root,
        "definitions": {"kind":"rule_sets","rule_sets":["ruleset.json"]},
        "payload_schemas": [],
        "limits": {"max_file_bytes": 65536},
        "output": "assembled_rules"
    })
}

pub fn assert_populated_output(outcome: &Value, root: &Path) {
    assert_eq!(
        outcome["kind"], "checked",
        "unexpected checker outcome: {outcome}"
    );
    assert_eq!(outcome["checked_rule_count"], 2);
    let printed_rules = outcome["printed_rules"].as_array().unwrap();
    assert_eq!(printed_rules.len(), 2);
    let root = root.canonicalize().unwrap();

    let marked = printed_rules
        .iter()
        .find(|rule| rule["definition"]["rule_id"] == MARKED_RULE_ID)
        .unwrap();
    let full = printed_rules
        .iter()
        .find(|rule| rule["definition"]["rule_id"] == FULL_RULE_ID)
        .unwrap();
    let set_path = root.join("ruleset.json");
    assert_eq!(marked["rule_set"], json!(set_path));
    assert_eq!(full["rule_set"], json!(set_path));
    assert_eq!(
        marked["source"],
        json!({
            "kind":"markdown",
            "provenance": {
                "markdown": root.join("standards.md"),
                "rules": root.join("standards.rules.json"),
                "marker_id":"K014",
                "marker_span": {
                    "byte_start":8,
                    "byte_end":21,
                    "start_line":1,
                    "start_column":9,
                    "end_line":1,
                    "end_column":22
                },
                "entry_location": {
                    "file":root.join("standards.rules.json"),
                    "line":1,
                    "column":9
                }
            }
        })
    );
    assert_eq!(
        full["source"],
        json!({"kind":"jsonl","file":root.join("full-rules.jsonl"),"line":1})
    );

    let paragraph_selection = json!({
        "kind":"markdown_units",
        "mode":"unit",
        "units":["paragraph"]
    });
    let target_evidence = json!({
        "id":"target",
        "input":"skill",
        "requirement":"required",
        "select":paragraph_selection
    });
    let marked_evidence = json!({
        "id":"narrowing",
        "input":"skill",
        "requirement":"required",
        "select":paragraph_selection
    });
    let gate_evidence = json!({
        "id":"gate",
        "input":"skill",
        "requirement":"required",
        "select":paragraph_selection
    });
    let marked_question = json!({
        "type":"noul",
        "instructions":"Does the marked standard apply?"
    });
    let narrowing_question = json!({
        "type":"noul",
        "instructions":"Is this standard relevant?"
    });
    let marked_definition = json!({
        "rule_id": MARKED_RULE_ID,
        "evidence": [target_evidence],
        "target":"target",
        "applicability": [{
            "kind":"decision",
            "question":marked_question,
            "evidence":[gate_evidence],
            "threshold":0.7,
            "uncertain_below":0.65
        }],
        "questions":{},
        "verdict": {
            "kind":"noul",
            "pass_means":"conforms",
            "question": {
                "type":"noul",
                "instructions":"Model\n\nA marked standard has a visible criterion."
            },
            "threshold":0.85
        },
        "severity":"warn",
        "mechanical_pass": {
            "predicate":{"kind":"literal","field":"/text","text":"A marked"}
        },
        "narrowing": {
            "question":narrowing_question,
            "evidence":[marked_evidence],
            "threshold":0.8,
            "uncertain_below":0.6
        }
    });
    let full_definition = json!({
        "rule_id": FULL_RULE_ID,
        "evidence": [target_evidence],
        "target":"target",
        "applicability":[{
            "kind":"decision",
            "question": {
                "type":"noul",
                "instructions":"Does the full model gate apply?"
            },
            "evidence":[{
                "id":"full_gate",
                "input":"skill",
                "requirement":"required",
                "select":paragraph_selection
            }],
            "threshold":0.72,
            "uncertain_below":0.62
        }],
        "questions":{},
        "verdict": {
            "kind":"noul",
            "question": {
                "type":"noul",
                "instructions":"Full definition verdict question."
            },
            "threshold":0.8,
            "uncertain_below":0.7
        },
        "severity":"warn",
        "mechanical_pass": {
            "predicate":{"kind":"literal","field":"/text","text":"A full"}
        },
        "narrowing": {
            "question": {
                "type":"noul",
                "instructions":"Is the full model relevant?"
            },
            "evidence":[{
                "id":"full_narrowing",
                "input":"skill",
                "requirement":"required",
                "select":paragraph_selection
            }],
            "threshold":0.75,
            "uncertain_below":0.55
        },
        "private_access":["thinking"]
    });
    assert_eq!(marked["definition"], marked_definition);
    assert_eq!(full["definition"], full_definition);
}

pub fn unknown_field_variants(outcome: &Value) -> Vec<(&'static str, Value)> {
    let printed_rules = outcome["printed_rules"].as_array().unwrap();
    let marked_index = printed_rules
        .iter()
        .position(|rule| rule["definition"]["rule_id"] == MARKED_RULE_ID)
        .unwrap();
    let full_index = printed_rules
        .iter()
        .position(|rule| rule["definition"]["rule_id"] == FULL_RULE_ID)
        .unwrap();

    let mut variants = Vec::new();
    let mut unknown = outcome.clone();
    unknown["unexpected"] = json!(true);
    variants.push(("outcome", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][marked_index]["definition"]["unexpected"] = json!(true);
    variants.push(("printed definition", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][full_index]["definition"]["mechanical_pass"]["unexpected"] =
        json!(true);
    variants.push(("full mechanical pass", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][full_index]["definition"]["narrowing"]["unexpected"] = json!(true);
    variants.push(("full narrowing", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][full_index]["definition"]["applicability"][0]["unexpected"] =
        json!(true);
    variants.push(("full gate", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][full_index]["definition"]["verdict"]["unexpected"] = json!(true);
    variants.push(("full verdict", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][marked_index]["definition"]["mechanical_pass"]["unexpected"] =
        json!(true);
    variants.push(("marked mechanical pass", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][marked_index]["definition"]["narrowing"]["unexpected"] = json!(true);
    variants.push(("marked narrowing", unknown));

    let mut unknown = outcome.clone();
    unknown["printed_rules"][marked_index]["definition"]["applicability"][0]["unexpected"] =
        json!(true);
    variants.push(("marked gate", unknown));

    variants
}

pub fn invalid_required_value_variants(outcome: &Value) -> Vec<(&'static str, Value)> {
    let printed_rules = outcome["printed_rules"].as_array().unwrap();
    let full_index = printed_rules
        .iter()
        .position(|rule| rule["definition"]["rule_id"] == FULL_RULE_ID)
        .unwrap();

    let mut variants = Vec::new();
    let mut invalid = outcome.clone();
    invalid["printed_rules"][full_index]["definition"]
        .as_object_mut()
        .unwrap()
        .remove("rule_id");
    variants.push(("missing printed rule_id", invalid));

    let mut invalid = outcome.clone();
    invalid["printed_rules"][full_index]["definition"]["verdict"]["threshold"] = json!("0.8");
    variants.push(("string verdict threshold", invalid));

    let mut invalid = outcome.clone();
    invalid["printed_rules"][full_index]["definition"]["private_access"][0] =
        json!("not_a_private_kind");
    variants.push(("invalid private_access kind", invalid));

    variants
}
