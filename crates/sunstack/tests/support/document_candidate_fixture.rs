#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "../../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../../sunstack-rules/tests/support/change_fixture.rs"]
mod change_fixture;
use serde_json::{Value, json};
pub fn candidate_response(output: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn fixture(
    endpoint: &str,
    selection: Value,
    pass: Option<Value>,
) -> (change_fixture::ChangeFixture, Value, String, String) {
    let repository = change_fixture::ChangeFixture::new();
    let text = "---\r\ndescription: Use when checking.\r\n---\r\n# First\r\n\r\nIf café is ready, load alpha.\r\n\r\nOrdinary words remain.\r\n\r\n# Second\r\n\r\nWhen ready, load beta.\r\n".to_owned();
    repository.write("guide.md", &text);
    repository.stage("guide.md");
    let revision = repository.commit();
    let root = repository.root.path();
    let mut input = candidate_fixture::fixture(root, endpoint);
    input["sources"] = json!([{"source":{"kind":"code","repository":"docs","revision":{"kind":"commit","revision":revision},"paths":[{"kind":"exact","path":"guide.md"}]}}]);
    input["recipes"] = json!([]);
    input["repositories"] = json!([{"key":"docs","root":root,"input_paths":[{"kind":"exact","path":"guide.md"}],"change_scope":{"kind":"full"}}]);
    input["discovery"] = serde_json::to_value(change_fixture::discovery()).unwrap();
    let mut set: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    set["inputs"]["subject"] = json!({"kind":"document"});
    std::fs::write(root.join("set.json"), set.to_string()).unwrap();
    let mut rule: Value =
        serde_json::from_slice(&std::fs::read(root.join("rules.jsonl")).unwrap()).unwrap();
    rule["evidence"][0]["select"] = selection;
    rule.as_object_mut().unwrap().remove("mechanical_pass");
    if let Some(pass) = pass {
        rule["mechanical_pass"] = pass;
    }
    std::fs::write(root.join("rules.jsonl"), rule.to_string()).unwrap();
    (repository, input, text, revision)
}
pub fn replace_document(repository: &change_fixture::ChangeFixture, input: &mut Value, text: &str) {
    repository.write("guide.md", text);
    repository.stage("guide.md");
    let revision = repository.commit();
    input["sources"][0]["source"]["revision"]["revision"] = json!(revision);
}

pub fn assert_document_span(span: &Value, text: &str, start: usize, end: usize, revision: &str) {
    let position = |offset| {
        let prefix = &text[..offset];
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap().chars().count() + 1,
        )
    };
    let (start_line, start_column) = position(start);
    let (end_line, end_column) = position(end);
    for (field, expected) in [
        ("byte_start", start),
        ("byte_end", end),
        ("start_line", start_line),
        ("start_column", start_column),
        ("end_line", end_line),
        ("end_column", end_column),
    ] {
        assert_eq!(span[field], expected, "{field}");
    }
    assert_eq!(span["source"]["kind"], "repository_file");
    assert_eq!(span["source"]["path"], "guide.md");
    assert_eq!(span["source"]["repository"], "docs");
    assert_eq!(
        span["source"]["revision"],
        json!({"kind":"commit","commit_id":revision,"side":"after"})
    );
}

pub fn assert_document_excerpt(span: &Value, text: &str, snippet: &str, revision: &str) {
    let start = text.find(snippet).unwrap();
    assert_document_span(span, text, start, start + snippet.len(), revision);
}

pub fn request_markdown_units(body: &Value) -> Vec<&Value> {
    body["state"]["contents"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|unit| unit.get("visible_text").is_some())
        .collect()
}
pub fn request_markdown_attributions(body: &Value) -> Vec<&Value> {
    body["state"]["evidence"]
        .as_object()
        .unwrap()
        .values()
        .filter(|evidence| evidence["kind"] == "markdown_units")
        .flat_map(|evidence| evidence["units"].as_array().unwrap())
        .collect()
}
