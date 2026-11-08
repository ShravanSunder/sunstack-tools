use super::*;
use std::process::Output;

fn summary_fixture(unsure_count: usize, warning_count: usize) -> tempfile::TempDir {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"local","steps":{
            "local":{"mechanical_check":{"tool":"contains","field":"body","text":"FINDING"},"yes":{"finding":"Keep this warning."},"no":"quality"},
            "quality":{"ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Low quality."},"uncertain":"uncertain"}
        }}),
    );
    let rules_path = root.path().join("rules.json");
    let mut rules = read(&rules_path);
    rules["K001"]["name"] = json!("r02");
    rules["K001"]["severity"] = json!("warn");
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body"}}});
    rules["K002"] = rules["K001"].clone();
    rules["K002"]["name"] = json!("other_rule");
    rules["K002"]["decision"]["steps"]["local"] = json!({"mechanical_check":{"tool":"contains","field":"body","text":"SECOND_RULE"},"yes":"quality","no":"pass"});
    save(&rules_path, &rules);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Document meets the standard. <!-- K001 -->\n\nAnother standard. <!-- K002 -->\n",
    )
    .unwrap();
    for index in 0..unsure_count {
        std::fs::write(
            root.path().join(format!("docs/unsure-{index:02}.md")),
            if index < 2 {
                "SECOND_RULE\n"
            } else {
                "Unsettled evidence.\n"
            },
        )
        .unwrap();
    }
    for index in 0..warning_count {
        std::fs::write(
            root.path().join(format!("docs/warning-{index}.md")),
            "FINDING\n",
        )
        .unwrap();
    }
    root
}

async fn run_format(
    root: &Path,
    server: &mut controlled::ControlledServer,
    format: &str,
    binary: Option<&Path>,
    expected_exit: i32,
) -> Output {
    let mut invocation = if let Some(binary) = binary {
        support::isolated_command_with_binary(root, binary)
    } else {
        support::isolated_command(root)
    };
    invocation.args([
        "lint",
        "--config",
        "skill-rules.config.json",
        "--all",
        "--format",
        format,
        "--report",
        "run.json",
    ]);
    invocation.env("OPENROUTER_API_KEY", "SYNTHETIC_LOOPBACK_ONLY");
    let mut operation = tokio::spawn(invocation.output());
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                output = &mut operation => break output.unwrap().unwrap(),
                request = server.receive() => request.noul(0.5),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

async fn show_result(root: &Path, id: usize, binary: Option<&Path>) -> Output {
    let mut invocation = if let Some(binary) = binary {
        support::isolated_command_with_binary(root, binary)
    } else {
        support::isolated_command(root)
    };
    let output = invocation
        .args(["lint", "show", "run.json", &id.to_string()])
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .await
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[tokio::test]
async fn many_unsure_results_are_grouped_without_changing_findings_or_saved_details() {
    // Arrange: 66 abstentions for R02, two for another rule and two real warnings.
    let root = summary_fixture(66, 2);
    let mut server = controlled::ControlledServer::start().await;
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&config_path, &config);
    let baseline = std::env::var_os("SUNSTACK_UNSURE_BASELINE_BINARY").map(PathBuf::from);

    // Act: all channels use the same fixture, endpoint and deterministic answers.
    let human = run_format(root.path(), &mut server, "default", None, 5).await;
    let report_bytes = std::fs::read(root.path().join("run.json")).unwrap();
    let report: Value = serde_json::from_slice(&report_bytes).unwrap();
    let human = String::from_utf8(human.stdout).unwrap();
    let unsure = report["uncertain"].as_array().unwrap();
    assert_eq!(unsure.len(), 68);
    assert_eq!(report["counts"], json!({"error":0,"warn":2}));
    assert_eq!(report["status"]["kind"], "uncertain");
    for format in ["json", "github", "sarif"] {
        let output = run_format(root.path(), &mut server, format, None, 5).await;
        assert_eq!(
            std::fs::read(root.path().join("run.json")).unwrap(),
            report_bytes
        );
        if format == "json" {
            let mut expected = report_bytes.clone();
            expected.push(b'\n');
            assert_eq!(output.stdout, expected);
        }
        if let Some(binary) = baseline.as_deref() {
            let before = run_format(root.path(), &mut server, format, Some(binary), 5).await;
            assert_eq!(before.stdout, output.stdout, "{format} changed");
            assert_eq!(
                std::fs::read(root.path().join("run.json")).unwrap(),
                report_bytes,
                "saved report changed"
            );
        }
    }
    let ids: Vec<_> = unsure
        .iter()
        .map(|result| result["id"].as_u64().unwrap() as usize)
        .collect();
    for id in &ids {
        let detail = show_result(root.path(), *id, None).await;
        let value: Value = serde_json::from_slice(&detail.stdout).unwrap();
        assert_eq!(value["result"]["id"], *id);
        assert_eq!(value["result"]["outcome"], "uncertain");
        if let Some(binary) = baseline.as_deref() {
            assert_eq!(
                show_result(root.path(), *id, Some(binary)).await.stdout,
                detail.stdout,
                "show changed for {id}"
            );
        }
    }
    if let Some(binary) = baseline.as_deref() {
        let before = run_format(root.path(), &mut server, "default", Some(binary), 5).await;
        let before = String::from_utf8(before.stdout).unwrap();
        let finding_lines = |text: &str| {
            text.lines()
                .filter(|line| line.contains(": warn ["))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(finding_lines(&before), finding_lines(&human));
        assert_eq!(
            std::fs::read(root.path().join("run.json")).unwrap(),
            report_bytes
        );
        assert_eq!(
            before
                .lines()
                .filter(|line| line.contains(": unsure ["))
                .count(),
            68
        );
    }
    assert_eq!(
        server.finish().await,
        if baseline.is_some() { 544 } else { 272 }
    );

    // Assert: short pointers replace individual locations, not their report data.
    let summaries: Vec<_> = human
        .lines()
        .filter(|line| line.contains(" unsure ["))
        .collect();
    assert_eq!(summaries.len(), 2, "{human}");
    assert!(
        summaries[0].starts_with("2 unsure [other_rule];"),
        "{human}"
    );
    assert!(summaries[1].starts_with("66 unsure [r02];"), "{human}");
    for (rule, line) in [("other_rule", summaries[0]), ("r02", summaries[1])] {
        let id = unsure.iter().find(|result| result["rule"] == rule).unwrap()["id"]
            .as_u64()
            .unwrap();
        assert!(line.contains(&format!("lint show <report> {id}")), "{line}");
        assert!(line.contains("--report <report>"), "{line}");
    }
    assert_eq!(
        human
            .lines()
            .filter(|line| line.contains(": warn [r02] Keep this warning."))
            .count(),
        2
    );
    assert!(human.contains("Uncertain: 0 error(s), 2 warning(s)"));
    assert!(!human.contains("at step quality"));
}

#[tokio::test]
async fn unsure_only_summary_keeps_uncertain_status_without_inventing_findings() {
    let root = summary_fixture(2, 0);
    let mut server = controlled::ControlledServer::start().await;
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&config_path, &config);

    let output = run_format(root.path(), &mut server, "default", None, 5).await;

    assert_eq!(server.finish().await, 4);
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(human.contains("2 unsure [other_rule];"), "{human}");
    assert!(human.contains("2 unsure [r02];"), "{human}");
    assert!(
        human.contains("Uncertain: 0 error(s), 0 warning(s)"),
        "{human}"
    );
    let report = read(&root.path().join("run.json"));
    assert_eq!(report["findings"], json!([]));
    assert_eq!(report["uncertain"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn no_unsure_results_keep_warning_and_error_findings_without_a_summary() {
    let root = summary_fixture(0, 1);
    let rules_path = root.path().join("rules.json");
    let mut rules = read(&rules_path);
    rules["K001"].as_object_mut().unwrap().remove("questions");
    rules["K001"]["decision"] = json!({"start":"local","steps":{
        "local":{"mechanical_check":{"tool":"contains","field":"body","text":"FINDING"},"yes":{"finding":"Keep this warning."},"no":"pass"}
    }});
    rules["K002"] = rules["K001"].clone();
    rules["K002"]["name"] = json!("error_rule");
    rules["K002"]["severity"] = json!("error");
    save(&rules_path, &rules);

    let output = command(root.path())
        .args(["--all", "--report", "run.json"])
        .output()
        .await
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(
        human.contains(": warn [r02] Keep this warning. #1"),
        "{human}"
    );
    assert!(
        human.contains(": error [error_rule] Keep this warning. #2"),
        "{human}"
    );
    assert!(
        human.contains("Lint findings: 1 error(s), 1 warning(s)"),
        "{human}"
    );
    assert!(!human.contains("unsure"), "{human}");
    assert_eq!(read(&root.path().join("run.json"))["uncertain"], json!([]));
}

#[tokio::test]
async fn error_finding_stays_visible_and_takes_precedence_over_unsure_summary() {
    let root = summary_fixture(2, 1);
    let rules_path = root.path().join("rules.json");
    let mut rules = read(&rules_path);
    rules["K001"]["severity"] = json!("error");
    save(&rules_path, &rules);
    let mut server = controlled::ControlledServer::start().await;
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&config_path, &config);

    let output = run_format(root.path(), &mut server, "default", None, 1).await;

    assert_eq!(server.finish().await, 4);
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(
        human.contains(": error [r02] Keep this warning. #3"),
        "{human}"
    );
    assert!(human.contains("2 unsure [r02];"), "{human}");
    assert!(
        human.contains("Lint findings: 1 error(s), 0 warning(s)"),
        "{human}"
    );
    let report = read(&root.path().join("run.json"));
    assert_eq!(report["status"]["kind"], "findings");
    assert_eq!(report["uncertain"].as_array().unwrap().len(), 4);
}
