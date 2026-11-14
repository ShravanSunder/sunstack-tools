use super::*;

#[tokio::test]
async fn mechanical_starter_runs_and_warning_summary_explains_the_exit_policy() {
    let root = tempfile::tempdir().unwrap();
    invoke(root.path(), &["--init"], 0).await;
    invoke(root.path(), &["--check"], 0).await;
    let clean = report(root.path(), &["--all"], 0).await;
    assert_eq!(clean["usage"]["model_requests"], 0);
    assert_eq!(clean["results"].as_array().unwrap().len(), 1);
    assert_eq!(clean["results"][0]["outcome"], "pass");
    std::fs::write(
        root.path().join("sunstack-starter/sample.md"),
        "TODO finish this paragraph.\n",
    )
    .unwrap();
    let output = invoke(root.path(), &["--all"], 0).await;
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("Completed with 1 warning; exit 0 because no --max-warnings was set"),
        "{text}"
    );
    let warnings = report(root.path(), &["--all"], 0).await;
    assert_eq!(warnings["status"]["kind"], "clean");
    assert_eq!(warnings["counts"]["warn"], 1);
    assert_eq!(warnings["usage"]["model_requests"], 0);
    invoke(root.path(), &["--all", "--max-warnings", "0"], 1).await;
    let explicit = invoke(root.path(), &["--all", "--max-warnings", "1"], 0).await;
    let text = String::from_utf8(explicit.stdout).unwrap();
    assert!(!text.contains("no --max-warnings was set"), "{text}");
    assert!(text.contains("within --max-warnings 1"), "{text}");
    std::fs::write(root.path().join("sunstack-starter/sample.md"), "").unwrap();
    invoke(root.path(), &["--all"], 0).await;
}

#[tokio::test]
async fn complete_readme_examples_execute_their_exact_documented_shell_blocks() {
    let readme = include_str!("../../../README.md");
    for (name, directory, expected_results, expected_passes) in [
        ("choice-pictures", "choice-example", 4, 2),
        ("requirement-coverage", "requirement-example", 3, 1),
    ] {
        let marker = format!("<!-- runnable-example: {name} -->\n````sh\n");
        let start = readme.split_once(&marker).unwrap().1;
        let script = start.split_once("\n````").unwrap().0;
        let root = tempfile::tempdir().unwrap();
        let output = tokio::time::timeout(
            Duration::from_secs(30),
            support::documented_shell_command(root.path(), script).output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(
            &std::fs::read(root.path().join(directory).join("report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            report["results"].as_array().unwrap().len(),
            expected_results
        );
        assert_eq!(
            report["results"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["outcome"] == "pass")
                .count(),
            expected_passes
        );
        assert_eq!(report["counts"]["warn"], 2);
        assert_eq!(report["counts"]["error"], 0);
        assert_eq!(report["usage"]["model_requests"], 0);
        assert_eq!(report["status"]["kind"], "clean");
        assert_eq!(report["incomplete"], json!([]));
        let expected_locations = if name == "choice-pictures" {
            vec![(3, "pass"), (11, "pass"), (13, "finding"), (15, "finding")]
        } else {
            vec![(1, "pass"), (3, "finding"), (5, "finding")]
        };
        for (row, (line, outcome)) in report["results"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected_locations)
        {
            assert_eq!(row["first_line"], line);
            assert_eq!(row["outcome"], outcome);
        }
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("Completed with 2 warnings; exit 0 because no --max-warnings was set")
        );
        let config = format!("{directory}/sunstack.lint.json");
        invoke(
            root.path(),
            &[
                "--config",
                &config,
                "--root",
                directory,
                "--all",
                "--max-warnings",
                "0",
            ],
            1,
        )
        .await;
    }
}

#[tokio::test]
async fn missing_document_routes_no_and_ordered_candidates_record_the_chosen_path() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    std::fs::write(
        root.path().join("evidence/rules/document.yml"),
        "id: markdown-document\nlanguage: Markdown\nrule: {kind: document}\n",
    )
    .unwrap();
    config["evidence"]["kinds"]["skill"]["fields"]["coverage"] =
        json!({"from":"document","path":"verification.md","rule":"markdown-document"});
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"description","evidence":"skill","severity":"warn","decision":{"start":"available","steps":{"available":{"mechanical_check":{"tool":"complete","field":"coverage"},"yes":"pass","no":"not_applicable"}}}}}),
    );
    fixture::config_file(root.path(), &config);
    let missing = report(root.path(), &["--all"], 0).await;
    assert_eq!(missing["results"][0]["outcome"], "not_applicable");
    assert_eq!(missing["incomplete"], json!([]));
    let evidence = invoke(root.path(), &["--all", "--evidence"], 0).await;
    let evidence: Value = serde_json::from_slice(&evidence.stdout).unwrap();
    assert_eq!(
        evidence["pieces"][0]["fields"]["coverage"],
        json!({"state":"missing_document","candidates":["docs/verification.md"]})
    );
    config["evidence"]["kinds"]["skill"]["fields"]["coverage"]["path"] =
        json!(["verification.md", "verification-design.md"]);
    fixture::config_file(root.path(), &config);
    std::fs::write(
        root.path().join("docs/verification-design.md"),
        "U1 covered.\n",
    )
    .unwrap();
    let fallback = report(root.path(), &["--all"], 0).await;
    assert_eq!(fallback["results"][0]["outcome"], "pass");
    let evidence = invoke(root.path(), &["--all", "--evidence"], 0).await;
    let evidence: Value = serde_json::from_slice(&evidence.stdout).unwrap();
    assert_eq!(
        evidence["pieces"][0]["fields"]["coverage"]["path"],
        "docs/verification-design.md"
    );
    std::fs::write(root.path().join("docs/verification.md"), "First wins.\n").unwrap();
    let evidence = invoke(root.path(), &["--all", "--evidence"], 0).await;
    let evidence: Value = serde_json::from_slice(&evidence.stdout).unwrap();
    assert_eq!(
        evidence["pieces"][0]["fields"]["coverage"]["path"],
        "docs/verification.md"
    );
    std::fs::write(root.path().join("docs/verification.md"), [255]).unwrap();
    let unreadable = report(root.path(), &["--all"], 3).await;
    assert_eq!(unreadable["results"][0]["outcome"], "incomplete");
}
