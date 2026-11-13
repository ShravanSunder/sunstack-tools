use super::*;

#[tokio::test]
async fn skipped_rules_and_undefined_fields_explain_incomplete_evidence_and_preparation() {
    for cause in ["undefined_field", "skipped_rule"] {
        for prepare in [false, true] {
            let root = tempfile::tempdir().unwrap();
            fixture::write(root.path());
            let mut entries: Value =
                serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap())
                    .unwrap();
            let expected = if cause == "undefined_field" {
                entries["K001"]["decision"]["steps"]["starts"]["mechanical_check"]["field"] =
                    json!("missing_field");
                "undefined or incompatible field"
            } else {
                entries["K002"] = entries["K001"].clone();
                entries["K002"]["name"] = json!("skipped_standard");
                entries["K002"]["evidence"] = json!("missing_kind");
                std::fs::write(root.path().join("rulebook.md"), "Description starts correctly. <!-- K001 -->\n\nSkipped standard. <!-- K002 -->\n").unwrap();
                "undefined evidence kind"
            };
            fixture::entry_file(root.path(), &entries);
            let saved = root.path().join("prepared.json");
            let args = if prepare {
                vec![
                    "--all",
                    "--prepare",
                    saved.to_str().unwrap(),
                    "--format",
                    "json",
                ]
            } else {
                vec!["--all", "--evidence", "skill"]
            };

            let output = invoke(root.path(), &args, 3).await;

            let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(stdout["usage"]["model_requests"], 0);
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains("rules.json:"), "{stderr}");
            assert!(stderr.contains(expected), "{stderr}");
            if prepare {
                let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
                assert_eq!(stdout, report);
                assert_eq!(report["status"]["kind"], "incomplete");
                if cause == "skipped_rule" {
                    assert_eq!(report["skipped_rules"][0]["key"], "K002");
                }
            } else {
                assert_eq!(stdout["pieces"].as_array().unwrap().len(), 1);
            }
        }
    }
}

#[tokio::test]
async fn successful_evidence_and_preparation_keep_stderr_empty() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let output = invoke(root.path(), &["--all", "--evidence", "skill"], 0).await;
    assert!(output.stderr.is_empty());
    std::fs::write(
        root.path().join("docs/one.md"),
        "---\ndescription: Use when checking documents\n---\n",
    )
    .unwrap();
    let output = invoke(
        root.path(),
        &["--all", "--prepare", "prepared.json", "--format", "json"],
        6,
    )
    .await;
    assert!(output.stderr.is_empty());
}
