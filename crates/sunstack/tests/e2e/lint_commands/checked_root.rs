use super::*;

fn separate_checked_root(config_root: &Path, checked_root: &Path) {
    let mut config = fixture::write(config_root);
    config["check"]["ignore"] = json!(["docs/ignored.md"]);
    fixture::config_file(config_root, &config);
    std::fs::create_dir_all(checked_root.join("docs")).unwrap();
    for name in ["checked.md", "ignored.md"] {
        std::fs::write(
            checked_root.join("docs").join(name),
            "---\ndescription: Wrong checked opening\n---\n",
        )
        .unwrap();
    }
}

#[tokio::test]
async fn root_flag_checks_another_directory_and_keeps_config_inputs_relative() {
    let config = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    separate_checked_root(config.path(), checked.path());

    let result = report(
        config.path(),
        &["--all", "--root", checked.path().to_str().unwrap()],
        1,
    )
    .await;

    assert_eq!(result["findings"].as_array().unwrap().len(), 1);
    assert_eq!(result["findings"][0]["file"], "docs/checked.md");
    assert_eq!(result["exclusions"]["ignored"], 1);
    assert_eq!(result["usage"]["model_requests"], 0);
    assert_eq!(
        result["findings"][0]["related"][0]["file"],
        json!(config.path().canonicalize().unwrap().join("rulebook.md"))
    );
}

#[tokio::test]
async fn serialized_library_request_honors_the_checked_root() {
    let config = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    separate_checked_root(config.path(), checked.path());
    let mut request = serde_json::to_value(fixture::request(config.path())).unwrap();
    request["root"] = json!(checked.path());
    let request = serde_json::from_value(request).unwrap();

    let result = sunstack_lint::run(request, CancellationToken::new(), None).await;

    assert_eq!(result.status, sunstack_lint::LintStatus::Findings);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].file, Path::new("docs/checked.md"));
    assert_eq!(result.exclusions.ignored, 1);
}

#[tokio::test]
async fn named_path_outside_checked_root_is_invalid_and_names_both_paths() {
    let config = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fixture::write(config.path());
    let outside_file = outside.path().join("outside.md");
    std::fs::write(&outside_file, "outside").unwrap();

    let result = report(config.path(), &[outside_file.to_str().unwrap()], 2).await;

    assert_eq!(result["status"]["kind"], "invalid");
    let diagnostics = result["diagnostics"].to_string();
    assert!(diagnostics.contains("outside root"), "{diagnostics}");
    assert!(
        diagnostics.contains(outside_file.canonicalize().unwrap().to_str().unwrap()),
        "{diagnostics}"
    );
    assert!(
        diagnostics.contains(config.path().canonicalize().unwrap().to_str().unwrap()),
        "{diagnostics}"
    );
    assert_eq!(result["usage"]["checked_files_acquired"], 0);

    let missing_outside = outside.path().canonicalize().unwrap().join("missing.md");
    let missing = report(config.path(), &[missing_outside.to_str().unwrap()], 2).await;

    assert_eq!(missing["status"]["kind"], "invalid");
    assert_eq!(missing["diagnostics"][0]["code"], "path_not_found");
    let message = missing["diagnostics"][0]["message"].as_str().unwrap();
    assert!(
        message.contains(missing_outside.to_str().unwrap()),
        "{message}"
    );
    assert!(
        message.contains(config.path().canonicalize().unwrap().to_str().unwrap()),
        "{message}"
    );
    assert_eq!(missing["usage"]["checked_files_acquired"], 0);
}

#[tokio::test]
async fn named_empty_selection_is_incomplete_while_unnamed_empty_stays_nothing_selected() {
    let config = tempfile::tempdir().unwrap();
    let mut configuration = fixture::write(config.path());
    configuration["check"]["files"] = json!(["absent/*.md"]);
    fixture::config_file(config.path(), &configuration);
    std::fs::write(config.path().join("unmatched.txt"), "not checked").unwrap();
    let unnamed = report(config.path(), &["--all"], 0).await;
    assert_eq!(unnamed["status"]["kind"], "nothing_selected");

    let named = report(config.path(), &["unmatched.txt"], 3).await;

    assert_eq!(named["status"]["kind"], "incomplete");
    assert!(!named["incomplete"].as_array().unwrap().is_empty());
    assert!(
        named["diagnostics"]
            .to_string()
            .contains("nothing selected"),
        "{named}"
    );
    assert_eq!(named["usage"]["checked_files_acquired"], 0);
}

#[tokio::test]
async fn root_since_captures_the_checked_repository_and_reports_root_relative_paths() {
    let config = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    separate_checked_root(config.path(), checked.path());
    fixture::commit(checked.path(), "initial checked repository");
    std::fs::write(
        checked.path().join("docs/checked.md"),
        "---\ndescription: Wrong changed opening\n---\n",
    )
    .unwrap();
    fixture::commit(checked.path(), "changed checked document");

    let result = report(
        config.path(),
        &[
            "--root",
            checked.path().to_str().unwrap(),
            "--since",
            "HEAD~1",
        ],
        1,
    )
    .await;

    assert_eq!(result["scope"]["kind"], "since");
    assert_eq!(result["findings"].as_array().unwrap().len(), 1);
    assert_eq!(result["findings"][0]["file"], "docs/checked.md");
}

#[tokio::test]
async fn root_subdirectory_supports_all_but_refuses_git_change_scope() {
    let config = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    let checked = repository.path().join("project");
    separate_checked_root(config.path(), &checked);
    std::fs::write(
        repository.path().join(".gitignore"),
        "project/docs/ignored.md\n",
    )
    .unwrap();
    fixture::commit(repository.path(), "initial repository");
    let relative_root =
        sunstack_lint::lint_execution::targets::relative_path(config.path(), &checked);
    let all = report(
        config.path(),
        &["--root", relative_root.to_str().unwrap(), "--all"],
        1,
    )
    .await;
    assert_eq!(all["findings"][0]["file"], "docs/checked.md");
    assert_eq!(all["exclusions"]["ignored"], 1);

    let refused = report(
        config.path(),
        &["--root", checked.to_str().unwrap(), "--since", "HEAD"],
        2,
    )
    .await;

    assert_eq!(refused["status"]["kind"], "invalid");
    assert!(
        refused["diagnostics"]
            .to_string()
            .contains("requires the repository root"),
        "{refused}"
    );
    assert_eq!(refused["usage"]["checked_files_acquired"], 0);
}

#[tokio::test]
async fn root_is_refused_for_cases_before_reading_a_case_file() {
    let root = tempfile::tempdir().unwrap();

    let refused = invoke(
        root.path(),
        &["--root", ".", "cases", "missing-cases.json"],
        2,
    )
    .await;

    let diagnostic = String::from_utf8(refused.stdout).unwrap();
    assert!(
        diagnostic.contains("historical snapshot root"),
        "{diagnostic}"
    );
}
