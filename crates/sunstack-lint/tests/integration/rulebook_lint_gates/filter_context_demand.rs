use super::*;

fn configured(root: &Path) -> Value {
    let mut config = fixture::write(root);
    std::fs::create_dir_all(root.join("packages/unused")).unwrap();
    std::fs::write(root.join("packages/unused/member.md"), "# Heading only\n").unwrap();
    config["check"]["files"] = json!(["packages/*/member.md"]);
    config["evidence"]["kinds"] = json!({
        "paragraph":{"inline_rule":"{language: Markdown, rule: {kind: paragraph}}","fields":{"body":{"from":"first_line"}}},
        "heading":{"inline_rule":"{language: Markdown, rule: {kind: atx_heading}}","fields":{"body":{"from":"first_line"}}},
        "description":{"inline_rule":"{language: Markdown, rule: {matches: frontmatter}}","fields":{"description":{"from":"yaml","key":"description"}}}
    });
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"description","field":"description"},"question":{"type":"choice","instructions":{"task":"Classify description"},"criteria":{"a":"A","b":"B"}},"min_probability":0.7}});
    config["decision_model"]["endpoint"] = json!("http://127.0.0.1:1/v1/systemone");
    fixture::entry_file(
        root,
        &json!({
            "K001":{"name":"gated","evidence":"paragraph","only_when":{"purpose":["a"]},"decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"body","text":"ok"},"yes":"pass","no":"not_applicable"}}}},
            "K002":{"name":"ungated","evidence":"heading","decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"body","text":"Heading"},"yes":"pass","no":"not_applicable"}}}}
        }),
    );
    std::fs::write(
        root.join("rulebook.md"),
        "Check paragraphs. <!-- K001 -->\n\nCheck headings. <!-- K002 -->\n",
    )
    .unwrap();
    fixture::config_file(root, &config);
    config
}

#[tokio::test]
async fn unused_context_is_not_acquired_and_missing_or_present_description_does_not_change_results()
{
    let root = tempfile::tempdir().unwrap();
    configured(root.path());
    let mut results = Vec::new();
    for present in [false, true] {
        if present {
            std::fs::write(
                root.path().join("packages/unused/about.md"),
                "---\ndescription: Unused\n---\n",
            )
            .unwrap();
        }
        let (reader, reads) = source(root.path());
        let report = run_with_source(
            fixture::request(root.path()),
            CancellationToken::new(),
            Some("SYNTHETIC".into()),
            Some(reader),
        )
        .await;
        assert_eq!(
            *reads.lock().unwrap(),
            vec![PathBuf::from("packages/unused/member.md")]
        );
        assert_eq!(report.status, LintStatus::Clean, "{report:?}");
        assert!(report.incomplete.is_empty());
        assert!(report.filter_answers.is_empty());
        assert!(report.filter_skips.is_empty());
        assert_eq!(report.usage.model_requests, 0);
        assert_eq!(report.usage.checked_files_acquired, 1);
        assert_eq!(report.results.len(), 1);
        results.push(serde_json::to_value(report.results).unwrap());
    }
    assert_eq!(results[0], results[1]);
}

#[tokio::test]
async fn demanded_missing_context_still_fails_and_is_listed() {
    let root = tempfile::tempdir().unwrap();
    configured(root.path());
    std::fs::write(
        root.path().join("packages/unused/member.md"),
        "# Heading only\n\nok paragraph.\n",
    )
    .unwrap();
    let report = run(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
    )
    .await;
    assert_eq!(report.status, LintStatus::Incomplete);
    assert_eq!(report.filter_answers.len(), 1);
    assert_eq!(report.filter_skips.len(), 1);
    assert_eq!(report.usage.model_requests, 0);
    assert_eq!(report.results.len(), 1);
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(
        value["filter_answers"][0]["reason"]["kind"],
        "reads_unavailable"
    );
}

#[tokio::test]
async fn within_and_selection_removed_gated_pieces_do_not_demand_context() {
    let root = tempfile::tempdir().unwrap();
    let mut config = configured(root.path());
    std::fs::write(
        root.path().join("packages/unused/member.md"),
        "# Heading only\n\nok paragraph.\n",
    )
    .unwrap();
    config["evidence"]["kinds"]["container"] = json!({"inline_rule":"{language: Markdown, rule: {kind: section, regex: missing_container}}","fields":{}});
    config["evidence"]["kinds"]["paragraph"]["within"] = json!(["container"]);
    fixture::config_file(root.path(), &config);
    let (reader, reads) = source(root.path());
    let report = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
        Some(reader),
    )
    .await;
    assert_eq!(report.status, LintStatus::Clean);
    assert_eq!(
        *reads.lock().unwrap(),
        vec![PathBuf::from("packages/unused/member.md")]
    );
    assert!(report.filter_answers.is_empty());

    config["evidence"]["kinds"]["paragraph"]
        .as_object_mut()
        .unwrap()
        .remove("within");
    config["evidence"]["kinds"]["paragraph"]["select"] = json!({"question":{"type":"noul","instructions":{"task":"Keep?"}},"yes_at_or_above":0.7,"no_at_or_below":0.3});
    let mut server = sunstack_rules::test_support::controlled_http::ControlledServer::start().await;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let (reader, reads) = source(root.path());
    let mut task = tokio::spawn(run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
        Some(reader),
    ));
    let pending = tokio::select! {pending=server.next()=>pending,result=&mut task=>panic!("selection not reached: {result:?}")};
    pending.noul(0.1);
    let report = task.await.unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(report.status, LintStatus::Clean);
    assert_eq!(
        *reads.lock().unwrap(),
        vec![PathBuf::from("packages/unused/member.md")]
    );
    assert!(report.filter_answers.is_empty());
    assert_eq!(report.results.len(), 1);
}

#[tokio::test]
async fn checked_files_keep_all_slots_before_context_at_the_exact_checked_count_limit() {
    let root = tempfile::tempdir().unwrap();
    let mut config = configured(root.path());
    for folder in ["unused", "second"] {
        std::fs::create_dir_all(root.path().join(format!("packages/{folder}"))).unwrap();
        std::fs::write(
            root.path().join(format!("packages/{folder}/member.md")),
            "# Heading only\n\nok paragraph.\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join(format!("packages/{folder}/about.md")),
            "---\ndescription: Context\n---\n",
        )
        .unwrap();
    }
    config["limits"] = json!({"max_files":2});
    fixture::config_file(root.path(), &config);
    let (reader, reads) = source(root.path());
    let report = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
        Some(reader),
    )
    .await;
    let observed = reads
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        observed,
        [
            PathBuf::from("packages/second/member.md"),
            PathBuf::from("packages/unused/member.md")
        ]
        .into()
    );
    assert_eq!(report.usage.checked_files_acquired, 2);
    assert_eq!(report.results.len(), 2);
    assert!(
        report
            .results
            .iter()
            .all(|row| row.name.as_str() == "ungated")
    );
    assert_eq!(report.filter_skips.len(), 2);
    assert_eq!(report.filter_answers.len(), 2);
    assert_eq!(report.status, LintStatus::Incomplete);
    assert!(report.diagnostics.iter().all(|row| {
        row.file
            .as_ref()
            .is_some_and(|file| file.ends_with("about.md"))
    }));
}
