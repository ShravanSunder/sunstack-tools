use super::*;

#[tokio::test]
async fn cases_report_saves_complete_stdout_and_keeps_exit_and_show_readback() {
    for scenario in ["pass", "wrong", "stale", "missing", "incomplete"] {
        // Arrange: all outcomes use the existing permanent mechanical fixture.
        let root = limit_fixture(
            json!({"body":{"from":"remainder"}}),
            mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
        );
        let historic = "Historical long source.\n";
        std::fs::write(root.path().join("docs/one.md"), historic).unwrap();
        std::fs::write(root.path().join("outside.md"), historic).unwrap();
        if scenario == "incomplete" {
            let path = root.path().join("skill-rules.config.json");
            let mut config = read(&path);
            config["limits"]["max_file_bytes"] = json!(1);
            save(&path, &config);
        }
        let revision = super::case_runs::commit_tree(root.path());
        let case = json!({
            "id":"sample","kind":"rule","rule":"document_standard",
            "file":if scenario == "missing" {"outside.md"} else {"docs/one.md"},
            "revision":revision,"lines":[1,1],
            "text_sha256":if scenario == "stale" {"0".repeat(64)} else {sunstack_evidence::content_digest(historic.as_bytes())},
            "expected":if scenario == "wrong" {"pass"} else {"warn"},
            "set":"development","label_from":"blind-agents"
        });
        let mut not_selected = case.clone();
        not_selected["id"] = json!("held_out");
        not_selected["set"] = json!("other");
        save(
            &root.path().join("cases.json"),
            &json!({"cases":[case,not_selected],"bars":[]}),
        );

        // Act: the parent --report flag must persist the same complete native result.
        let result = command(root.path())
            .args([
                "--report",
                "saved.json",
                "cases",
                "cases.json",
                "--set",
                "development",
            ])
            .output()
            .await
            .unwrap();

        // Assert: saved evidence survives the disposable historical snapshot.
        let expected_exit = if scenario == "pass" { 0 } else { 1 };
        assert_eq!(
            result.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(read(&root.path().join("saved.json")), stdout);
        assert_eq!(stdout["not_selected"], json!(["held_out"]));
        for run in stdout["runs"].as_array().unwrap() {
            assert_eq!(run["report"]["usage"]["model_requests"], 0);
        }
        let show = command(root.path())
            .args(["show", "saved.json", "sample"])
            .output()
            .await
            .unwrap();
        assert!(
            show.status.success(),
            "{}",
            String::from_utf8_lossy(&show.stderr)
        );
        let detail: Value = serde_json::from_slice(&show.stdout).unwrap();
        assert_eq!(detail["case"], stdout["cases"][0]);
    }
}

#[tokio::test]
async fn cases_report_save_failure_uses_ordinary_report_failure_exit() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let historic = "Historical source.\n";
    std::fs::write(root.path().join("docs/one.md"), historic).unwrap();
    let revision = super::case_runs::commit_tree(root.path());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[{
        "id":"sample","kind":"rule","rule":"document_standard","file":"docs/one.md",
        "revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(historic.as_bytes()),
        "expected":"warn","set":"development","label_from":"blind-agents"
    }],"bars":[]}),
    );

    let result = command(root.path())
        .args(["--report", "missing/saved.json", "cases", "cases.json"])
        .output()
        .await
        .unwrap();

    assert_eq!(
        result.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("cannot save report"));
    assert!(!root.path().join("missing/saved.json").exists());
}
