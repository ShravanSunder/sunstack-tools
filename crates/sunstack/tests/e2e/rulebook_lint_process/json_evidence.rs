use super::*;

fn json_fixture() -> tempfile::TempDir {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"contains","field":"body","text":"required"})),
    );
    std::fs::write(root.path().join("evidence/rules/pieces.yml"), "id: document\nlanguage: Markdown\nrule: {kind: document}\n---\nid: json_pair\nlanguage: Json\nrule: {kind: pair}\n---\nid: json_object\nlanguage: Json\nrule: {kind: object}\n---\nid: json_string\nlanguage: Json\nrule: {kind: string}\n").unwrap();
    std::fs::write(root.path().join("docs/one.md"), "Markdown needs a nudge.\n").unwrap();
    std::fs::write(
        root.path().join("docs/one.json"),
        "{\n  \"good\": true,\n  \"needs_check\": false\n}\n",
    )
    .unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["check"]["files"] = json!(["docs/*.md", "docs/*.json"]);
    config["evidence"]["kinds"]["pair"] = json!({"rule":"json_pair","fields":{
        "heading":{"from":"first_line"}, "captured":{"from":"capture","regex":"(needs_check)"}, "inside":{"from":"inside","rule":"json_string"},
        "enclosing":{"from":"enclosing","rule":"json_object","text":"full"}, "same":{"from":"same_file","rule":"json_string"},
        "next":{"from":"next","rule":"json_pair"}, "previous":{"from":"previous","rule":"json_pair"},
        "body":{"from":"remainder"}, "targets":{"from":"target","regex":"(no_such_target)","resolve":"path"}
    }});
    save(&path, &config);
    let path = root.path().join("rules.json");
    let mut entries = read(&path);
    entries["K002"] = json!({"name":"json_standard","evidence":"pair","decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"heading","text":"needs_check"},"yes":{"finding":"Name the checked field.","at":"heading"},"no":"pass"}}}});
    save(&path, &entries);
    std::fs::write(root.path().join("rulebook.md"), "Document meets the standard. <!-- K001 -->\n\nQuestions name their checked field. <!-- K002 -->\n").unwrap();
    root
}

#[tokio::test]
async fn mixed_json_and_markdown_keep_source_lines_and_isolate_an_invalid_json_file() {
    let root = json_fixture();
    assert_eq!(
        invoke(root.path(), &["--check"], 0)
            .await
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let listed = invoke(root.path(), &["--all", "--evidence", "pair"], 0).await;
    assert_eq!(listed["pieces"].as_array().unwrap().len(), 2);
    let first = &listed["pieces"][0];
    assert_eq!(first["where"]["first_line"], 2);
    assert_eq!(
        first["fields"]["next"]["values"],
        json!(["\"needs_check\": false"])
    );
    assert_eq!(
        listed["pieces"][1]["fields"]["previous"]["values"],
        json!(["\"good\": true"])
    );
    let report = invoke(root.path(), &["--all", "--format", "json"], 1).await;
    assert_eq!(report["findings"].as_array().unwrap().len(), 2);
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["rule"] == "json_standard")
        .unwrap();
    assert_eq!(finding["file"], "docs/one.json");
    assert_eq!(finding["line"], 3);
    assert_eq!(finding["span"]["start_line"], 3);
    std::fs::write(root.path().join("docs/broken.json"), "{\n \"broken\":\n").unwrap();
    let incomplete = invoke(root.path(), &["--all", "--format", "json"], 3).await;
    assert_eq!(incomplete["findings"], report["findings"]);
    assert!(
        incomplete["incomplete"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["file"] == "docs/broken.json")
    );
    assert!(
        !incomplete["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["file"] == "docs/broken.json")
    );
}

#[tokio::test]
async fn json_kinds_refuse_yaml_and_cross_language_references_with_the_field_named() {
    for source in [
        json!({"from":"yaml","key":"description"}),
        json!({"from":"inside","rule":"document"}),
    ] {
        let root = json_fixture();
        let path = root.path().join("skill-rules.config.json");
        let mut config = read(&path);
        config["evidence"]["kinds"]["pair"]["fields"]["bad_field"] = source;
        save(&path, &config);
        let output = command(root.path()).arg("--check").output().await.unwrap();
        assert_eq!(output.status.code(), Some(2));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for part in ["pair", "bad_field"] {
            assert!(text.contains(part), "{text}");
        }
    }
    let root = json_fixture();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]["json_reference"] =
        json!({"rule":"json_pair","fields":{"heading":{"from":"first_line"}}});
    config["evidence"]["kinds"]["review"]["fields"]["reference"] =
        json!({"from":"target","regex":"(one.json)","resolve":"path","as_kind":"json_reference"});
    save(&path, &config);
    assert_eq!(
        invoke(root.path(), &["--check"], 0)
            .await
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn a_rulefile_with_three_entries_points_at_the_question_missing_its_checked_field() {
    let root = json_fixture();
    let rules = "{\n \"first\": {\"questions\": {\"question\": {\"type\": \"noul\", \"instructions\": {\"check\": \"body\", \"task\": \"Read it.\"}}}},\n \"second\": {\"questions\": {\"question\": {\"type\": \"noul\", \"instructions\": {\"task\": \"Missing checked field.\"}}}},\n \"third\": {\"questions\": {\"question\": {\"type\": \"noul\", \"instructions\": {\"check\": \"body\", \"task\": \"Read it.\"}}}}\n}\n";
    std::fs::write(root.path().join("docs/authored.rules.json"), rules).unwrap();
    std::fs::write(
        root.path().join("evidence/rules/questions.yml"),
        "id: json_question\nlanguage: Json\nrule: {kind: pair, regex: '^\"question\"'}\n",
    )
    .unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]["pair"]["rule"] = json!("json_question");
    save(&path, &config);
    let path = root.path().join("rules.json");
    let mut entries = read(&path);
    entries["K002"]["decision"] = json!({"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"heading","text":"\"check\""},"yes":"pass","no":{"finding":"Name the checked field.","at":"heading"}}}});
    save(&path, &entries);
    let report = invoke(root.path(), &["--all", "--format", "json"], 1).await;
    let findings = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule"] == "json_standard")
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["file"], "docs/authored.rules.json");
    assert_eq!(findings[0]["line"], 3);
    assert_eq!(
        report["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["name"] == "json_standard")
            .count(),
        3
    );
}

#[tokio::test]
async fn disabled_json_evidence_rules_still_refuse_markdown_only_fields_at_check() {
    let root = json_fixture();
    let path = root.path().join("evidence/rules/pieces.yml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace("id: json_pair\n", "id: json_pair\nseverity: off\n"),
    )
    .unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]["pair"]["fields"]["bad_field"] =
        json!({"from":"yaml","key":"description"});
    save(&path, &config);
    let output = command(root.path()).arg("--check").output().await.unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("pair") && text.contains("bad_field"),
        "{text}"
    );
}
