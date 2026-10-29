#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
use serde_json::{Value, json};
use std::path::Path;

pub fn record_run(
    root: &Path,
    endpoint: &str,
    records_by_input: &[Vec<String>],
    samples_per_pass: usize,
) -> Value {
    assert!(!records_by_input.is_empty());
    assert!(records_by_input.len() <= 2);

    let mut input = candidate_fixture::fixture(root, endpoint);
    input["sources"]
        .as_array_mut()
        .unwrap()
        .truncate(records_by_input.len());
    input["samples_per_pass"] = json!(samples_per_pass);
    // The process proof holds a response and observes the next scheduler admission.
    input["execution"]["max_concurrent_requests"] = json!(2);
    input["request_budget"]["max_requests"] = json!(512);

    let rules_path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    let evidence = rule["evidence"].clone();
    rule["narrowing"] = json!({
        "question":{"type":"noul","instructions":"NARROW_SAMPLE"},
        "evidence":evidence,
        "threshold":0.8,
        "uncertain_below":0.7
    });
    std::fs::write(rules_path, rule.to_string()).unwrap();

    for (source_index, records) in records_by_input.iter().enumerate() {
        let source_name = if source_index == 0 { "first" } else { "second" };
        let lines = records
            .iter()
            .enumerate()
            .map(|(record_index, content)| {
                json!({
                    "type":"assistant",
                    "message":{
                        "id":format!("narrowing-{source_index}-{record_index}"),
                        "content":content
                    }
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(
            root.join(format!("{source_name}.jsonl")),
            format!("{lines}\n"),
        )
        .unwrap();
    }

    input
}
