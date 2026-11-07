use super::*;

#[tokio::test]
async fn cases_use_historical_source_and_account_for_wrong_stale_and_missing_rows() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let historic = "Historical long source.\n";
    std::fs::write(root.path().join("docs/one.md"), historic).unwrap();
    std::fs::write(root.path().join("outside.md"), historic).unwrap();
    let revision = commit_tree(root.path());
    let make = |id: &str, file: &str, hash: String, expected: &str| json!({"id":id,"kind":"rule","rule":"document_standard","file":file,"revision":revision,"lines":[1,1],"text_sha256":hash,"expected":expected,"set":"development","label_from":"blind-agents"});
    let hash = sunstack_evidence::content_digest(historic.as_bytes());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[
        make("right", "docs/one.md", hash.clone(), "warn"),
        make("wrong", "docs/one.md", hash.clone(), "pass"),
        make("stale", "docs/one.md", "0".repeat(64), "pass"),
        make("missing", "outside.md", hash, "pass")
    ],"bars":[]}),
    );
    std::fs::write(root.path().join("docs/one.md"), "x").unwrap();
    let original_config = std::fs::read(root.path().join("skill-rules.config.json")).unwrap();
    let output = command(root.path())
        .args(["cases", "cases.json"])
        .output()
        .await
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["cases"].as_array().unwrap().len(), 4);
    for (index, expected) in ["right", "false_warning", "stale", "no_matching_piece"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            report["cases"][index]["outcome"]["kind"], expected,
            "{report}"
        );
    }
    assert_eq!(report["counts"]["right"], 1);
    assert_eq!(
        report["counts"]
            .as_object()
            .unwrap()
            .values()
            .map(|n| n.as_u64().unwrap())
            .sum::<u64>(),
        4
    );
    assert_eq!(
        std::fs::read(root.path().join("docs/one.md")).unwrap(),
        b"x"
    );
    assert_eq!(
        std::fs::read(root.path().join("skill-rules.config.json")).unwrap(),
        original_config
    );
}

fn commit_tree(root: &Path) -> String {
    let repository = git2::Repository::init(root).unwrap();
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Case fixture", "fixture@example.invalid").unwrap();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "historical source",
            &tree,
            &[],
        )
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn nested_config_case_copy_keeps_the_same_repository_root_as_normal_lint() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "Historical source.\n";
    std::fs::write(root.path().join("docs/one.md"), source).unwrap();
    std::fs::create_dir(root.path().join("settings")).unwrap();
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["rulebook"] = json!([{"markdown":"../rulebook.md","rules":"../rules.json"}]);
    config["evidence"]["ast_grep"] = json!("../evidence/sgconfig.yml");
    save(&root.path().join("settings/lint.json"), &config);
    let revision = commit_tree(root.path());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[{"id":"nested","kind":"rule","rule":"document_standard","file":"docs/one.md","revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(source.as_bytes()),"expected":"warn","set":"development","label_from":"blind-agents"}],"bars":[]}),
    );
    let ordinary = support::isolated_command(root.path())
        .args([
            "lint",
            "--config",
            "settings/lint.json",
            "--all",
            "--format",
            "json",
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(ordinary.status.code(), Some(1));
    let ordinary: Value = serde_json::from_slice(&ordinary.stdout).unwrap();
    assert_eq!(ordinary["results"][0]["file"], "docs/one.md");
    let cases = support::isolated_command(root.path())
        .args([
            "lint",
            "--config",
            "settings/lint.json",
            "cases",
            "cases.json",
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(
        cases.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&cases.stdout),
        String::from_utf8_lossy(&cases.stderr)
    );
    let cases: Value = serde_json::from_slice(&cases.stdout).unwrap();
    assert_eq!(cases["cases"][0]["outcome"]["kind"], "right");
}

#[tokio::test]
async fn skipped_case_matching_keeps_unmatched_and_ambiguous_lines_separate() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let source = "First paragraph.\n\nSecond paragraph.\n\n# Unmatched heading\n";
    std::fs::write(root.path().join("docs/one.md"), source).unwrap();
    std::fs::write(
        root.path().join("docs/about.md"),
        "---\ndescription: Folder purpose\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("evidence/rules/pieces.yml"),
        "id: document\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["check"]["files"] = json!(["docs/one.md"]);
    config["evidence"]["kinds"]["description"] = json!({"inline_rule":"{language: Markdown, rule: {matches: frontmatter}}","fields":{"description":{"from":"yaml","key":"description"}}});
    config["filters"] = json!({"purpose":{"per":"{name}","reads":{"file":"about.md","kind":"description","fields":["description"]},"labels":["eligible","blocked"],"questions":{"covers":{"type":"noul","instructions":{"check":"description"}}},"decision":{"start":"purpose","steps":{"purpose":{"ask":"covers","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"label":"eligible"},"no":{"label":"blocked"},"uncertain":"unsure"}}}}});
    save(&path, &config);
    let mut entries = read(&root.path().join("rules.json"));
    entries["K001"]["only_when"] = json!({"purpose":["eligible"]});
    save(&root.path().join("rules.json"), &entries);
    let revision = commit_tree(root.path());
    let cases = [("matched",[1,1]),("unmatched",[5,5]),("ambiguous",[1,3])].map(|(id,lines)| {
        let text = source.split_inclusive('\n').skip(lines[0]-1).take(lines[1]-lines[0]+1).collect::<String>();
        json!({"id":id,"kind":"rule","rule":"document_standard","file":"docs/one.md","revision":revision,"lines":lines,"text_sha256":sunstack_evidence::content_digest(text.as_bytes()),"expected":"pass","set":"development","label_from":"blind-agents"})
    });
    save(
        &root.path().join("cases.json"),
        &json!({"cases":cases,"bars":[]}),
    );
    let (report, bodies) = judged_with_args(root.path(), 0.7, 1, &["cases", "cases.json"]).await;
    assert_eq!(bodies.len(), 1);
    let bars = report["cases"][0]["bars"].as_array().unwrap();
    assert_eq!(
        bars.len(),
        2,
        "a filter skip must retain both compared bounds"
    );
    for (bar, (bound, value)) in bars
        .iter()
        .zip([("yes_at_or_above", 0.7), ("no_at_or_below", 0.3)])
    {
        assert_eq!(bar["step"], "purpose");
        assert_eq!(bar["bound"], bound);
        assert_eq!(bar["bar"], value);
        assert!((bar["observed"].as_f64().unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(bar["crossed"], false);
    }
    assert_eq!(report["cases"][1]["bars"], json!([]));
    assert_eq!(report["cases"][2]["bars"], json!([]));
    for (index, expected) in [
        "unsure",
        "no_matching_piece",
        "more_than_one_matching_piece",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(report["cases"][index]["outcome"]["kind"], expected);
    }
}

#[tokio::test]
async fn selected_case_scope_preserves_the_same_two_request_budget_as_named_lint() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"covers","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":{"finding":"Read the instruction."},"uncertain":"uncertain"}}}),
    );
    let selected = "SELECTED_CASE_BODY_SENTINEL.\n";
    for (folder, description, body) in [
        (
            "a",
            "UNRELATED_FOLDER_SENTINEL",
            "UNRELATED_BODY_SENTINEL.\n",
        ),
        ("b", "SELECTED_FOLDER_SENTINEL", selected),
    ] {
        std::fs::create_dir_all(root.path().join("packages").join(folder)).unwrap();
        std::fs::write(
            root.path().join("packages").join(folder).join("about.md"),
            format!("---\ndescription: {description}\n---\n"),
        )
        .unwrap();
        std::fs::write(
            root.path().join("packages").join(folder).join("member.md"),
            body,
        )
        .unwrap();
    }
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["check"]["files"] = json!(["packages/*/member.md"]);
    config["limits"] = json!({"max_model_requests":2});
    config["decision_model"]["endpoint"] = json!("http://127.0.0.1:9/v1/systemone");
    config["evidence"]["kinds"]["folder"] =
        json!({"rule":"document","fields":{"description":{"from":"yaml","key":"description"}}});
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"folder","fields":["description"]},"labels":["eligible","blocked"],"questions":{"folder":{"type":"noul","instructions":{"check":"description"}}},"decision":{"start":"purpose","steps":{"purpose":{"ask":"folder","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"label":"eligible"},"no":{"label":"blocked"},"uncertain":"unsure"}}}}});
    save(&path, &config);
    let mut entries = read(&root.path().join("rules.json"));
    entries["K001"]["questions"] =
        json!({"covers":{"type":"noul","instructions":{"check":"body"}}});
    entries["K001"]["only_when"] = json!({"purpose":["eligible"]});
    save(&root.path().join("rules.json"), &entries);
    let revision = commit_tree(root.path());
    let rule_case = |id: &str, set: &str| json!({"id":id,"kind":"rule","rule":"document_standard","file":"packages/b/member.md","revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(selected.as_bytes()),"expected":"pass","set":set,"label_from":"blind-agents"});
    let filter_case = json!({"id":"filter-development","kind":"filter","filter":"purpose","folder":"packages/b","revision":revision,"reads_sha256":sunstack_evidence::content_digest(&serde_json::to_vec(&json!({"description":"SELECTED_FOLDER_SENTINEL"})).unwrap()),"expected_label":"eligible","set":"development","label_from":"blind-agents"});
    let bars=[("document_standard","quality","rule-development"),("purpose","purpose","filter-development")].into_iter().flat_map(|(owner,step,id)| [("yes_at_or_above",0.7),("no_at_or_below",0.3)].map(|(bound,value)|json!({"for":owner,"step":step,"bound":bound,"value":value,"set_from":[id]}))).collect::<Vec<_>>();
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[rule_case("rule-development","development"),filter_case,rule_case("selected","held-out")],"bars":bars}),
    );
    let (direct, direct_requests) =
        judged_with_args(root.path(), 0.94, 0, &["packages/b/member.md"]).await;
    assert_eq!(direct_requests.len(), 2);
    assert_eq!(direct["results"][0]["outcome"], "pass");
    let (cases, case_requests) = judged_with_args(
        root.path(),
        0.94,
        0,
        &["cases", "cases.json", "--set", "held-out"],
    )
    .await;
    assert_eq!(cases["cases"].as_array().unwrap().len(), 1);
    assert_eq!(cases["cases"][0]["outcome"]["kind"], "right");
    assert_eq!(
        case_requests, direct_requests,
        "unrelated folders must not consume the case's allowance"
    );
    assert!(
        case_requests
            .iter()
            .all(|body| !String::from_utf8_lossy(body).contains("UNRELATED_"))
    );
}
