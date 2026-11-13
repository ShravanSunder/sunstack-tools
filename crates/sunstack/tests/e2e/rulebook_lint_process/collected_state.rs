use super::*;

fn loaded_fixture(resolve: &str, opening_chars: usize) -> tempfile::TempDir {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"},"targets":{"from":"target","regex":"`([^`]+)`","resolve":resolve,"opening_chars":opening_chars}}),
        json!({"start":"contains","steps":{
            "contains":{"ask":"covers","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"finding":"Read this material and decide where it belongs."},"no":"complete","uncertain":"uncertain"},
            "complete":{"mechanical_check":{"tool":"complete","field":"targets"},"yes":"pass","no":"uncertain"}
        }}),
    );
    std::fs::write(
        root.path().join("evidence/rules/pieces.yml"),
        "id: document\nlanguage: Markdown\nrule: {kind: document, regex: '^Load'}\n",
    )
    .unwrap();
    let path = root.path().join("rules.json");
    let mut entries = read(&path);
    entries["K001"]["questions"] = json!({"covers":{"type":"noul","instructions":{"check":"body","context":["targets"],"task":"Does this contain product material?"}}});
    save(&path, &entries);
    root
}

#[tokio::test]
async fn actual_skill_name_openings_and_cut_flags_reach_the_single_http_request() {
    let skill = "---\nname: reader\ndescription: Portable reader guidance\n---\nRead carefully.\n";
    for (opening_chars, exit) in [(1000, 0), (12, 5)] {
        let root = loaded_fixture("skill", opening_chars);
        std::fs::create_dir_all(root.path().join("skills/reader")).unwrap();
        std::fs::write(root.path().join("skills/reader/SKILL.md"), skill).unwrap();
        std::fs::write(root.path().join("docs/one.md"), "Load `reader`.\n").unwrap();
        let path = root.path().join("skill-rules.config.json");
        let mut config = read(&path);
        config["check"]["files"] = json!(["docs/*.md", "skills/*/SKILL.md"]);
        save(&path, &config);
        let (report, bodies) = judged(root.path(), 0.46, exit).await;
        assert_eq!(bodies.len(), 1);
        let body: Value = serde_json::from_slice(&bodies[0]).unwrap();
        let target = &body["state"]["targets"][0];
        assert_eq!(target["path"], "skills/reader/SKILL.md");
        assert_eq!(target["exists"], true);
        assert_eq!(
            target["opening"],
            skill.chars().take(opening_chars).collect::<String>()
        );
        assert_eq!(
            target
                .get("opening_cut")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            opening_chars < skill.chars().count()
        );
        assert_eq!(body["state"]["unavailable-fields"], json!({}));
        assert!(body["state"].get("where").is_none());
        assert_eq!(report["usage"]["model_requests"], 1);
        assert_eq!(report["results"].as_array().unwrap().len(), 1);
        assert_eq!(
            report["results"][0]["outcome"],
            if exit == 0 { "pass" } else { "uncertain" }
        );
        assert_eq!(
            report["results"][0]["path"],
            json!(["contains", "complete"])
        );
    }
}

#[tokio::test]
async fn collected_unavailable_target_is_named_in_state_and_refused_before_http() {
    let root = loaded_fixture("path", 1000);
    std::fs::write(root.path().join("docs/one.md"), "Load `bad.md`.\n").unwrap();
    std::fs::write(root.path().join("docs/bad.md"), [0xff, 0xfe]).unwrap();
    let config_path = root.path().join("skill-rules.config.json");
    let mut config = read(&config_path);
    config["check"]["files"] = json!(["docs/one.md"]);
    save(&config_path, &config);
    let (report, bodies) = judged(root.path(), 0.46, 3).await;
    assert!(bodies.is_empty());
    assert_eq!(report["usage"]["model_requests"], 0);
    assert_eq!(report["results"][0]["gap"], "evidence_unavailable");
    assert_eq!(report["incomplete"][0]["field"], "targets");
    assert_eq!(report["results"][0]["path"], json!(["contains"]));
    let piece: sunstack_evidence::EvidencePiece =
        serde_json::from_value(report["piece_evidence"]["1"]["piece"].clone()).unwrap();
    let project = sunstack_evidence::load_evidence_project(
        &root.path().join("evidence/sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let assembled = sunstack_rules::assemble_rulebook(
        &[sunstack_rules::RulebookPair {
            markdown: root.path().join("rulebook.md"),
            rules: root.path().join("rules.json"),
        }],
        &sunstack_evidence::MarkerPrefix::new("K").unwrap(),
        400,
        &kinds,
        &project,
        &Default::default(),
    )
    .unwrap();
    let state =
        serde_json::to_value(sunstack_rules::piece_state(&assembled.rules[0], &piece).unwrap())
            .unwrap();
    assert_eq!(state["unavailable-fields"], json!({"targets":"unreadable"}));
    assert!(state.get("targets").is_none());
}
