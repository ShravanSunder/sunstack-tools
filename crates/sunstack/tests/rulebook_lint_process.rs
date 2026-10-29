#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled;
mod support;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rulebook-lint"),
        root.path(),
    );
    root
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn save(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn command(root: &Path) -> Command {
    let mut command = support::isolated_command(root);
    command
        .arg("lint")
        .current_dir(root)
        .arg("--config")
        .arg("skill-rules.config.json")
        .env_remove("OPENROUTER_API_KEY")
        .stdin(Stdio::null())
        .kill_on_drop(true);
    command
}
async fn invoke(root: &Path, args: &[&str], expected: i32) -> Value {
    let output = tokio::time::timeout(Duration::from_secs(10), command(root).args(args).output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn model_response(body: &Value, combine: f64) -> Value {
    let mut answers = serde_json::Map::new();
    for name in body["questions"].as_object().unwrap().keys() {
        answers.insert(name.clone(),match name.as_str() {
            "kind"=>json!({"type":"choice","choice":"outcome","confidence":0.86,"probabilities":{"outcome":0.86,"action":0.10,"other":0.04}}),
            "covers"=>json!({"type":"noul","noul":(combine-0.4)/0.6}),
            "checkable"=>json!({"type":"score","score":2.0,"confidence":1.0,"legend":{"0":"Not checkable.","1":"Partly checkable.","2":"Directly checkable."},"probabilities":{"0":0.0,"1":0.0,"2":1.0}}),
            _=>json!({"type":"noul","noul":0.91}),
        });
    }
    json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
}
async fn judged(root: &Path, combine: f64, exit: i32) -> (Value, Vec<Vec<u8>>) {
    let mut server = controlled::ControlledServer::start().await;
    let config_path = root.join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&config_path, &config);
    let child = command(root)
        .args(["--all", "--format", "json"])
        .env("OPENROUTER_API_KEY", "SYNTHETIC_RULEBOOK_KEY")
        .output();
    let mut operation = tokio::spawn(child);
    let mut bodies = Vec::new();
    let output=tokio::time::timeout(Duration::from_secs(10),async {
        loop {tokio::select! {
            output=&mut operation=>break output.unwrap().unwrap(),
            request=server.receive()=>{
                let response=model_response(&request.body,combine);bodies.push(request.raw_body.clone());request.respond(200,response);
            },
        }}
    }).await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SYNTHETIC_RULEBOOK_KEY"));
    assert_eq!(server.finish().await, bodies.len());
    (serde_json::from_slice(&output.stdout).unwrap(), bodies)
}

#[tokio::test]
async fn binary_lists_native_evidence_and_drives_worked_examples_and_uncertain_precedence() {
    let root = fixture();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["skills/verification-design/SKILL.md"]);
    save(&config_path, &config);
    let checked = invoke(root.path(), &["--check"], 0).await;
    assert_eq!(checked.as_array().unwrap().len(), 3);
    let evidence = invoke(root.path(), &["--all", "--evidence", "step"], 0).await;
    assert_eq!(evidence["pieces"].as_array().unwrap().len(), 6);
    let piece = evidence["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|piece| piece["where"]["first_line"] == 49)
        .unwrap();
    assert_eq!(piece["where"]["span"]["byte_start"], 6114);
    assert_eq!(piece["where"]["span"]["byte_end"], 7909);
    let (report, bodies) = judged(root.path(), 0.58, 1).await;
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["marker"] == "K001" && finding["line"] == 59)
        .unwrap();
    assert!((finding["judgment"]["value"].as_f64().unwrap() - 0.58).abs() < 1e-12);
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["marker"] == "K002"
                && finding["line"] == 3
                && finding["severity"] == "error")
    );
    assert_eq!(bodies.len(), 7);
    let expected = read(&root.path().join("rulebook/steering.rules.json"));
    for raw in &bodies {
        let body: Value = serde_json::from_slice(raw).unwrap();
        assert!(body["state"].get("text").is_none());
        let marker = if body["questions"].get("kind").is_some() {
            "K001"
        } else {
            "K003"
        };
        assert_eq!(body["questions"], expected[marker]["questions"]);
    }
    let (uncertain, _) = judged(root.path(), 0.65, 1).await;
    assert_eq!(uncertain["status"]["kind"], "findings");
    assert!(!uncertain["uncertain"].as_array().unwrap().is_empty());
    let mut entries = expected;
    entries
        .as_object_mut()
        .unwrap()
        .retain(|key, _| key == "K001");
    save(&root.path().join("rulebook/steering.rules.json"), &entries);
    std::fs::write(root.path().join("rulebook/steering.md"),"Each workflow step states a checkable completion covering its obligations. <!-- K001 -->\n").unwrap();
    let (uncertain, _) = judged(root.path(), 0.65, 5).await;
    assert_eq!(uncertain["status"]["kind"], "uncertain");
    let result = uncertain["uncertain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["where"]["first_line"] == 49)
        .unwrap();
    assert_eq!(result["step"], "quality");
    assert_eq!(result["answers"].as_object().unwrap().len(), 3);
    assert!(result["state"].get("standard").is_some());
    let typed: sunstack_lint::LintReport = serde_json::from_value(uncertain).unwrap();
    for format in [
        sunstack_lint::OutputFormat::Default,
        sunstack_lint::OutputFormat::Github,
        sunstack_lint::OutputFormat::Sarif,
    ] {
        let rendered = sunstack_lint::render(&typed, format);
        assert!(
            rendered.contains("quality") && rendered.contains("checkable"),
            "uncertain step and evidence missing: {format:?}"
        );
    }
}

#[tokio::test]
async fn exact_encoded_escape_heavy_long_question_cap_prevents_dispatch_and_keeps_small_peer() {
    let root = fixture();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["skills/verification-design/SKILL.md"]);
    config["limits"] = json!({"request_max_chars":100000});
    save(&config_path, &config);
    let mut entries = read(&root.path().join("rulebook/steering.rules.json"));
    let mut large = entries["K003"].clone();
    large["name"] = json!("large");
    large["questions"]["names_trigger"]["instructions"]["task"] = json!("é尾\"\\\n".repeat(2000));
    entries = json!({"K003":entries["K003"],"K004":large});
    save(&root.path().join("rulebook/steering.rules.json"), &entries);
    std::fs::write(root.path().join("rulebook/steering.md"),"A description names its trigger. <!-- K003 -->\n\nA description names its trigger with detailed instructions. <!-- K004 -->\n").unwrap();
    assert_encoded_cap_blocks_only_large(root.path()).await;
}
async fn assert_encoded_cap_blocks_only_large(root: &Path) {
    let config_path = root.join("skill-rules.config.json");
    let mut config;
    let (_, bodies) = judged(root, 0.58, 0).await;
    assert_eq!(bodies.len(), 2);
    let large_body = bodies.iter().max_by_key(|body| body.len()).unwrap();
    let character_count = std::str::from_utf8(large_body).unwrap().chars().count();
    assert!(
        large_body.len() > character_count,
        "fixture must distinguish bytes from Unicode characters"
    );
    config = read(&config_path);
    config["limits"]["request_max_chars"] = json!(character_count);
    save(&config_path, &config);
    let (accepted, equal_bodies) = judged(root, 0.58, 0).await;
    assert_eq!(equal_bodies.len(), 2);
    assert_eq!(accepted["usage"]["model_requests"], 2);
    assert_eq!(accepted["incomplete"], json!([]));
    assert_eq!(
        equal_bodies.iter().max_by_key(|body| body.len()).unwrap(),
        large_body
    );
    let cap = character_count - 1;
    config = read(&config_path);
    config["limits"]["request_max_chars"] = json!(cap);
    save(&config_path, &config);
    let (report, bodies) = judged(root, 0.58, 3).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(report["usage"]["model_requests"], 1);
    assert!(
        report["incomplete"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["rule"] == "large" && part["reason"]["kind"] == "request_too_large")
    );
    assert_eq!(report["counts"]["error"], 0);
}

#[tokio::test]
async fn exact_encoded_escape_heavy_evidence_cap_blocks_only_the_large_view() {
    let root = fixture();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["skills/verification-design/SKILL.md"]);
    config["limits"] = json!({"request_max_chars":100000});
    let mut large_kind = config["evidence"]["kinds"]["skill"].clone();
    large_kind["max_chars"] = json!(10000);
    config["evidence"]["kinds"]["large_skill"] = large_kind;
    config["evidence"]["kinds"]["skill"]["max_chars"] = json!(1000);
    save(&config_path, &config);
    let mut entries = read(&root.path().join("rulebook/steering.rules.json"));
    let mut large = entries["K003"].clone();
    large["name"] = json!("large");
    large["evidence"] = json!("large_skill");
    entries = json!({"K003":entries["K003"],"K004":large});
    save(&root.path().join("rulebook/steering.rules.json"), &entries);
    std::fs::write(root.path().join("rulebook/steering.md"),"Description names a trigger. <!-- K003 -->\n\nFull description names a trigger. <!-- K004 -->\n").unwrap();
    let description = serde_json::to_string(&"é尾\"\\\n".repeat(1200)).unwrap();
    std::fs::write(
        root.path().join("skills/verification-design/SKILL.md"),
        format!("---\ndescription: {description}\n---\n"),
    )
    .unwrap();
    assert_encoded_cap_blocks_only_large(root.path()).await;
}
