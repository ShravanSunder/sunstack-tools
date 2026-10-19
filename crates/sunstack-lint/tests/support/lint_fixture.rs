#![allow(dead_code, clippy::unwrap_used)]
use serde_json::{Value, json};
use std::path::Path;
use sunstack_lint::{LintRequest, ScopeChoice};
pub fn write(root: &Path) -> Value {
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
        root.join("docs/one.md"),
        "---\ndescription: Wrong opening\n---\n\n# One\n",
    )
    .unwrap();
    std::fs::write(root.join("set.json"),serde_json::to_vec_pretty(&json!({"schema_version":1,"name":"document standards","description":"fixture","inputs":{"subject":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":"Evaluate the document.","rule_paths":["rules.jsonl"]})).unwrap()).unwrap();
    std::fs::write(root.join("rules.jsonl"),serde_json::to_vec(&json!({"rule_id":"description","evidence":[{"id":"target","input":"subject","select":{"kind":"frontmatter_field","field":"description"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"mechanical","explanation":"Description must begin with Use when.","check":{"kind":"expect","expect":{"starts_with":"Use when"}}},"severity":"error"})).unwrap()).unwrap();
    let config = json!({"$schema":"https://example.invalid/sunstack.lint.schema.json","rule_sets":{"docs":{"path":"set.json","files":["docs/*.md"]}}});
    config_file(root, &config);
    config
}
pub fn config_file(root: &Path, config: &Value) {
    std::fs::write(
        root.join("sunstack.lint.json"),
        serde_json::to_vec_pretty(config).unwrap(),
    )
    .unwrap();
}
pub fn request(root: &Path) -> LintRequest {
    LintRequest {
        config_path: Some(root.join("sunstack.lint.json")),
        paths: vec![],
        scope: ScopeChoice::All,
        max_warnings: None,
    }
}
pub fn commit(root: &Path, message: &str) {
    let repository =
        git2::Repository::open(root).unwrap_or_else(|_| git2::Repository::init(root).unwrap());
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents = parent.iter().collect::<Vec<_>>();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap();
}
