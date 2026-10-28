#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
use serde_json::{Value, json};
use std::path::Path;
pub fn fixture(root: &Path, endpoint: &str, document: bool) -> Value {
    let mut input = candidate_fixture::fixture(root, endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    std::fs::write(
        root.join("first.jsonl"),
        format!(
            "{}\n",
            json!({"type":"assistant","message":{"content":"MATCH public occurrence"}})
        ),
    )
    .unwrap();
    let path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["fact"] = if document {
        json!({"kind":"document"})
    } else {
        json!({"kind":"payload","schema":"urn:owner-fact"})
    };
    std::fs::write(path, set.to_string()).unwrap();
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"outside","input":"fact","select":if document {json!({"kind":"whole_document"})} else {json!({"kind":"whole_payload"})}}));
    std::fs::write(path, rule.to_string()).unwrap();
    if document {
        input["repositories"] = json!([{"key":"facts","root":root,"input_paths":[{"kind":"exact","path":"owner.md"}],"change_scope":{"kind":"full"}}]);
    } else {
        std::fs::write(root.join("fact-schema.json"), json!({"type":"object","properties":{"decision":{"type":"string"}},"required":["decision"],"additionalProperties":false}).to_string()).unwrap();
        input["payload_schemas"] = json!([{"name":"urn:owner-fact","path":"fact-schema.json"}]);
    }
    input
}
pub fn report(output: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    wire["result"]["outcome"].clone()
}
