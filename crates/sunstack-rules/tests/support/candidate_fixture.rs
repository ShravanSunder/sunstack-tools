#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::path::Path;
pub fn fixture(root: &Path, endpoint: &str) -> Value {
    std::fs::write(root.join("set.json"), serde_json::to_vec(&json!({
        "schema_version":1,"name":"candidates","description":"positioned session proof",
        "inputs":{"subject":{"kind":"conversation"}},"applicability":[],
        "common_ruleset_instructions":"judge the supplied occurrence", "rule_paths":["rules.jsonl"]
    })).unwrap()).unwrap();
    std::fs::write(root.join("rules.jsonl"), serde_json::to_vec(&json!({
        "rule_id":"candidate.one","evidence":[{"id":"target","input":"subject","select":{"kind":"conversation",
            "turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"included"},"hook_injected":"included"
        }}],"target":"target","applicability":[],"questions":{},
        "mechanical_pass":{"predicate":{"kind":"regex","field":"/body/blocks/0/block/text","pattern":"MATCH"}},
        "verdict":{"kind":"noul","question":{"type":"noul","instructions":"Is this occurrence a finding?"},"threshold":0.8}
    })).unwrap()).unwrap();
    json!({"sources":[{"source":{"kind":"session_pair","provider":"claude_code","session":"first"}},{"source":{"kind":"session_pair","provider":"claude_code","session":"second"}}],
        "definitions":{"kind":"rule_sets","rule_sets":["set.json"]},"definition_root":root,
        "recipes":[{"provider":"claude_code","locate":{"roots":[root],"pattern":"{session}.jsonl"},"read":{"format":"claude_code_jsonl"},"map":{"fields":{}}}],
        "repositories":[],"payload_schemas":[],"execution":{"speculation":"off","max_concurrent_requests":4},
        "limits":{"max_files":10,"max_file_bytes":65536,"max_total_bytes":262144,"max_matches":100,"max_transcript_bytes":131072,"include_ignored":false},
        "backend":{"kind":"jev_openrouter","endpoint":endpoint,"model":"jev-latest"},"attempts":sunstack_decisions::AttemptSettings::default(),
        "request_budget":{"max_input_tokens":100000,"max_request_bytes":100000,"max_questions":20,"max_requests":20,"safety_margin_tokens":32,"accounting_policy":"allow_estimated"}})
}
