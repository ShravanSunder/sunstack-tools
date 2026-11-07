use super::*;

#[tokio::test]
async fn saved_report_shows_one_mechanical_result_after_rulebook_removal() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    std::fs::write(root.path().join("docs/one.md"), "Too long.\n").unwrap();
    let output = command(root.path())
        .args(["--all", "--report", "run.json"])
        .output()
        .await
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(human.contains("Standard failed. #1"), "{human}");
    let saved = read(&root.path().join("run.json"));
    assert_eq!(saved["findings"][0]["id"], 1);
    assert_eq!(saved["results"][0]["id"], 1);
    let files = saved["rulebook_files"].as_array().unwrap();
    assert!(files.iter().any(|file| {
        file["content"]
            .as_str()
            .is_some_and(|text| text.contains("Standard failed."))
    }));
    std::fs::remove_file(root.path().join("skill-rules.config.json")).unwrap();
    std::fs::remove_file(root.path().join("rules.json")).unwrap();
    let detail = support::isolated_command(root.path())
        .current_dir(root.path())
        .args(["lint", "show", "run.json", "1"])
        .output()
        .await
        .unwrap();
    assert_eq!(
        detail.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&detail.stderr)
    );
    let detail: Value = serde_json::from_slice(&detail.stdout).unwrap();
    assert_eq!(detail["result"]["id"], 1);
    assert_eq!(detail["result"]["path"], json!(["check"]));
    assert_eq!(detail["finding"]["message"], "Standard failed.");
    assert!(
        detail.get("results").is_none(),
        "show must not dump the whole report"
    );
    let missing = support::isolated_command(root.path())
        .current_dir(root.path())
        .args(["lint", "show", "run.json", "999"])
        .output()
        .await
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("unknown result id 999"));
}

#[tokio::test]
async fn reference_and_all_answers_survive_assembly_http_and_saved_report() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    let path = root.path().join("rules.json");
    let mut rules = read(&path);
    let reference = json!({"examples":{"portable":{"text":"Portable instruction.","answer":"pass","why":"Reusable practice."}}});
    rules["K001"]["reference"] = reference.clone();
    rules["K001"]["questions"] = json!({
        "quality":{"type":"noul","instructions":{"check":"body","context":["reference.examples"],"task":"Does it comply?"}},
        "unused":{"type":"noul","instructions":{"check":"body","task":"Unused on this route."}}
    });
    save(&path, &rules);
    std::fs::write(root.path().join("docs/one.md"), "Portable instruction.\n").unwrap();
    let (report, bodies) = judged_with_args(root.path(), 0.9, 0, &["--report", "run.json"]).await;
    assert_eq!(bodies.len(), 1);
    let request: Value = serde_json::from_slice(&bodies[0]).unwrap();
    assert_eq!(request["state"]["reference"], reference);
    assert_eq!(request["questions"], rules["K001"]["questions"]);
    assert!(request["state"].get("where").is_none());
    assert_eq!(
        report["results"][0]["answers"].as_object().unwrap().len(),
        1
    );
    assert_eq!(
        report["requests"][0]["request"],
        json!({"state":request["state"],"questions":request["questions"]})
    );
    assert_eq!(
        report["requests"][0]["outcome"]["answers"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(report["requests"][0]["requested_model"], "jev-latest");
    assert_eq!(
        report["requests"][0]["resolved_model"],
        "typesafe/controlled"
    );
    assert_eq!(read(&root.path().join("run.json")), report);
    for file in report["rulebook_files"].as_array().unwrap() {
        let content = file["content"].as_str().unwrap();
        assert_eq!(
            file["sha256"],
            sunstack_evidence::content_digest(content.as_bytes())
        );
    }
    rules["K001"]["questions"]["quality"]["instructions"]["context"] =
        json!(["reference.examples", "where"]);
    save(&path, &rules);
    let (_, bodies) = judged(root.path(), 0.9, 0).await;
    let request: Value = serde_json::from_slice(&bodies[0]).unwrap();
    assert_eq!(
        request["state"]["where"],
        json!({"file":"docs/one.md","first_line":1,"last_line":1})
    );
}

#[tokio::test]
async fn reference_size_is_counted_in_exact_request_cap_without_dispatching_large_peer() {
    let root = fixture();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["skills/verification-design/SKILL.md"]);
    config["limits"] = json!({"request_max_chars":100000});
    save(&config_path, &config);
    let mut entries = read(&root.path().join("rulebook/steering.rules.json"));
    let mut large = entries["K003"].clone();
    large["name"] = json!("large");
    large["reference"] = json!({"examples":{"text":"é尾\"\\\n".repeat(2000)}});
    large["questions"]["names_trigger"]["instructions"]["context"] = json!(["reference.examples"]);
    entries = json!({"K003":entries["K003"],"K004":large});
    save(&root.path().join("rulebook/steering.rules.json"), &entries);
    std::fs::write(root.path().join("rulebook/steering.md"),"Description names a trigger. <!-- K003 -->\n\nDescription follows the examples. <!-- K004 -->\n").unwrap();
    assert_encoded_cap_blocks_only_large(root.path()).await;
}

#[tokio::test]
async fn saved_filter_keeps_raw_mechanical_fields_and_matches_after_folder_deletion() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let sentinel = "RAW_MECHANICAL_MIDDLE_SENTINEL";
    let opening = format!("{} {sentinel} {}\n", "x".repeat(1700), "y".repeat(1700));
    std::fs::write(root.path().join("docs/one.md"), "Checked member.\n").unwrap();
    std::fs::write(
        root.path().join("docs/about.md"),
        format!("---\ndescription: Portable description\n---\n\n{opening}"),
    )
    .unwrap();
    std::fs::write(root.path().join("evidence/rules/pieces.yml"), format!("id: document\nlanguage: Markdown\nrule: {{kind: document}}\n---\nid: marker\nlanguage: Markdown\nrule: {{kind: paragraph, regex: '{sentinel}'}}\n")).unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["check"]["files"] = json!(["docs/one.md"]);
    config["evidence"]["kinds"]["folder"] = json!({"rule":"document","max_chars":1000,"fields":{"description":{"from":"yaml","key":"description"},"opening":{"from":"remainder"}}});
    config["filters"] = json!({"purpose":{"per":"{name}","reads":{"file":"about.md","kind":"folder","fields":["description","opening"]},"labels":["eligible","blocked"],"questions":{"quality":{"type":"noul","instructions":{"check":"description"}}},"decision":{"start":"gate","steps":{
        "gate":{"ask":"quality","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"content","no":{"label":"blocked"},"uncertain":"unsure"},
        "content":{"mechanical_check":{"tool":"contains","field":"opening","text":sentinel},"yes":"matcher","no":{"label":"blocked"}},
        "matcher":{"mechanical_check":{"tool":"ast_grep","rule":"marker"},"yes":{"label":"eligible"},"no":{"label":"blocked"}}
    }}}});
    save(&path, &config);
    let mut entries = read(&root.path().join("rules.json"));
    entries["K001"]["only_when"] = json!({"purpose":["eligible"]});
    save(&root.path().join("rules.json"), &entries);
    let (report, bodies) = judged_with_args(root.path(), 0.9, 1, &["--report", "run.json"]).await;
    assert_eq!(bodies.len(), 1);
    let request: Value = serde_json::from_slice(&bodies[0]).unwrap();
    assert!(
        request["state"].get("opening").is_none(),
        "mechanical-only field must stay out of model state"
    );
    assert!(!request.to_string().contains(sentinel));
    assert_eq!(report["filter_answers"][0]["label"], "eligible");
    assert_eq!(
        report["filter_answers"][0]["path"],
        json!(["gate", "content", "matcher"])
    );
    let id = report["filter_answers"][0]["id"]
        .as_u64()
        .unwrap()
        .to_string();
    let evidence = &report["piece_evidence"][&id];
    let raw = evidence["piece"]["fields"]["opening"]["value"]
        .as_str()
        .unwrap();
    assert!(raw.contains(sentinel));
    assert!(
        evidence["piece"]["model_view"]["cut"]
            .as_array()
            .unwrap()
            .contains(&json!("opening"))
    );
    assert!(
        !evidence["piece"]["model_view"]["fields"]["opening"]
            .as_str()
            .unwrap()
            .contains(sentinel)
    );
    assert_eq!(evidence["piece"]["where"]["file"], "docs/about.md");
    let matches = evidence["matches"]["marker"].as_array().unwrap();
    assert!(!matches.is_empty());
    for matched in matches {
        assert!(
            matched["start"].as_u64().unwrap()
                >= evidence["piece"]["where"]["span"]["byte_start"]
                    .as_u64()
                    .unwrap()
        );
        assert!(
            matched["end"].as_u64().unwrap()
                <= evidence["piece"]["where"]["span"]["byte_end"]
                    .as_u64()
                    .unwrap()
        );
    }
    std::fs::remove_dir_all(root.path().join("docs")).unwrap();
    let detail = support::isolated_command(root.path())
        .args(["lint", "show", "run.json", &id])
        .output()
        .await
        .unwrap();
    assert_eq!(detail.status.code(), Some(0));
    let detail: Value = serde_json::from_slice(&detail.stdout).unwrap();
    assert_eq!(&detail["evidence"], evidence);
    assert!(
        detail["evidence"]["piece"]["fields"]["opening"]["value"]
            .as_str()
            .unwrap()
            .contains(sentinel)
    );
}
