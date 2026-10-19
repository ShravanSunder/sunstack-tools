#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled;
#[path = "../../sunstack-lint/tests/support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Output, Stdio},
    time::Duration,
};
use sunstack_decisions::CancellationToken;
use tokio::process::Command;

fn command(root: &Path) -> Command {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut command = Command::new(workspace.join("scripts/cargo-debug-signing-runner.sh"));
    command
        .arg(env!("CARGO_BIN_EXE_sunstack"))
        .arg("lint")
        .current_dir(root)
        .env_remove("OPENROUTER_API_KEY")
        .stdin(Stdio::null())
        .kill_on_drop(true);
    command
}
async fn invoke(root: &Path, arguments: &[&str], exit: i32) -> Output {
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        command(root).args(arguments).output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        result.status.code(),
        Some(exit),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    result
}
async fn report(root: &Path, arguments: &[&str], exit: i32) -> Value {
    let mut arguments = arguments.to_vec();
    arguments.extend(["--format", "json"]);
    serde_json::from_slice(&invoke(root, &arguments, exit).await.stdout).unwrap()
}

#[tokio::test]
async fn real_cli_json_equals_the_library_and_each_format_preserves_the_finding() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let cli = report(root.path(), &["--all"], 1).await;
    let library = sunstack_lint::run(fixture::request(root.path()), CancellationToken::new()).await;
    assert_eq!(cli, serde_json::to_value(library).unwrap());
    let human = invoke(root.path(), &["--all"], 1).await;
    assert!(String::from_utf8(human.stdout).unwrap().contains(
        "docs/one.md:2:14: error [docs/description] Description must begin with Use when."
    ));
    let github = invoke(root.path(), &["--all", "--format", "github"], 1).await;
    assert!(
        String::from_utf8(github.stdout)
            .unwrap()
            .contains("::error file=docs/one.md,line=2,col=14,")
    );
    let sarif: Value = serde_json::from_slice(
        &invoke(root.path(), &["--all", "--format", "sarif"], 1)
            .await
            .stdout,
    )
    .unwrap();
    assert_eq!(sarif["runs"][0]["results"][0]["ruleId"], "docs/description");
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startColumn"],
        14
    );
}

#[tokio::test]
async fn config_check_admits_integer_type_maxima_and_refuses_the_next_integers() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    for (field, maximum, overflow) in [
        ("max_files", u64::from(u32::MAX), "4294967296"),
        ("max_file_bytes", u64::MAX, "18446744073709551616"),
        ("max_total_bytes", u64::MAX, "18446744073709551616"),
    ] {
        let mut config = baseline.clone();
        config["limits"] = json!({field: maximum});
        fixture::config_file(root.path(), &config);
        invoke(root.path(), &["--check"], 0).await;
        let raw = format!(
            "{{\"rule_sets\":{},\"limits\":{{\"{field}\":{overflow}}}}}",
            baseline["rule_sets"]
        );
        std::fs::write(root.path().join("sunstack.lint.json"), raw).unwrap();
        invoke(root.path(), &["--check"], 2).await;
    }
}

#[tokio::test]
async fn nested_config_reports_binding_root_paths_in_every_format_and_incomplete_parts() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("config");
    std::fs::create_dir_all(&nested).unwrap();
    let mut config = fixture::write(&nested);
    std::fs::rename(nested.join("docs"), root.path().join("docs")).unwrap();
    config["rule_sets"]["docs"]["files"] = json!(["../docs/*.md"]);
    // Policy still matches paths relative to the governing config.
    config["rules"] = json!({"docs/*":"off"});
    config["overrides"] = json!([{"files":["../docs/*.md"],"rules":{"docs/*":"error"}}]);
    fixture::config_file(&nested, &config);
    fixture::commit(root.path(), "nested governing config");
    let json = report(&nested, &["--all"], 1).await;
    assert_eq!(json["findings"][0]["file"], "docs/one.md");
    assert_eq!(json["findings"][0]["severity_origin"]["kind"], "override");
    let human = String::from_utf8(invoke(&nested, &["--all"], 1).await.stdout).unwrap();
    assert!(human.contains("docs/one.md:2:14: error"));
    assert!(!human.contains("../docs/one.md"));
    let github = String::from_utf8(
        invoke(&nested, &["--all", "--format", "github"], 1)
            .await
            .stdout,
    )
    .unwrap();
    assert!(github.contains("file=docs/one.md,line=2,col=14,"));
    let sarif: Value = serde_json::from_slice(
        &invoke(&nested, &["--all", "--format", "sarif"], 1)
            .await
            .stdout,
    )
    .unwrap();
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "docs/one.md"
    );
    let mut server = controlled::ControlledServer::start().await;
    let rules = nested.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules).unwrap()).unwrap();
    rule["evidence"][0]["select"] = json!({"kind":"paragraph","target_id":"U001"});
    rule["verdict"] = json!({"kind":"noul","question":{"type":"noul","instructions":"Does it comply?"},"threshold":0.8});
    std::fs::write(rules, serde_json::to_vec(&rule).unwrap()).unwrap();
    std::fs::write(root.path().join("docs/one.md"), "U001: failing text.\n").unwrap();
    config["backend"] =
        json!({"kind":"jev_openrouter","model":"jev-latest","endpoint":server.endpoint});
    fixture::config_file(&nested, &config);
    let operation = command(&nested)
        .args(["--all", "--format", "json"])
        .env("OPENROUTER_API_KEY", "SYNTHETIC_CLI_CREDENTIAL")
        .output();
    let task = tokio::spawn(operation);
    server
        .next()
        .await
        .respond(400, json!({"error":"fixture refusal"}));
    let result = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.status.code(), Some(3));
    let incomplete: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        incomplete["incomplete"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["file"] == "docs/one.md")
    );
    assert!(
        !incomplete["incomplete"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["file"] == "../docs/one.md")
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn clean_named_paths_history_and_warning_limit_keep_exit_and_selection_semantics() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    fixture::commit(root.path(), "initial");
    let clean = report(root.path(), &[], 0).await;
    assert_eq!(clean["status"]["kind"], "nothing_selected");
    assert!(
        String::from_utf8(invoke(root.path(), &[], 0).await.stdout)
            .unwrap()
            .contains("No checks selected")
    );
    assert_eq!(
        report(root.path(), &["docs/one.md"], 1).await["counts"]["error"],
        1
    );
    std::fs::write(root.path().join("unrelated.txt"), "unrelated").unwrap();
    assert_eq!(
        report(root.path(), &[], 0).await["status"]["kind"],
        "nothing_selected"
    );
    let rules = root.path().join("rules.jsonl");
    let bytes = std::fs::read_to_string(&rules).unwrap();
    std::fs::write(&rules, format!("{bytes}\n")).unwrap();
    assert_eq!(
        report(root.path(), &[], 1).await["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fixture::commit(root.path(), "rule edited");
    assert_eq!(
        report(root.path(), &["--since", "HEAD~1"], 1).await["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    config["rules"] = json!({"docs/*":"warn"});
    fixture::config_file(root.path(), &config);
    let widened = report(root.path(), &[], 0).await;
    assert_eq!(widened["scope"]["widened"]["kind"], "config_changed");
    assert_eq!(widened["counts"]["warn"], 1);
    assert_eq!(
        report(root.path(), &["--max-warnings", "0"], 1).await["counts"]["warn"],
        1
    );
    config["rules"] = json!({"docs/*":"off"});
    fixture::config_file(root.path(), &config);
    let off = report(root.path(), &["--all"], 0).await;
    assert_eq!(off["exclusions"]["off"], 1);
    assert_eq!(off["status"]["kind"], "nothing_selected");
    config["rules"] = json!({});
    config["ignore_patterns"] = json!(["docs/explicit.md"]);
    fixture::config_file(root.path(), &config);
    std::fs::write(root.path().join(".gitignore"), "docs/git.md\n").unwrap();
    for name in ["explicit.md", "git.md"] {
        std::fs::write(
            root.path().join("docs").join(name),
            "---\ndescription: Wrong\n---\n",
        )
        .unwrap();
    }
    std::fs::write(
        root.path().join("docs/one.md"),
        "---\ndescription: Use when checking documents\n---\n",
    )
    .unwrap();
    let clean = report(root.path(), &["--all"], 0).await;
    assert_eq!(clean["status"]["kind"], "clean");
    assert_eq!(clean["findings"], json!([]));
    assert_eq!(clean["exclusions"]["ignored"], 2);
}

#[tokio::test]
async fn discovery_inspection_and_init_need_no_key_or_target_read_and_bad_flags_are_invalid() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["backend"] =
        json!({"kind":"jev_openrouter","model":"jev-latest","endpoint":"http://127.0.0.1:1"});
    config["rules"] = json!({"docs/*":"off"});
    config["overrides"] = json!([{"files":["docs/one.md"],"rules":{"docs/*":"warn"}}]);
    fixture::config_file(root.path(), &config);
    // A FIFO would block if --check opened target content. No writer is started.
    std::fs::remove_file(root.path().join("docs/one.md")).unwrap();
    assert!(
        std::process::Command::new("/usr/bin/mkfifo")
            .arg(root.path().join("docs/one.md"))
            .status()
            .unwrap()
            .success()
    );
    invoke(&root.path().join("docs"), &["--check"], 0).await;
    invoke(
        &root.path().join("docs"),
        &["--check", "--config", "../sunstack.lint.json"],
        0,
    )
    .await;
    let listed: Value =
        serde_json::from_slice(&invoke(root.path(), &["--rules"], 0).await.stdout).unwrap();
    assert_eq!(listed[0]["rule"], "docs/description");
    assert_eq!(listed[0]["authored"], "error");
    assert_eq!(listed[0]["configured"], "off");
    assert_eq!(listed[0]["kind"]["kind"], "mechanical");
    assert_eq!(listed[0]["source"]["line"], 1);
    let for_file: Value = serde_json::from_slice(
        &invoke(&root.path().join("docs"), &["--rules", "one.md"], 0)
            .await
            .stdout,
    )
    .unwrap();
    assert_eq!(
        for_file[0]["for_file"],
        json!({"setting":"warn","decided_by":{"kind":"override","index":0}})
    );
    let parent_path: Value = serde_json::from_slice(
        &invoke(&root.path().join("docs"), &["--rules", "../docs/one.md"], 0)
            .await
            .stdout,
    )
    .unwrap();
    assert_eq!(parent_path, for_file);
    for arguments in [
        vec!["--all", "--since", "HEAD"],
        vec!["--check", "--all"],
        vec!["--rules", "--check"],
        vec!["--init", "--config", "other"],
        vec!["--format", "unknown"],
        vec!["--max-warnings", "-1"],
    ] {
        invoke(root.path(), &arguments, 2).await;
    }
    let empty = tempfile::tempdir().unwrap();
    invoke(empty.path(), &["--check"], 2).await;
    invoke(empty.path(), &["--init"], 0).await;
    let original = std::fs::read(empty.path().join("sunstack.lint.json")).unwrap();
    let skeleton: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(skeleton.as_object().unwrap().len(), 2);
    assert_eq!(skeleton["rule_sets"], json!({}));
    assert!(skeleton["$schema"].is_string());
    invoke(empty.path(), &["--init"], 2).await;
    invoke(empty.path(), &["--check"], 2).await;
    assert_eq!(
        std::fs::read(empty.path().join("sunstack.lint.json")).unwrap(),
        original
    );
    fixture::write(empty.path());
    invoke(empty.path(), &["--check"], 0).await;
    assert_eq!(
        report(empty.path(), &[], 2).await["diagnostics"][0]["code"],
        "no_git_for_default_scope"
    );
    assert_eq!(
        report(empty.path(), &["missing.md"], 2).await["diagnostics"][0]["code"],
        "path_not_found"
    );
}

fn judged(root: &Path, endpoint: &str) {
    let mut config = fixture::write(root);
    config["backend"] = json!({"kind":"jev_openrouter","model":"jev-latest","endpoint":endpoint});
    fixture::config_file(root, &config);
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["select"] = json!({"kind":"paragraph","target_id":"U001"});
    rule["verdict"] = json!({"kind":"noul","question":{"type":"noul","instructions":"Does the paragraph comply?"},"threshold":0.8});
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    std::fs::write(root.join("docs/one.md"), "U001: failing fixture.\n").unwrap();
}

#[tokio::test]
async fn no_key_sends_no_request_provider_failure_is_incomplete_and_real_signals_cancel() {
    for case in ["missing-key", "provider-failure", "-INT", "-TERM"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        judged(root.path(), &server.endpoint);
        if case == "missing-key" {
            let missing = report(root.path(), &["--all"], 2).await;
            assert_eq!(missing["diagnostics"][0]["code"], "credential_missing");
            assert_eq!(server.finish().await, 0);
            continue;
        }
        let child = command(root.path())
            .args(["--all", "--format", "json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_CLI_CREDENTIAL")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = server.next().await;
        let expected = if case == "provider-failure" {
            request.respond(400, json!({"error":"fixture refusal"}));
            3
        } else {
            // Receiving a full real request establishes operation-owned readiness.
            assert!(
                Command::new("/bin/kill")
                    .args([case, &child.id().unwrap().to_string()])
                    .status()
                    .await
                    .unwrap()
                    .success()
            );
            4
        };
        let result = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            report["status"]["kind"],
            if expected == 3 {
                "incomplete"
            } else {
                "cancelled"
            }
        );
        assert!(!String::from_utf8_lossy(&result.stdout).contains("SYNTHETIC_CLI_CREDENTIAL"));
        assert_eq!(server.finish().await, 1);
    }
}
