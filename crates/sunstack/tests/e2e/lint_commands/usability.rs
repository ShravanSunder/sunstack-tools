use super::*;

#[tokio::test]
async fn clean_asking_cli_scopes_finish_keyless_and_named_files_still_refuse() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    fixture::commit(root.path(), "clean asking project");
    for arguments in [vec![], vec!["--since", "HEAD"]] {
        let value = report(root.path(), &arguments, 0).await;
        assert_eq!(value["status"]["kind"], "nothing_selected");
        assert_eq!(value["usage"]["model_requests"], 0);
        assert_eq!(value["usage"]["checked_files_acquired"], 0);
    }
    let named = report(root.path(), &["docs/one.md"], 2).await;
    assert_eq!(named["diagnostics"][0]["code"], "credential_missing");
    assert_eq!(named["scope"]["kind"], "all");
    assert_eq!(named["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn prepared_cli_summary_names_the_actual_saved_requests_and_spaced_path() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    for (second_file, count, noun) in [(false, 1, "request"), (true, 2, "requests")] {
        if second_file {
            std::fs::write(
                root.path().join("docs/two.md"),
                "---\ndescription: Wrong second\n---\n",
            )
            .unwrap();
        }
        let destination = root.path().join("saved requests.json");

        let output = invoke(
            root.path(),
            &["--all", "--prepare", destination.to_str().unwrap()],
            6,
        )
        .await;

        let saved: Value = serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
        assert_eq!(saved["requests"].as_array().unwrap().len(), count);
        assert_eq!(saved["usage"]["model_requests"], 0);
        let human = String::from_utf8(output.stdout).unwrap();
        assert!(
            human.contains(&format!(
                "Prepared {count} {noun} in {}: 0 error(s), 0 warning(s)",
                destination.display()
            )),
            "{human}"
        );
    }
}

#[tokio::test]
async fn clean_preparation_saves_zero_requests_and_reports_the_destination() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    fixture::commit(root.path(), "clean asking project");
    // Keep the destination outside the captured tree and alive through readback.
    let output_directory = tempfile::tempdir().unwrap();
    let destination = output_directory.path().join("prepared.json");

    let output = invoke(
        root.path(),
        &[
            "--since",
            "HEAD",
            "--prepare",
            destination.to_str().unwrap(),
        ],
        6,
    )
    .await;

    let saved: Value = serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
    assert!(saved["requests"].as_array().unwrap().is_empty());
    assert!(String::from_utf8(output.stdout).unwrap().contains(&format!(
        "Prepared 0 requests in {}:",
        destination.display()
    )));
}

#[tokio::test]
async fn lint_help_documents_config_discovery_and_all_status_exit_codes() {
    let root = tempfile::tempdir().unwrap();

    let output = invoke(root.path(), &["--help"], 0).await;

    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("nearest sunstack.lint.json upward"), "{help}");
    for line in [
        "0  Clean or nothing selected",
        "1  Findings",
        "2  Invalid input",
        "3  Incomplete",
        "4  Cancelled",
        "5  Uncertain",
        "6  Prepared",
    ] {
        assert!(help.contains(line), "missing {line}: {help}");
    }
}
