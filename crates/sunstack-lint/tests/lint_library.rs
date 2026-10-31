#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_lint::*;
#[path = "lint_library/related_places.rs"]
mod related_places;

#[tokio::test]
async fn invalid_rule_files_keep_the_native_file_line_and_column() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    std::fs::write(
        root.path().join("rules.json"),
        "{\n  \"K001\":\n  broken\n}\n",
    )
    .unwrap();
    let report = run(fixture::request(root.path()), CancellationToken::new()).await;
    assert_eq!(report.status, LintStatus::Invalid);
    assert_eq!(
        report.diagnostics[0].file.as_ref().unwrap(),
        &root.path().canonicalize().unwrap().join("rules.json")
    );
    assert_eq!(report.diagnostics[0].line, Some(3));
    assert!(
        report.diagnostics[0]
            .column
            .is_some_and(|column| column > 0)
    );
    let github = render(&report, OutputFormat::Github);
    assert!(github.contains("rules.json,line=3,col="));
    let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
    let location = &sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"][0]["locations"]
        [0]["physicalLocation"];
    assert!(
        location["artifactLocation"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/rules.json")
    );
    assert_eq!(location["region"]["startLine"], 3);
}

#[tokio::test]
async fn real_engine_mechanical_failures_are_findings_and_ignored_targets_select_nothing() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    let report = run(fixture::request(root.path()), CancellationToken::new()).await;
    assert_eq!(report.status, LintStatus::Findings, "{report:?}");
    assert_eq!(report.status.exit_code(), 1);
    assert_eq!(report.counts.error, 1);
    assert_eq!(report.findings[0].rule.as_str(), "description");
    assert_eq!(report.findings[0].file, std::path::Path::new("docs/one.md"));
    assert_eq!(report.findings[0].span.as_ref().unwrap().start_line, 2);
    assert_eq!(report.usage.model_requests, 0);
    let json: Value = serde_json::from_str(&render(&report, OutputFormat::Json)).unwrap();
    assert_eq!(json["status"], json!({"kind":"findings"}));
    assert_eq!(json["findings"][0]["rule"], "description");
    for format in [
        OutputFormat::Default,
        OutputFormat::Github,
        OutputFormat::Sarif,
    ] {
        let output = render(&report, format);
        assert!(output.contains("description"));
        assert!(output.contains("docs/one.md"));
        assert!(output.contains("Description must begin with Use when."));
    }
    let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    let result = &sarif["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "description");
    assert_eq!(result["level"], "error");
    assert_eq!(
        result["locations"][0]["physicalLocation"]["region"]["startLine"],
        2
    );
    config["check"]["ignore"] = json!(["docs/**"]);
    fixture::config_file(root.path(), &config);
    let ignored = run(fixture::request(root.path()), CancellationToken::new()).await;
    assert_eq!(ignored.status, LintStatus::NothingSelected);
    assert_eq!(ignored.exclusions.ignored, 1);
}

#[tokio::test]
async fn git_ignored_and_explicitly_ignored_files_never_become_targets() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    let repository = git2::Repository::init(root.path()).unwrap();
    std::fs::write(root.path().join(".gitignore"), "docs/git-ignored.md\n").unwrap();
    for name in ["git-ignored.md", "explicitly-ignored.md"] {
        std::fs::write(
            root.path().join("docs").join(name),
            "---\ndescription: Wrong\n---\n",
        )
        .unwrap();
    }
    config["check"]["ignore"] = json!(["docs/explicitly-ignored.md"]);
    fixture::config_file(root.path(), &config);
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::All;
    let report = run(request, CancellationToken::new()).await;
    assert_eq!(report.status, LintStatus::Findings, "{report:?}");
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].file, std::path::Path::new("docs/one.md"));
    assert_eq!(report.exclusions.ignored, 2);
    assert!(repository.is_path_ignored("docs/git-ignored.md").unwrap());
}

#[tokio::test]
async fn config_invalid_codes_and_locations_reject_bad_structure_and_references() {
    for case in ["syntax", "unknown", "duplicate", "glob", "missing_project"] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        match case {
            "syntax" => {
                std::fs::write(root.path().join("sunstack.lint.json"), "{\n broken").unwrap()
            }
            "duplicate" => std::fs::write(
                root.path().join("sunstack.lint.json"),
                "{\"marker_prefix\":\"K\",\"marker_prefix\":\"L\"}",
            )
            .unwrap(),
            _ => {
                match case {
                    "unknown" => config["unexpected"] = json!(true),
                    "glob" => config["check"]["files"] = json!(["["]),
                    "missing_project" => config["evidence"]["ast_grep"] = json!("missing.yml"),
                    _ => unreachable!(),
                };
                fixture::config_file(root.path(), &config);
            }
        }
        let report = run(fixture::request(root.path()), CancellationToken::new()).await;
        assert_eq!(report.status, LintStatus::Invalid, "{case}: {report:?}");
        assert!(report.diagnostics[0].file.is_some() && report.diagnostics[0].line.is_some());
        assert_eq!(report.usage.checked_files_acquired, 0);
    }
}

#[tokio::test]
async fn validation_listing_and_init_work_without_backend_or_target_content() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    std::fs::remove_file(root.path().join("docs/one.md")).unwrap();
    let location = discover_config(root.path(), None).unwrap();
    let loaded = load_config(&location).unwrap();
    let checked = check(&loaded, CancellationToken::new()).await;
    let listings = list_rules(&loaded, &checked, Some(std::path::Path::new("docs/one.md")));
    assert_eq!(listings.len(), 1, "{checked:?}");
    assert_eq!(listings[0].severity, sunstack_rules::Severity::Error);
    assert_eq!(listings[0].marker.as_str(), "K001");
    let empty = tempfile::tempdir().unwrap();
    let skeleton = init_config(empty.path()).unwrap();
    let original = std::fs::read(&skeleton.file).unwrap();
    assert!(init_config(empty.path()).is_err());
    assert_eq!(std::fs::read(&skeleton.file).unwrap(), original);
    let loaded = load_config(&skeleton).unwrap();
    assert!(matches!(
        check(&loaded, CancellationToken::new()).await,
        CheckResult::Invalid { .. }
    ));
}

#[tokio::test]
async fn native_change_selection_keeps_named_clean_files_and_dependency_affected_targets() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    fixture::commit(root.path(), "baseline");
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    let empty = run(request.clone(), CancellationToken::new()).await;
    assert_eq!(empty.status, LintStatus::NothingSelected, "{empty:?}");
    assert!(empty.exclusions.outside_change_scope > 0);
    request.paths = vec![root.path().join("docs/one.md")];
    let named = run(request.clone(), CancellationToken::new()).await;
    assert_eq!(named.status, LintStatus::Findings, "{named:?}");
    request.paths.clear();
    let path = root.path().join("rules.json");
    let source = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        source.replace("Description must begin", "Descriptions must begin"),
    )
    .unwrap();
    let changed = run(request.clone(), CancellationToken::new()).await;
    assert_eq!(changed.status, LintStatus::Findings, "{changed:?}");
    fixture::commit(root.path(), "new rule");
    std::fs::write(root.path().join("unrelated.txt"), "unrelated").unwrap();
    let unrelated = run(request.clone(), CancellationToken::new()).await;
    assert_eq!(
        unrelated.status,
        LintStatus::NothingSelected,
        "{unrelated:?}"
    );
    let mut config: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("sunstack.lint.json")).unwrap())
            .unwrap();
    config["limits"] = json!({"max_files":100});
    fixture::config_file(root.path(), &config);
    let widened = run(request, CancellationToken::new()).await;
    assert_eq!(widened.counts.error, 1, "{widened:?}");
    assert!(matches!(
        widened.scope.widened,
        Some(WidenedReason::ConfigChanged)
    ));
}

#[tokio::test]
async fn missing_named_path_default_outside_git_and_reachable_missing_backend_are_invalid() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let mut request = fixture::request(root.path());
    request.paths = vec![root.path().join("missing.md")];
    let missing = run(request, CancellationToken::new()).await;
    assert_eq!(missing.diagnostics[0].code, LintInvalidCode::PathNotFound);
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    let no_git = run(request, CancellationToken::new()).await;
    assert_eq!(
        no_git.diagnostics[0].code,
        LintInvalidCode::NoGitForDefaultScope
    );
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    let missing = run(fixture::request(root.path()), CancellationToken::new()).await;
    assert_eq!(missing.status, LintStatus::Invalid, "{missing:?}");
    assert_eq!(
        missing.diagnostics[0].code,
        LintInvalidCode::CredentialMissing,
        "{missing:?}"
    );
    assert_eq!(missing.usage.model_requests, 0);
}

async fn run(request: LintRequest, cancellation: CancellationToken) -> LintReport {
    sunstack_lint::run(request, cancellation, None).await
}
