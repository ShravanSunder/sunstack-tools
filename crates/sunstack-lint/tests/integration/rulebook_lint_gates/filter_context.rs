use super::*;
use sunstack_rules::test_support::controlled_http::ControlledServer;

fn configured(root: &Path) -> Value {
    let mut config = fixture::write(root);
    std::fs::create_dir_all(root.join("packages/one/references")).unwrap();
    std::fs::write(
        root.join("packages/one/about.md"),
        "---\ndescription: Package purpose\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.join("packages/one/references/member.md"),
        "---\ndescription: Member text\n---\n",
    )
    .unwrap();
    config["check"]["files"] = json!(["packages/*/references/*.md"]);
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["a","b"],"questions":{"filter":{"type":"choice","instructions":{"task":"Classify","check":"description"},"criteria":{"a":"A","b":"B"}}},"decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.join("rules.json")).unwrap()).unwrap();
    entries["K001"]["only_when"] = json!({"purpose":["a"]});
    fixture::entry_file(root, &entries);
    fixture::config_file(root, &config);
    config
}

#[tokio::test]
async fn named_mechanical_gated_rule_requires_credentials_before_evidence_access() {
    let root = tempfile::tempdir().unwrap();
    configured(root.path());
    let (reader, reads) = source(root.path());
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    request.paths = vec![root.path().join("packages/one/references/member.md")];
    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;
    assert_eq!(report.status, LintStatus::Invalid);
    assert_eq!(
        report.diagnostics[0].code,
        LintInvalidCode::CredentialMissing
    );
    assert!(reads.lock().unwrap().is_empty());
    assert!(!root.path().join(".git").exists());
    assert_eq!(report.usage.checked_files_acquired, 0);
}

#[tokio::test]
async fn reference_only_run_loads_context_through_the_checked_source_without_context_results() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = configured(root.path());
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let (reader, reads) = source(root.path());
    let mut request = fixture::request(root.path());
    request.paths = vec![root.path().join("packages/one/references/member.md")];
    let mut task = tokio::spawn(run_with_source(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
        Some(reader),
    ));
    let pending = tokio::select! {pending=server.next()=>pending,result=&mut task=>panic!("no filter question: {:?}",result.unwrap())};
    assert_eq!(pending.body["state"]["description"], "Package purpose");
    pending.respond(200,json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":"a","confidence":0.9,"probabilities":{"a":0.9,"b":0.1}}},"usage":{"input_tokens":1,"output_tokens":1}}));
    let report = task.await.unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(
        report
            .file_counts
            .iter()
            .map(|count| count.file.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new("packages/one/references/member.md")]
    );
    assert_eq!(
        report.exclusions.outside_change_scope, 0,
        "context is not an unchecked rule pair: {report:?}"
    );
    assert!(
        reads
            .lock()
            .unwrap()
            .contains(&PathBuf::from("packages/one/about.md")),
        "{:?}",
        reads.lock().unwrap()
    );
    assert!(
        report
            .results
            .iter()
            .all(|result| result.file == Path::new("packages/one/references/member.md"))
    );
}

#[tokio::test]
async fn description_change_reaches_selection_and_final_picking_for_the_member() {
    let root = tempfile::tempdir().unwrap();
    let mut config = configured(root.path());
    std::fs::write(
        root.path().join("evidence/rules/item.yml"),
        "id: item\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("packages/one/references/member.md"),
        "Member evidence.\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.path().join("packages/other/references")).unwrap();
    std::fs::write(
        root.path().join("packages/other/about.md"),
        "---\ndescription: Unchanged sibling\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("packages/other/references/member.md"),
        "Unchanged sibling member.\n",
    )
    .unwrap();
    config["evidence"]["kinds"]["member"] = json!({"rule":"item","fields":{"description":{"from":"first_line"}},"select":{"question":{"type":"noul","instructions":{"task":"Keep this candidate?"}},"yes_at_or_above":0.7,"no_at_or_below":0.3}});
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K001"]["evidence"] = json!("member");
    fixture::entry_file(root.path(), &entries);
    let mut server = ControlledServer::start().await;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    fixture::commit(root.path(), "context baseline");
    std::fs::write(
        root.path().join("packages/one/about.md"),
        "---\ndescription: Changed purpose\n---\n",
    )
    .unwrap();
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    let mut task = tokio::spawn(sunstack_lint::run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
    ));
    let pending = tokio::select! {pending=server.next()=>pending,result=&mut task=>panic!("ended before member selection: {:?}",result.unwrap())};
    assert!(
        pending.body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .all(|name| name.starts_with('c'))
    );
    pending.noul(0.9);
    let pending = server.next().await;
    assert_eq!(pending.body["state"]["description"], "Changed purpose");
    pending.respond(200,json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":"a","confidence":0.9,"probabilities":{"a":0.9,"b":0.1}}},"usage":{"input_tokens":1,"output_tokens":1}}));
    let report = task.await.unwrap();
    assert_eq!(server.finish().await, 2);
    assert_eq!(report.results.len(), 1, "{report:?}");
    assert_eq!(
        report.exclusions.zero_matches, 1,
        "only the unselected checked sibling contributes, not either context file"
    );
    assert_eq!(
        report.results[0].file,
        Path::new("packages/one/references/member.md")
    );
}

#[tokio::test]
async fn explicitly_checked_context_keeps_public_counts_and_rule_results() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = configured(root.path());
    config["check"]["files"] = json!(["packages/*/references/*.md", "packages/*/about.md"]);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let (reader, reads) = source(root.path());
    let mut task = tokio::spawn(run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
        Some(reader),
    ));
    let pending = tokio::select! {pending=server.next()=>pending,result=&mut task=>panic!("no filter question: {result:?}")};
    pending.respond(200,json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":"a","confidence":0.9,"probabilities":{"a":0.9,"b":0.1}}},"usage":{"input_tokens":1,"output_tokens":1}}));
    let report = task.await.unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(
        report
            .file_counts
            .iter()
            .map(|count| count.file.as_path())
            .collect::<Vec<_>>(),
        vec![
            Path::new("packages/one/about.md"),
            Path::new("packages/one/references/member.md")
        ]
    );
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.usage.checked_files_acquired, 2);
    assert_eq!(reads.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn unsure_filter_and_ungated_error_keep_findings_status_and_both_results() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = configured(root.path());
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K002"] = entries["K001"].clone();
    entries["K002"].as_object_mut().unwrap().remove("only_when");
    entries["K002"]["name"] = json!("independent_error");
    fixture::entry_file(root.path(), &entries);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check description. <!-- K001 -->\n\nCheck independent description. <!-- K002 -->\n",
    )
    .unwrap();
    let mut task = tokio::spawn(run(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
    ));
    let pending = tokio::select! {pending=server.next()=>pending, result=&mut task=>panic!("filter not asked: {:?}", result.unwrap())};
    pending.respond(200, json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":"a","confidence":0.6,"probabilities":{"a":0.6,"b":0.4}}},"usage":{"input_tokens":1,"output_tokens":1}}));
    let report = task.await.unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(report.status, LintStatus::Findings);
    assert_eq!(report.status.exit_code(), 1);
    assert_eq!(report.filter_skips.len(), 1);
    assert!(matches!(
        report.filter_answers[0].state,
        sunstack_rules::FilterAnswerState::Unsure { .. }
    ));
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule.as_str(), "independent_error");
    assert_eq!(report.counts.error, 1);
    assert!(report.incomplete.is_empty());
}
