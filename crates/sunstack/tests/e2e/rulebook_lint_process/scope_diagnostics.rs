use super::*;

fn scope_fixture(limits: Value, binary: bool) -> tempfile::TempDir {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"contains","field":"body","text":"good"})),
    );
    std::fs::write(root.path().join("docs/one.md"), "good\n").unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["limits"] = limits;
    save(&config_path, &config);
    let asset = root.path().join(if binary {
        "assets/image.png"
    } else {
        "assets/large.txt"
    });
    std::fs::write(
        &asset,
        if binary {
            b"\x89PNG\x00".to_vec()
        } else {
            vec![b'a'; 128]
        },
    )
    .unwrap();
    super::case_runs::commit_tree(root.path());
    std::fs::write(
        &asset,
        if binary {
            b"\x89PNG\x01".to_vec()
        } else {
            vec![b'b'; 128]
        },
    )
    .unwrap();
    root
}

async fn refused_scope(root: &Path, format: &str, expected: i32) -> (std::process::Output, Value) {
    // Saved output lives outside the captured repository, so it cannot change its scope.
    let output_directory = tempfile::tempdir().unwrap();
    let report_file = output_directory.path().join("report.json");
    let mut invocation = command(root);
    invocation
        .args(["--since", "HEAD", "--prepare"])
        .arg(&report_file);
    if format != "human" {
        invocation.args(["--format", format]);
    }
    let output = tokio::time::timeout(Duration::from_secs(15), invocation.output())
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
    (output, read(&report_file))
}

fn assert_scope_location(
    output: &std::process::Output,
    report: &Value,
    root: &Path,
    asset: &str,
    format: &str,
) {
    let path = root.canonicalize().unwrap().join(asset);
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["path"], json!(path), "{report}");
    assert_eq!(report["usage"]["model_requests"], 0);
    if format == "sarif" {
        let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            sarif["runs"][0]["invocations"][0]["toolExecutionNotifications"][0]["locations"][0]["physicalLocation"]
                ["artifactLocation"]["uri"],
            json!(path.to_string_lossy())
        );
    } else if format == "human" {
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(&path.display().to_string()),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    } else {
        let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(printed["diagnostics"][0]["path"], json!(path));
    }
}

#[tokio::test]
async fn changed_binary_scope_names_the_asset_in_human_json_and_sarif() {
    // Arrange: a real changed PNG is outside checked Markdown but inside Git discovery.
    let root = scope_fixture(json!({}), true);
    for format in ["json", "human", "sarif"] {
        // Act: prepare takes the real change-capture refusal route without credentials.
        let (output, report) = refused_scope(root.path(), format, 2).await;

        // Assert: retain invalid status, with the asset's structured and displayed identity.
        assert_eq!(report["status"]["kind"], "invalid");
        assert_eq!(report["diagnostics"][0]["file"], report["config"]);
        assert_eq!(report["diagnostics"][0]["line"], 1);
        assert_eq!(report["diagnostics"][0]["column"], 1);
        assert!(
            report["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains("UnsupportedEncoding")
        );
        assert_scope_location(&output, &report, root.path(), "assets/image.png", format);
    }
}

#[tokio::test]
async fn change_scope_limits_name_the_setting_value_and_stopped_path() {
    for (name, value) in [
        ("max_files", 1),
        ("max_file_bytes", 64),
        ("max_total_bytes", 64),
    ] {
        // Arrange: the alphabetically first Git file deterministically meets each cap.
        let mut limits = json!({});
        limits[name] = json!(value);
        let root = scope_fixture(limits, false);
        for format in ["json", "human", "sarif"] {
            // Act: the existing limited capture remains an incomplete result.
            let (output, report) = refused_scope(root.path(), format, 3).await;

            // Assert: each presentation carries the actual cap, value and source path.
            assert_eq!(report["status"]["kind"], "incomplete");
            assert_eq!(report["incomplete"][0]["file"], Value::Null);
            assert_eq!(report["diagnostics"][0]["file"], Value::Null);
            let named_limit = format!("{name}={value}");
            assert!(
                report["diagnostics"][0]["message"]
                    .as_str()
                    .unwrap()
                    .contains(&named_limit),
                "{report}"
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains(&named_limit));
            assert_scope_location(&output, &report, root.path(), "assets/large.txt", format);
        }
    }
}

#[tokio::test]
async fn since_prepares_one_checked_file_beside_more_context_than_the_acquisition_cap() {
    // Arrange: unchanged repository context is outside the one configured checked document.
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"contains","field":"body","text":"good"})),
    );
    std::fs::write(root.path().join("docs/one.md"), "good before\n").unwrap();
    std::fs::create_dir(root.path().join("context")).unwrap();
    for index in 0..16 {
        std::fs::write(
            root.path().join(format!("context/stable-{index}.md")),
            "stable\n",
        )
        .unwrap();
    }
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["limits"] = json!({"max_files":2});
    save(&root.path().join("skill-rules.config.json"), &config);
    super::case_runs::commit_tree(root.path());
    std::fs::write(root.path().join("docs/one.md"), "good after\n").unwrap();
    let outputs = tempfile::tempdir().unwrap();
    let saved = outputs.path().join("prepared.json");
    // Act: the actual lint scope requests changed-only capture, keyless.
    let output = command(root.path())
        .args(["--since", "HEAD", "--prepare"])
        .arg(&saved)
        .args(["--format", "json"])
        .output()
        .await
        .unwrap();
    // Assert: the existing prepared-success code and complete selected result survive the cap.
    assert_eq!(
        output.status.code(),
        Some(6),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read(&saved);
    assert_eq!(report["status"]["kind"], "prepared");
    assert_eq!(report["results"].as_array().unwrap().len(), 1);
    assert_eq!(report["results"][0]["file"], "docs/one.md");
    assert_eq!(report["usage"]["model_requests"], 0);
}
