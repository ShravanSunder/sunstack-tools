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

fn limit_fixture(fields: Value, decision: Value) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("evidence/rules")).unwrap();
    std::fs::write(
        root.path().join("evidence/sgconfig.yml"),
        "ruleDirs: [rules]\n",
    )
    .unwrap();
    std::fs::write(root.path().join("evidence/rules/pieces.yml"), "id: document\nlanguage: Markdown\nrule:\n  kind: document\n---\nid: paragraph\nlanguage: Markdown\nrule:\n  kind: paragraph\n").unwrap();
    std::fs::write(
        root.path().join("rulebook.md"),
        "Document meets the standard. <!-- K001 -->\n",
    )
    .unwrap();
    save(
        &root.path().join("rules.json"),
        &json!({"K001":{"name":"document_standard","evidence":"review","severity":"error","decision":decision}}),
    );
    save(
        &root.path().join("skill-rules.config.json"),
        &json!({
            "rulebook":[{"markdown":"rulebook.md","rules":"rules.json"}],"marker_prefix":"K",
            "check":{"files":["docs/*.md"]},
            "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{"review":{"rule":"document","max_chars":1000,"fields":fields}}},
            "decision_model":{"model":"jev-latest"}
        }),
    );
    std::fs::create_dir(root.path().join("docs")).unwrap();
    root
}
fn mechanical_graph(tool: Value) -> Value {
    json!({"start":"check","steps":{"check":{"mechanical_check":tool,"yes":"pass","no":{"finding":"Standard failed."}}}})
}

#[tokio::test]
async fn literal_cut_looking_tail_reaches_the_real_asking_path() {
    for tail in ["ordinary author text", "[cut: literal author text]"] {
        let root = limit_fixture(
            json!({"items":{"from":"same_file","rule":"paragraph"}}),
            json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.6,"yes":"pass","no":{"finding":"Needs work."}}}}),
        );
        let mut entries = read(&root.path().join("rules.json"));
        entries["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"items","task":"Does this document meet the standard?"}}});
        save(&root.path().join("rules.json"), &entries);
        let mut paragraphs = vec!["a".repeat(32); 79];
        paragraphs.push(tail.into());
        std::fs::write(root.path().join("docs/one.md"), paragraphs.join("\n\n")).unwrap();
        let listed = invoke(root.path(), &["--all", "--evidence", "review"], 0).await;
        assert_eq!(
            listed["pieces"][0]["fields"]["items"]["values"],
            json!(paragraphs)
        );
        let (report, bodies) = judged(root.path(), 0.9, 0).await;
        assert_eq!(report["incomplete"], json!([]));
        assert_eq!(report["usage"]["model_requests"], 1);
        assert_eq!(bodies.len(), 1);
    }
}

#[tokio::test]
async fn match_limit_is_shared_across_checked_files_with_a_sufficient_cap_control() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    // Only document matches participate in this budget counterexample.
    std::fs::write(
        root.path().join("evidence/rules/pieces.yml"),
        "id: document\nlanguage: Markdown\nrule:\n  kind: document\n",
    )
    .unwrap();
    for name in ["a.md", "b.md"] {
        std::fs::write(root.path().join("docs").join(name), "Long document.\n").unwrap();
    }
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["limits"] = json!({"max_matches":1});
    save(&path, &config);
    let limited = invoke(root.path(), &["--all", "--format", "json"], 3).await;
    assert_eq!(limited["counts"]["error"], 1);
    assert_eq!(limited["findings"][0]["file"], "docs/a.md");
    assert!(!limited["incomplete"].as_array().unwrap().is_empty());
    assert_eq!(limited["limits"]["max_matches"], 1);
    assert!(
        limited["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"] == "max_matches")
    );
    config["limits"]["max_matches"] = json!(2);
    save(&path, &config);
    let complete = invoke(root.path(), &["--all", "--format", "json"], 1).await;
    assert_eq!(complete["counts"]["error"], 2);
    assert_eq!(complete["incomplete"], json!([]));
}

#[tokio::test]
async fn target_content_shares_total_bytes_and_cached_targets_are_charged_once() {
    let root = limit_fixture(
        json!({"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":10}}),
        mechanical_graph(json!({"tool":"file_exists","field":"targets"})),
    );
    let checked = "Open `../one.md`, `../two.md`, `../three.md`, `../one.md` and `a.md`.\n";
    std::fs::write(root.path().join("docs/a.md"), checked).unwrap();
    for name in ["one.md", "two.md", "three.md"] {
        std::fs::write(root.path().join(name), "x".repeat(400)).unwrap();
    }
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["limits"] = json!({"max_file_bytes":500,"max_total_bytes":600});
    save(&path, &config);
    let limited = invoke(root.path(), &["--all", "--format", "json"], 3).await;
    assert!(!limited["incomplete"].as_array().unwrap().is_empty());
    assert!(
        limited["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"] == "max_total_bytes")
    );
    // Exact distinct-byte total must pass: repeated and already-checked targets add no bytes.
    config["limits"]["max_total_bytes"] = json!(checked.len() + 1200);
    save(&path, &config);
    let complete = invoke(root.path(), &["--all", "--format", "json"], 0).await;
    assert_eq!(complete["incomplete"], json!([]));
    assert_eq!(complete["usage"]["checked_files_acquired"], 4);
}

#[tokio::test]
async fn file_limit_counts_checked_and_target_content_once_per_resolved_path() {
    let root = limit_fixture(
        json!({"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":10}}),
        mechanical_graph(json!({"tool":"file_exists","field":"targets"})),
    );
    std::fs::write(
        root.path().join("docs/a.md"),
        "Open `a.md`, `../one.md`, `.././one.md`, `../two.md`, `../three.md` and `../one.md`.\n",
    )
    .unwrap();
    for name in ["one.md", "two.md", "three.md"] {
        std::fs::write(root.path().join(name), "x".repeat(400)).unwrap();
    }
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["limits"] = json!({"max_files":1,"max_file_bytes":500,"max_total_bytes":6000});
    save(&path, &config);
    let limited = invoke(root.path(), &["--all", "--format", "json"], 3).await;
    assert_eq!(limited["usage"]["checked_files_acquired"], 1);
    assert_eq!(limited["status"]["kind"], "incomplete");
    assert!(
        limited["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"] == "max_files")
    );
    config["limits"]["max_files"] = json!(4);
    save(&path, &config);
    let complete = invoke(root.path(), &["--all", "--format", "json"], 0).await;
    assert_eq!(complete["incomplete"], json!([]));
    assert_eq!(complete["usage"]["checked_files_acquired"], 4);
    // Checked selection truncation remains deterministic and names the same cap.
    config["limits"]["max_files"] = json!(1);
    save(&path, &config);
    std::fs::write(root.path().join("docs/a.md"), "No targets.\n").unwrap();
    std::fs::write(root.path().join("docs/b.md"), "No targets.\n").unwrap();
    let truncated = invoke(root.path(), &["--all", "--format", "json"], 3).await;
    assert_eq!(truncated["usage"]["checked_files_acquired"], 1);
    assert!(
        truncated["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"] == "max_files")
    );
}
