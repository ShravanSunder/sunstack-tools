#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "change_fixture.rs"]
pub mod changes;
#[path = "rule_fixture.rs"]
pub mod fixture;
use serde_json::{Value, json};
use sunstack_evidence::*;
use sunstack_rules::*;
pub struct ScopeFixture {
    pub repository: changes::ChangeFixture,
}
impl ScopeFixture {
    pub fn new() -> Self {
        let repository = changes::ChangeFixture::new();
        repository.write("source.md", "U001: original\n\nU002: neighbor\n");
        let set = json!({"schema_version":1,"name":"scope","description":"independent literals","inputs":{"subject":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["changed.jsonl","neighbor.jsonl"]});
        repository.write("set.json", &set.to_string());
        repository.write("changed.jsonl", &fixture::rule("changed").to_string());
        let mut neighbor = fixture::rule("neighbor");
        neighbor["evidence"][0]["select"]["target_id"] = json!("U002");
        repository.write("neighbor.jsonl", &neighbor.to_string());
        for path in ["source.md", "set.json", "changed.jsonl", "neighbor.jsonl"] {
            repository.stage(path);
        }
        repository.commit();
        Self { repository }
    }
    pub fn input(&self, endpoint: &str) -> RuleInvocationInput {
        let root = self.repository.root.path();
        let mut input = serde_json::to_value(fixture::input(root, endpoint)).unwrap();
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] =
            json!({"subject":{"kind":"document","repository":"repo","path":"source.md"}});
        input["repositories"] = json!([self.repository.binding(
            "repo",
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged]
            }
        )]);
        input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        input["limits"]["max_files"] = json!(100);
        input["limits"]["max_total_bytes"] = json!(1000000);
        serde_json::from_value(input).unwrap()
    }
}

pub const GATE_TEXT: &str = "U009: stable gate input\n";
impl ScopeFixture {
    pub fn add_gate(&self, endpoint: &str) -> RuleInvocationInput {
        let path = self.repository.root.path().join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        set["inputs"]["gate"] = json!({"kind":"document"});
        let mut gate = fixture::decision("gate");
        gate["evidence"][0]["select"]["target_id"] = json!("U009");
        set["applicability"] = json!([gate]);
        self.repository.write("set.json", &set.to_string());
        self.repository.write("gate.md", GATE_TEXT);
        self.repository.stage("set.json");
        self.repository.stage("gate.md");
        self.repository.commit();
        let mut input = self.input(endpoint);
        let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
            panic!("fixture owner")
        };
        rule_sets[0].binding_groups[0].bindings.insert(
            SourceName::new("gate").unwrap(),
            InputBinding::Document {
                repository: RepositoryId::new("repo").unwrap(),
                path: "gate.md".into(),
            },
        );
        input
    }
}
