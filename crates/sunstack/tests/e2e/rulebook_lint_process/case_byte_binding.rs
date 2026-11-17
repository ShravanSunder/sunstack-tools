use super::*;

#[tokio::test]
async fn adjacent_byte_cases_keep_distinct_saved_links_in_both_modes() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "<p>yes</p><p>no</p>";
    std::fs::write(root.path().join("docs/one.html"), source).unwrap();
    std::fs::write(
        root.path().join("evidence/rules/pieces.yml"),
        "id: document\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n",
    )
    .unwrap();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["docs/*.html"]);
    save(&config_path, &config);
    let revision = super::case_runs::commit_tree(root.path());
    let cases = [("first", [0, 10]), ("second", [10, 19])].map(|(id, bytes)| {
        json!({
            "id":id,"kind":"rule","rule":"document_standard","file":"docs/one.html",
            "revision":revision,"lines":[1,1],"bytes":bytes,
            "text_sha256":sunstack_evidence::content_digest(&source.as_bytes()[bytes[0]..bytes[1]]),
            "expected":"warn","set":"development","label_from":"blind-agents"
        })
    });
    save(&root.path().join("cases.json"), &json!({"cases":cases}));

    for (args, saved, expected_exit, expected_outcome) in [
        (
            vec!["--report", "scored.json", "cases", "cases.json"],
            "scored.json",
            0,
            "right",
        ),
        (
            vec!["cases", "cases.json", "--prepare", "prepared.json"],
            "prepared.json",
            1,
            "prepared",
        ),
    ] {
        let output = command(root.path()).args(args).output().await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report = read(&root.path().join(saved));
        assert_eq!(report["counts"][expected_outcome], 2);
        if saved == "prepared.json" {
            assert_eq!(report["status"]["kind"], "findings");
        }
        let ids = [
            report["cases"][0]["result_id"].as_u64().unwrap(),
            report["cases"][1]["result_id"].as_u64().unwrap(),
        ];
        assert_ne!(ids[0], ids[1]);
        for (index, id) in ["first", "second"].iter().enumerate() {
            assert_eq!(report["cases"][index]["outcome"]["kind"], expected_outcome);
            assert_eq!(
                report["cases"][index]["case"]["bytes"],
                json!([[0, 10], [10, 19]][index])
            );
            let shown = command(root.path())
                .args(["show", saved, id])
                .output()
                .await
                .unwrap();
            assert!(
                shown.status.success(),
                "{}",
                String::from_utf8_lossy(&shown.stderr)
            );
            let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
            assert_eq!(detail["case"]["result_id"], ids[index]);
            let span = &detail["detail"]["evidence"]["piece"]["where"]["span"];
            assert_eq!(
                [
                    span["byte_start"].as_u64().unwrap(),
                    span["byte_end"].as_u64().unwrap()
                ],
                [[0, 10], [10, 19]][index]
            );
        }
    }
}

#[tokio::test]
async fn later_invalid_range_refuses_all_dispatch_and_all_stale_has_no_child() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "Historical source.\n";
    std::fs::write(root.path().join("docs/one.md"), source).unwrap();
    let revision = super::case_runs::commit_tree(root.path());
    let make = |id, bytes, hash| {
        json!({"id":id,"kind":"rule","rule":"document_standard",
        "file":"docs/one.md","revision":revision,"lines":[1,1],"bytes":bytes,
        "text_sha256":hash,"expected":"warn","set":"development","label_from":"blind-agents"})
    };
    let good = make(
        "good",
        json!([0, 10]),
        sunstack_evidence::content_digest(&source.as_bytes()[0..10]),
    );
    let bad = make("bad", json!([0, 1000]), "unused".into());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[good,bad]}),
    );
    let output = command(root.path())
        .args(["cases", "cases.json"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside archived source"));

    let stale = make("stale", json!([0, 10]), "0".repeat(64));
    save(&root.path().join("cases.json"), &json!({"cases":[stale]}));
    let report = invoke(root.path(), &["cases", "cases.json"], 1).await;
    assert_eq!(report["counts"], json!({"stale":1}));
    assert_eq!(report["runs"], json!([]));
    assert_eq!(report["cases"][0]["result_id"], Value::Null);
}

#[cfg(unix)]
#[tokio::test]
async fn every_selected_revision_is_admitted_before_launching_any_child() {
    use std::os::unix::fs::PermissionsExt;
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "First historical source.\n";
    std::fs::write(root.path().join("docs/one.md"), "Short.\n").unwrap();
    super::case_runs::commit_tree(root.path());
    let repository = git2::Repository::open(root.path()).unwrap();
    std::fs::write(root.path().join("docs/one.md"), source).unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("docs/one.md")).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    let signature = git2::Signature::now("Case fixture", "fixture@example.invalid").unwrap();
    let _second = repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "long source",
            &tree,
            &[&parent],
        )
        .unwrap()
        .to_string();
    let case = |id, revision, bytes, digest| {
        json!({"id":id,"kind":"rule","rule":"document_standard",
        "file":"docs/one.md","revision":revision,"lines":[1,1],"bytes":bytes,
        "text_sha256":digest,"expected":"warn","set":"development","label_from":"blind-agents"})
    };
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[
            case("first", "HEAD", json!([0,10]), sunstack_evidence::content_digest(&source.as_bytes()[0..10])),
            case("second", "HEAD~1", json!([0,10]), "unused".into())
        ]}),
    );
    let observer = root.path().join("observe-child");
    let marker = root.path().join("child-started");
    std::fs::write(
        &observer,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&observer, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Witness the same executable on an admitted selection before testing silence.
    let combined = read(&root.path().join("cases.json"));
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[combined["cases"][0].clone()]}),
    );
    let _positive = sunstack_lint::case_runs::run_cases(
        &root.path().join("cases.json"),
        Some(&root.path().join("skill-rules.config.json")),
        None,
        &[],
        &observer,
        sunstack_decisions::CancellationToken::new(),
    )
    .await;
    assert!(
        marker.exists(),
        "valid selection did not reach the observed executable"
    );
    std::fs::remove_file(&marker).unwrap();
    save(&root.path().join("cases.json"), &combined);
    let result = sunstack_lint::case_runs::run_cases(
        &root.path().join("cases.json"),
        Some(&root.path().join("skill-rules.config.json")),
        None,
        &[],
        &observer,
        sunstack_decisions::CancellationToken::new(),
    )
    .await;
    assert!(result.unwrap_err().contains("outside archived source"));
    assert!(
        !marker.exists(),
        "a child ran before every selected range was admitted"
    );
}

#[tokio::test]
async fn colliding_nested_ids_read_back_their_own_archived_revision() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["docs/*.html"]);
    save(&config_path, &config);
    std::fs::write(
        root.path().join("evidence/rules/pieces.yml"),
        "id: document\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("docs/one.html"), "<p>yes</p>").unwrap();
    super::case_runs::commit_tree(root.path());
    let repository = git2::Repository::open(root.path()).unwrap();
    std::fs::write(root.path().join("docs/one.html"), "x<p>no</p>").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(Path::new("docs/one.html")).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    let signature = git2::Signature::now("Case fixture", "fixture@example.invalid").unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "second source",
            &tree,
            &[&parent],
        )
        .unwrap();
    let case = |id, revision, bytes, text: &str| {
        json!({
            "id":id,"kind":"rule","rule":"document_standard","file":"docs/one.html",
            "revision":revision,"lines":[1,1],"bytes":bytes,
            "text_sha256":sunstack_evidence::content_digest(text.as_bytes()),
            "expected":"warn","set":"development","label_from":"blind-agents"
        })
    };
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[
            case("old", "HEAD~1", [0,10], "<p>yes</p>"),
            case("new", "HEAD", [1,10], "<p>no</p>")
        ]}),
    );
    for (args, saved, expected_exit, outcome) in [
        (
            vec!["--report", "revisions-scored.json", "cases", "cases.json"],
            "revisions-scored.json",
            0,
            "right",
        ),
        (
            vec![
                "cases",
                "cases.json",
                "--prepare",
                "revisions-prepared.json",
            ],
            "revisions-prepared.json",
            1,
            "prepared",
        ),
    ] {
        let output = command(root.path()).args(args).output().await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report = read(&root.path().join(saved));
        let first_id = report["cases"][0]["result_id"].as_u64().unwrap();
        assert_eq!(report["cases"][1]["result_id"].as_u64(), Some(first_id));
        for (index, (id, expected_text, bytes)) in [
            ("old", "<p>yes</p>", [0, 10]),
            ("new", "<p>no</p>", [1, 10]),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(report["cases"][index]["outcome"]["kind"], outcome);
            let shown = command(root.path())
                .args(["show", saved, id])
                .output()
                .await
                .unwrap();
            assert!(
                shown.status.success(),
                "{}",
                String::from_utf8_lossy(&shown.stderr)
            );
            let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
            assert_eq!(
                detail["revision"],
                report["cases"][index]["case"]["revision"]
            );
            assert_eq!(detail["detail"]["evidence"]["piece"]["text"], expected_text);
            let span = &detail["detail"]["evidence"]["piece"]["where"]["span"];
            assert_eq!(
                [
                    span["byte_start"].as_u64().unwrap(),
                    span["byte_end"].as_u64().unwrap()
                ],
                bytes
            );
        }
    }
}

#[tokio::test]
async fn archived_byte_hashes_keep_crlf_and_partial_codepoints_distinct_from_matches() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "Café.\r\n";
    std::fs::write(
        root.path().join(".gitattributes"),
        "docs/one.md -text -export-subst\n",
    )
    .unwrap();
    std::fs::write(root.path().join("docs/one.md"), source.as_bytes()).unwrap();
    let revision = super::case_runs::commit_tree(root.path());
    // Changing today's file cannot change bounds or hashes from the archived revision.
    std::fs::write(root.path().join("docs/one.md"), "x").unwrap();
    let make = |id, bytes: [usize; 2], digest: String| {
        json!({
            "id":id,"kind":"rule","rule":"document_standard","file":"docs/one.md",
            "revision":revision,"lines":[1,1],"bytes":bytes,"text_sha256":digest,
            "expected":"warn","set":"development","label_from":"blind-agents"
        })
    };
    let hash = |range: std::ops::Range<usize>| {
        sunstack_evidence::content_digest(&source.as_bytes()[range])
    };
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[
            make("partial-utf8", [3,4], hash(3..4)),
            make("crlf", [0,8], hash(0..8)),
            make("one-off", [0,5], hash(0..5)),
            make("stale-no-piece", [0,5], "0".repeat(64))
        ]}),
    );
    let report = invoke(root.path(), &["cases", "cases.json"], 1).await;
    assert_eq!(report["cases"][0]["outcome"]["kind"], "no_matching_piece");
    assert_eq!(report["cases"][2]["outcome"]["kind"], "no_matching_piece");
    assert_eq!(report["cases"][3]["outcome"]["kind"], "stale");
    // A matching original range must be admitted regardless of CRLF; the engine owns
    // whether its document span includes the line ending.
    assert_ne!(report["cases"][1]["outcome"]["kind"], "stale");
    assert_ne!(report["cases"][1]["outcome"]["kind"], "incomplete");
}

#[tokio::test]
async fn malformed_unselected_byte_ranges_refuse_before_configuration_acquisition() {
    let root = tempfile::tempdir().unwrap();
    let case = |id, set, bytes: Value| {
        json!({
            "id":id,"kind":"rule","rule":"document_standard","file":"docs/one.md",
            "revision":"unavailable","lines":[1,1],"bytes":bytes,"text_sha256":"unused",
            "expected":"warn","set":set,"label_from":"blind-agents"
        })
    };
    let case_file = root.path().join("cases.json");
    let missing_config = root.path().join("missing-config.json");
    let missing_child = root.path().join("missing-child");
    let selected = case("selected", "development", json!([0, 1]));
    save(&case_file, &json!({"cases":[selected.clone()]}));
    let control = sunstack_lint::case_runs::run_cases(
        &case_file,
        Some(&missing_config),
        Some("development"),
        &[],
        &missing_child,
        sunstack_decisions::CancellationToken::new(),
    )
    .await
    .unwrap_err();

    for (name, raw_range) in [
        ("one offset", "[0]"),
        ("three offsets", "[0,1,2]"),
        ("string offset", "[0,\"1\"]"),
        ("negative offset", "[-1,1]"),
        ("overflow offset", "[0,18446744073709551616]"),
        ("empty range", "[1,1]"),
        ("reversed range", "[2,1]"),
    ] {
        let unselected = case("unselected", "held", json!("BYTE_RANGE"));
        let document = serde_json::to_string(&json!({"cases":[selected, unselected]}))
            .unwrap()
            .replace("\"BYTE_RANGE\"", raw_range);
        std::fs::write(&case_file, document).unwrap();
        let error = sunstack_lint::case_runs::run_cases(
            &case_file,
            Some(&missing_config),
            Some("development"),
            &[],
            &missing_child,
            sunstack_decisions::CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert_ne!(error, control, "{name} reached configuration acquisition");
        assert!(
            error.contains("invalid")
                || error.contains("out of range")
                || error.contains("expected"),
            "{name}: {error}"
        );
    }
}
