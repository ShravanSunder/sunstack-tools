#![allow(dead_code, clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use std::path::Path;
use sunstack_rules::RuleInvocationInput;
use sunstack_rules::test_support::rule_fixture as fixture;

pub fn input(root: &Path, endpoint: &str) -> RuleInvocationInput {
    for name in ["gated", "peer"] {
        std::fs::create_dir(root.join(name)).unwrap();
    }
    let mut many = fixture::rule("many");
    many["evidence"][0]["select"] =
        json!({"kind":"markdown_units","mode":"call_site","units":["list_item"],"pattern":"Use"});
    fixture::write_set(
        &root.join("gated"),
        &[many],
        vec![fixture::decision("other")],
    );
    fixture::write_set(&root.join("peer"), &[fixture::rule("peer")], vec![]);
    std::fs::write(
        root.join("gated/source.md"),
        "- Use alpha; Use beta; Use gamma. INITIAL\n",
    )
    .unwrap();
    let mut wire = serde_json::to_value(fixture::input(root, endpoint)).unwrap();
    let sets = ["gated", "peer"].into_iter().map(|name| json!({"path":format!("{name}/set.json"),"binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":format!("{name}/source.md")},"other":{"kind":"document","repository":"repo","path":format!("{name}/other.md")}}}]})).collect::<Vec<_>>();
    wire["definitions"]["rule_sets"] = json!(sets);
    wire["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"**"}]);
    wire["attempts"]["retry_count"] = json!(0);
    wire["execution"]["max_concurrent_requests"] = json!(1);
    serde_json::from_value(wire).unwrap()
}
