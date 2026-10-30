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
fn copy(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).unwrap();
        }
    }
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn save(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rulebook-lint/selection"),
        root.path(),
    );
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]
        .as_object_mut()
        .unwrap()
        .retain(|kind, _| ["workflow_section", "step", "load"].contains(&kind.as_str()));
    config["check"]["files"] = json!([
        "skills/practices-research/SKILL.md",
        "skills/presentation-tui/SKILL.md",
        "skills/no-list/SKILL.md"
    ]);
    save(&path, &config);
    root
}
fn command(root: &Path) -> tokio::process::Command {
    let mut command = support::isolated_command(root);
    command
        .args(["lint", "--config", "skill-rules.config.json"])
        .env_remove("OPENROUTER_API_KEY")
        .stdin(Stdio::null());
    command
}
async fn no_key(root: &Path, args: &[&str], expected: i32) -> Value {
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
async fn judged(root: &Path, args: &[&str], probability: f64, exit: i32) -> (Value, Vec<Value>) {
    let mut server = controlled::ControlledServer::start().await;
    let path = root.join("skill-rules.config.json");
    let mut config = read(&path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&path, &config);
    let mut operation = tokio::spawn(
        command(root)
            .args(args)
            .env("OPENROUTER_API_KEY", "SYNTHETIC_SELECTION_PROCESS_KEY")
            .output(),
    );
    let mut bodies = Vec::new();
    let output=tokio::time::timeout(Duration::from_secs(10),async{loop{tokio::select!{output=&mut operation=>break output.unwrap().unwrap(),request=server.receive()=>{let no=request.body["state"]["where"]["file"].as_str().is_some_and(|file|file.contains("presentation-tui"));bodies.push(request.body.clone());request.noul(if no{0.12}else{probability});}}}}).await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(server.finish().await, bodies.len());
    (serde_json::from_slice(&output.stdout).unwrap(), bodies)
}
#[tokio::test]
async fn check_lists_selected_and_contained_kinds_without_key_and_evidence_needs_key_before_acquisition()
 {
    let root = fixture();
    let listing = no_key(root.path(), &["--check"], 0).await;
    assert!(listing.as_array().unwrap().iter().any(|row|row["kind"]=="workflow_section" && row["select"]["yes_at_or_above"]==0.7));
    assert!(
        listing
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["kind"] == "step" && row["within"] == "workflow_section")
    );
    let missing = no_key(root.path(), &["--all", "--evidence", "--format", "json"], 2).await;
    assert_eq!(missing["status"]["kind"], "invalid");
    assert_eq!(missing["usage"]["checked_files_acquired"], 0);
}
#[tokio::test]
async fn binary_selection_reports_every_outcome_and_zero_files_then_builds_exact_chosen_steps() {
    let root = fixture();
    let (listing, bodies) = judged(root.path(), &["--all", "--evidence", "step"], 0.93, 0).await;
    assert_eq!(bodies.len(), 2);
    assert!(
        bodies
            .iter()
            .all(|body| body["questions"].get("c1").is_some())
    );
    assert_eq!(
        listing["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|piece| piece["where"]["first_line"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [21, 22, 24, 25, 26, 27]
    );
    let selections = listing["selections"].as_array().unwrap();
    assert_eq!(selections.len(), 2);
    assert_eq!(selections[0]["outcome"]["kind"], "selected");
    assert_eq!(selections[1]["outcome"]["kind"], "not_selected");
    let counts = listing["file_counts"].as_array().unwrap();
    assert_eq!(counts.len(), 3);
    let no_list = counts
        .iter()
        .find(|count| count["file"].as_str().unwrap().contains("no-list"))
        .unwrap();
    assert_eq!(
        no_list["kinds"]["workflow_section"]["selection"]["candidates"],
        0
    );
    let (uncertain, _) = judged(root.path(), &["--all", "--format", "json"], 0.5, 5).await;
    assert_eq!(uncertain["status"]["kind"], "uncertain");
    assert_eq!(uncertain["incomplete"], json!([]));
    assert_eq!(uncertain["usage"]["model_requests"], 2);
    let typed: sunstack_lint::LintReport = serde_json::from_value(uncertain).unwrap();
    let human = sunstack_lint::render(&typed, sunstack_lint::OutputFormat::Default);
    assert!(
        human.contains("Selection workflow_section")
            && human.contains("presentation-tui")
            && human.contains("none selected")
    );
}
#[tokio::test]
async fn selection_refusal_target_has_the_intended_no_target_field_diagnostic() {
    let root = fixture();
    let mut invalid = read(&root.path().join("refusals/question-target.json"));
    invalid["evidence"]["ast_grep"] = json!("evidence/sgconfig.yml");
    save(&root.path().join("skill-rules.config.json"), &invalid);
    let output = command(root.path()).arg("--check").output().await.unwrap();
    assert_eq!(output.status.code(), Some(2));
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostics.contains("selection question cannot name a target field"));
    assert!(!diagnostics.contains("instruction must name defined fields"));
}

#[tokio::test]
async fn null_match_sources_are_located_check_refusals_and_single_strings_still_pass() {
    for from in ["inside", "enclosing", "same_file"] {
        for keys in [
            json!({"rule":null,"kind":"step"}),
            json!({"rule":"step","kind":null}),
            json!({"rule":null}),
            json!({"kind":null}),
        ] {
            let root = fixture();
            let path = root.path().join("skill-rules.config.json");
            let mut config = read(&path);
            let mut field = keys;
            field["from"] = json!(from);
            config["evidence"]["kinds"]["load"]["fields"]["context"] = field;
            save(&path, &config);
            let output = command(root.path()).arg("--check").output().await.unwrap();
            assert_eq!(
                output.status.code(),
                Some(2),
                "{from}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            let diagnostic = String::from_utf8_lossy(&output.stderr);
            assert!(
                diagnostic.contains("skill-rules.config.json:"),
                "{diagnostic}"
            );
            assert!(
                diagnostic.contains("null") || diagnostic.contains("exactly one"),
                "{diagnostic}"
            );
        }
        for keys in [json!({"rule":"step"}), json!({"kind":"step"})] {
            let root = fixture();
            let path = root.path().join("skill-rules.config.json");
            let mut config = read(&path);
            let mut field = keys;
            field["from"] = json!(from);
            config["evidence"]["kinds"]["load"]["fields"]["context"] = field;
            save(&path, &config);
            no_key(root.path(), &["--check"], 0).await;
        }
    }
}

#[tokio::test]
async fn equal_range_selection_is_asked_once_and_positive_answer_survives() {
    let root = fixture();
    std::fs::write(root.path().join("skills/custom.md"), "Hello\n").unwrap();
    std::fs::write(
        root.path().join("evidence/rules/selection.yml"),
        "id: duplicate\nlanguage: Markdown\nrule: {any: [{kind: document}, {kind: paragraph}]}\n",
    )
    .unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["check"]["files"] = json!(["skills/custom.md"]);
    config["evidence"]["kinds"] = json!({"selected":{"rule":"duplicate","fields":{"body":{"from":"remainder"}},"select":{"question":{"type":"noul","instructions":{"check":"body","task":"Controlled candidate answer"}},"yes_at_or_above":0.7,"no_at_or_below":0.3}}});
    let mut server = controlled::ControlledServer::start().await;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&path, &config);
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["evidence"] = json!("selected");
    save(&root.path().join("rules.json"), &rules);
    let operation = tokio::spawn(
        command(root.path())
            .args(["--all", "--evidence"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_SELECTION_KEY")
            .output(),
    );
    let request = server.next().await;
    let questions = request.body["questions"].as_object().unwrap();
    let question_count = questions.len();
    let answers = questions
        .keys()
        .map(|key| {
            (
                key.clone(),
                json!({"type":"noul","noul":if key=="c1" {0.93} else {0.12}}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
    let output = tokio::time::timeout(Duration::from_secs(10), operation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(question_count, 1);
    assert_eq!(report["selections"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["selections"][0]["outcome"],
        json!({"kind":"selected","yes":0.93})
    );
    assert_eq!(report["pieces"].as_array().unwrap().len(), 1);
    let count = &report["file_counts"][0]["kinds"]["selected"];
    assert_eq!(count["pieces"], 1);
    assert_eq!(
        count["selection"],
        json!({"candidates":1,"asked":1,"selected":1,"not_selected":0,"uncertain":0,"incomplete":0})
    );
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
async fn deleting_only_parent_relative_target_selects_loader_and_keeps_missing_target_evidence() {
    for (deleted, outside) in [(false, false), (true, false), (true, true)] {
        let root = fixture();
        std::fs::create_dir_all(root.path().join("skills/loader")).unwrap();
        std::fs::create_dir_all(root.path().join("skills/target")).unwrap();
        std::fs::write(root.path().join("skills/loader/SKILL.md"), "# Loader\n\n## Workflow\n\n1. Inspect inputs.\n\n   load `../target/SKILL.md` and return its observations.\n\n2. Record results.\n").unwrap();
        if outside {
            let loader = root.path().join("skills/loader/SKILL.md");
            let text = std::fs::read_to_string(&loader)
                .unwrap()
                .replace("../target/SKILL.md", "../../../skills/target/SKILL.md");
            std::fs::write(loader, text).unwrap();
        }
        let target = root.path().join("skills/target/SKILL.md");
        std::fs::write(
            &target,
            "---\ndescription: Use when checking a target.\n---\n",
        )
        .unwrap();
        let path = root.path().join("skill-rules.config.json");
        let mut config = read(&path);
        config["check"]["files"] = json!(["skills/loader/SKILL.md"]);
        config["evidence"]["kinds"]["load"]["within"] = json!("workflow_section");
        let mut server = controlled::ControlledServer::start().await;
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        save(&path, &config);
        git(root.path(), &["init", "--quiet"]);
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "--quiet", "-m", "baseline"]);
        if deleted {
            std::fs::remove_file(&target).unwrap();
        } else {
            std::fs::write(
                &target,
                "---\ndescription: Use when checking a modified target.\n---\n",
            )
            .unwrap();
        }
        assert_eq!(
            git(root.path(), &["diff", "--name-only"]),
            "skills/target/SKILL.md\n"
        );
        let mut operation = tokio::spawn(
            command(root.path())
                .args(["--evidence", "--since", "HEAD"])
                .env("OPENROUTER_API_KEY", "SYNTHETIC_TARGET_KEY")
                .output(),
        );
        let mut bodies = Vec::new();
        let output = tokio::time::timeout(Duration::from_secs(10), async { loop { tokio::select! { output = &mut operation => break output.unwrap().unwrap(), request = server.receive() => { bodies.push(request.body.clone()); request.noul(0.93); } } } }).await.unwrap();
        assert_eq!(server.finish().await, bodies.len());
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        if outside {
            assert!(bodies.is_empty());
            assert_eq!(report["selections"], json!([]));
            assert_eq!(report["pieces"], json!([]));
            continue;
        }
        assert_eq!(bodies.len(), 1, "selection must ask for unchanged loader");
        assert_eq!(report["selections"].as_array().unwrap().len(), 1);
        let load = report["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|piece| piece["kind"] == "load")
            .unwrap();
        assert_eq!(load["fields"]["targets"]["values"][0]["exists"], !deleted);
        if deleted {
            assert_eq!(report["usage"]["checked_files_acquired"], 1);
            assert_eq!(
                load["fields"]["targets"]["values"][0]["path"],
                "skills/loader/../target/SKILL.md"
            );
        }
    }
}
