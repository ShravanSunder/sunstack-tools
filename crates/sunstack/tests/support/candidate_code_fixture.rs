#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../../sunstack-rules/tests/support/change_fixture.rs"]
mod change_fixture;
use serde_json::{Value, json};
pub struct CodeFixture {
    pub repository: change_fixture::ChangeFixture,
    pub input: Value,
    pub before: String,
    pub after: String,
    pub before_text: String,
    pub after_text: String,
}
impl CodeFixture {
    pub fn new(endpoint: &str, deleted: bool) -> Self {
        let repository = change_fixture::ChangeFixture::new();
        let before_text = format!(
            "mod helpers; mod caller;\nuse crate::helpers::callee;\nconst TOP: &str = \"TOP_SENTINEL\";\npub fn touched() {{\n    unflagged(\"old\");\n    flagged(\"UNCHANGED_MATCH\");\n{}    let credential = \"sk-proj-publicfixturecredential123456789\";\n    callee(); missing_name(); ambiguous();\n}}\nfn ambiguous() {{}}\nfn ambiguous() {{}}\n",
            if deleted {
                "    flagged(\"DELETED_MATCH\");\n"
            } else {
                ""
            }
        );
        let after_text = before_text
            .replace("unflagged(\"old\")", "flagged(\"CHANGED_MATCH\")")
            .replace("    flagged(\"DELETED_MATCH\");\n", "");
        repository.write("src/lib.rs", &before_text);
        repository.write(
            "src/helpers.rs",
            "pub fn callee() -> &'static str { \"COMMITTED_CALLEE\" }\n",
        );
        repository.write(
            "src/caller.rs",
            "pub fn caller() { crate::touched(); let note = \"COMMITTED_CALLER\"; }\n",
        );
        for path in ["src/lib.rs", "src/helpers.rs", "src/caller.rs"] {
            repository.stage(path);
        }
        let before = repository.commit();
        repository.write("src/lib.rs", &after_text);
        repository.stage("src/lib.rs");
        let after = repository.commit();
        repository.write(
            "src/caller.rs",
            "pub fn caller() { crate::touched(); let note = \"DIRTY_CALLER_MUST_NOT_APPEAR\"; }\n",
        );
        let root = repository.root.path();
        let mut input = candidate_fixture::fixture(root, endpoint);
        input["recipes"] = json!([]);
        input["sources"] = json!([{"source":{"kind":"diff","repository":"code","scope":{"kind":"revision_diff","base_revision":before,"head":{"kind":"commit","revision":after}}}}]);
        input["repositories"] = json!([{"key":"code","root":root,"input_paths":[{"kind":"glob","pattern":"src/**"}],"change_scope":{"kind":"full"}}]);
        input["discovery"] = serde_json::to_value(change_fixture::discovery()).unwrap();
        input["limits"]["max_files"] = json!(100);
        input["limits"]["max_total_bytes"] = json!(1000000);
        let path = root.join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["inputs"]["subject"] = json!({"kind":"change_set"});
        std::fs::write(path, set.to_string()).unwrap();
        let path = root.join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["mechanical_pass"] =
            json!({"predicate":{"kind":"ast","language":"rust","pattern":"flagged($VALUE)"}});
        rule["evidence"][0]["select"] = json!({"kind":"changed_units","languages":["rust"],"unit_kinds":["function"],"source_sides":"both_available"});
        std::fs::write(path, rule.to_string()).unwrap();
        Self {
            repository,
            input,
            before,
            after,
            before_text,
            after_text,
        }
    }
    pub fn edit_rule(&self, edit: impl FnOnce(&mut Value)) {
        let path = self.repository.root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        edit(&mut rule);
        std::fs::write(path, rule.to_string()).unwrap();
    }
    pub fn code_input(&self) -> Value {
        let mut input = self.input.clone();
        input["sources"] = json!([{"source":{"kind":"code","repository":"code","revision":{"kind":"commit","revision":self.after},"paths":[{"kind":"exact","path":"src/lib.rs"}]}}]);
        input
    }
}

pub fn assert_span(
    span: &Value,
    text: &str,
    snippet: &str,
    path: &str,
    repository: &str,
    revision: &str,
    side: &str,
) {
    let start = text.find(snippet).unwrap();
    let end = start + snippet.len();
    let position = |offset: usize| {
        let prefix = &text[..offset];
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap().chars().count() + 1,
        )
    };
    let (start_line, start_column) = position(start);
    let (end_line, end_column) = position(end);
    for (field, value) in [
        ("byte_start", start),
        ("byte_end", end),
        ("start_line", start_line),
        ("start_column", start_column),
        ("end_line", end_line),
        ("end_column", end_column),
    ] {
        assert_eq!(span[field], value, "{field} for {snippet}");
    }
    assert_eq!(span["source"]["kind"], "repository_file");
    assert_eq!(span["source"]["path"], path);
    assert_eq!(span["source"]["repository"], repository);
    assert_eq!(
        span["source"]["revision"],
        json!({"kind":"commit","commit_id":revision,"side":side})
    );
}
