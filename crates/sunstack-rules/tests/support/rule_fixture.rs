#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::path::Path;
use sunstack_rules::*;
pub fn declaration(input: &str, id: &str) -> Value {
    json!({"id":id,"input":input,"select":{"kind":"paragraph","target_id":"U001"}})
}
pub fn rule(id: &str) -> Value {
    json!({"rule_id":id,"evidence":[declaration("subject","target")],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":format!("judge {id}")},"threshold":0.8}})
}
pub fn decision(input: &str) -> Value {
    json!({"kind":"decision","question":{"type":"noul","instructions":format!("gate {input}")},"evidence":[declaration(input,"context")],"threshold":0.6})
}
pub fn write_set(root: &Path, rules: &[Value], applicability: Vec<Value>) {
    std::fs::write(root.join("source.md"), "U001: original content\n").unwrap();
    std::fs::write(root.join("other.md"), "U001: gate content\n").unwrap();
    std::fs::write(root.join("set.json"),serde_json::to_vec(&json!({"schema_version":1,"name":"proof","description":"controlled boundary","inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},"applicability":applicability,"common_ruleset_instructions":"RULE_COMMON","rule_paths":["rules.jsonl"]})).unwrap()).unwrap();
    std::fs::write(
        root.join("rules.jsonl"),
        rules
            .iter()
            .map(|rule| serde_json::to_string(rule).unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
}
pub fn input(root: &Path, endpoint: &str) -> RuleInvocationInput {
    serde_json::from_value(json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"},"other":{"kind":"document","repository":"repo","path":"other.md"}}}]}]},"repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"exact","path":"source.md"},{"kind":"exact","path":"other.md"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"origin":{"kind":"direct"},"cursors":[],"execution":{"speculation":"off","max_concurrent_requests":4},"limits":{"max_files":8,"max_file_bytes":65536,"max_total_bytes":131072,"max_matches":20,"max_transcript_bytes":65536,"include_ignored":false},"backend":{"kind":"jev_openrouter","endpoint":endpoint,"model":"jev-latest"},"attempts":sunstack_decisions::AttemptSettings::default(),"request_budget":{"max_input_tokens":100000,"max_request_bytes":100000,"max_questions":20,"max_requests":20,"safety_margin_tokens":32,"accounting_policy":"allow_estimated"}})).unwrap()
}
