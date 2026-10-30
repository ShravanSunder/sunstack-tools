#![allow(dead_code, clippy::unwrap_used)]
use serde_json::{Value, json};
use std::path::Path;
use sunstack_lint::{LintRequest, ScopeChoice};
pub fn write(root: &Path) -> Value {
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("evidence/rules")).unwrap();
    std::fs::write(
        root.join("docs/one.md"),
        "---\ndescription: Wrong opening\n---\n\n# One\n",
    )
    .unwrap();
    std::fs::write(root.join("evidence/sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        "id: skill\nlanguage: Markdown\nrule: {matches: frontmatter}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("rulebook.md"),
        "Description must begin with Use when. <!-- K001 -->\n",
    )
    .unwrap();
    entry_file(
        root,
        &json!({"K001":{"name":"description","evidence":"skill","severity":"error","decision":{"start":"starts","steps":{"starts":{"mechanical_check":{"tool":"regex","field":"description","pattern":"^Use when"},"yes":"pass","no":{"finding":"Description must begin with Use when.","at":"description"}}}}}}),
    );
    let config = json!({"$schema":"https://example.invalid/sunstack.lint.schema.json","rulebook":[{"markdown":"rulebook.md","rules":"rules.json"}],"marker_prefix":"K","check":{"files":["docs/*.md"]},"evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{"skill":{"rule":"skill","fields":{"description":{"from":"yaml","key":"description"}}}}},"decision_model":{"model":"jev-latest"}});
    config_file(root, &config);
    config
}
pub fn entry_file(root: &Path, entries: &Value) {
    std::fs::write(
        root.join("rules.json"),
        serde_json::to_vec_pretty(entries).unwrap(),
    )
    .unwrap();
}
pub fn judged(root: &Path, endpoint: &str) -> Value {
    let mut config = write(root);
    config["decision_model"]["endpoint"] = json!(endpoint);
    config_file(root, &config);
    entry_file(
        root,
        &json!({"K001":{"name":"description","evidence":"skill","severity":"error","questions":{"complies":{"type":"noul","instructions":{"check":"description","task":"Does it comply?"}},"details":{"type":"noul","instructions":{"check":"description","task":"Is the evidence useful?"}}},"decision":{"start":"complies","steps":{"complies":{"ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.3,"yes":"pass","no":{"finding":"Description must begin with Use when.","at":"description"}}}}}}),
    );
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
        evidence_listing: None,
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
