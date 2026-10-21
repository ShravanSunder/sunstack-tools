#![allow(dead_code, clippy::unwrap_used)]
use serde_json::{Value, json};
pub fn source_fixture() -> (tempfile::TempDir, Value) {
    let directory = tempfile::tempdir().unwrap();
    let input = write_source_fixture(directory.path());
    (directory, input)
}

pub fn write_source_fixture(root: &std::path::Path) -> Value {
    std::fs::write(root.join("set.json"), serde_json::to_vec(&json!({
        "schema_version":1,"name":"standards","description":"source check","inputs":{"skill":{"kind":"document"}},
        "applicability":[],"common_ruleset_instructions":"Check the standard.","rule_sources":[{"markdown":"teaching.md","rules":"teaching.rules.json"}],"marker_prefix":"K"
    })).unwrap()).unwrap();
    std::fs::write(
        root.join("teaching.md"),
        "## Standard<!-- K014 -->\n\nKeep *visible* words and `code`.\n",
    )
    .unwrap();
    std::fs::write(root.join("teaching.rules.json"), serde_json::to_vec(&json!({
        "K014":{"name":"standard.visible","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8}
    })).unwrap()).unwrap();
    let input = json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"skill":{"kind":"document","repository":"repo","path":"missing-reviewed.md"}}}]}]},
        "repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],
        "limits":{"max_files":10,"max_file_bytes":65536,"max_total_bytes":131072,"max_matches":20,"max_transcript_bytes":65536,"include_ignored":false},"output":"assembled_rules"});
    input
}

pub fn runtime_input(mut input: Value, endpoint: &str) -> Value {
    input.as_object_mut().unwrap().remove("output");
    input["backend"] = json!({"kind":"jev_openrouter","endpoint":endpoint,"model":"jev-latest"});
    input["origin"] = json!({"kind":"direct"});
    input["cursors"] = json!([]);
    input["execution"] = json!({"speculation":"off","max_concurrent_requests":2});
    input["attempts"] =
        serde_json::to_value(sunstack_decisions::AttemptSettings::default()).unwrap();
    input["request_budget"] = json!({"max_input_tokens":100000,"max_request_bytes":100000,"max_questions":20,"max_requests":20,"safety_margin_tokens":32,"accounting_policy":"allow_estimated"});
    input
}
