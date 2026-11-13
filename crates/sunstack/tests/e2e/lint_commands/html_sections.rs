use super::*;

fn outline_fixture(root: &Path) -> (Value, String) {
    let mut config = fixture::write(root);
    config["check"]["files"] = json!(["docs/*.html"]);
    // The native kind intentionally shares its name with the existing AST rule.
    config["evidence"]["kinds"] = json!({"skill": {
        "html_units":{"mode":"unit","units":["section"]},
        "fields":{
            "complete":{"from":"section","text":"full"},
            "own_body":{"from":"section","text":"own"},
            "parent_body":{"from":"section","text":"enclosing_own"}
        }
    }});
    config["evidence"]["kinds"]["ast_heading"] =
        json!({"rule":"skill","fields":{"body":{"from":"remainder"}}});
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        "id: skill\nlanguage: Html\nrule: {kind: element, regex: '^<h3>'}\n",
    )
    .unwrap();
    fixture::config_file(root, &config);
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"section_picture","evidence":"skill","severity":"warn",
            "decision":{"start":"raw","steps":{"raw":{
                "mechanical_check":{"tool":"contains","field":"own_body","text":"<h"},
                "yes":"pass","no":{"finding":"Raw heading markup was lost."}
            }}}
        }}),
    );
    let html = "猫<div><h2>Parent</h2>\n<p>One</p><p>Two</p><p>Three</p><p>Four</p><p>Five</p><p>Six</p><img alt='Late diagram'>\n<section><h3>Child</h3><p>Child body</p></section><p>Lexical child tail</p></div><h2>Sibling</h2><p>Last</p>".to_owned();
    std::fs::write(root.join("docs/page.html"), &html).unwrap();
    (config, html)
}

#[tokio::test]
async fn html_outline_evidence_uses_exact_lexical_fields_and_normal_absent_parent() {
    let root = tempfile::tempdir().unwrap();
    let (_, html) = outline_fixture(root.path());
    let child = html.find("<h3>").unwrap();
    let sibling = html.find("<h2>Sibling").unwrap();
    let parent = html.find("<h2>Parent").unwrap();

    let output = invoke(root.path(), &["--all", "--evidence", "skill"], 0).await;

    let listing: Value = serde_json::from_slice(&output.stdout).unwrap();
    let pieces = listing["pieces"].as_array().unwrap();
    assert_eq!(pieces.len(), 3, "{listing}");
    assert_eq!(pieces[0]["text"], html[parent..sibling]);
    assert_eq!(pieces[0]["where"]["span"]["byte_start"], parent);
    assert_eq!(pieces[0]["where"]["span"]["byte_end"], sibling);
    assert_eq!(
        pieces[0]["fields"]["own_body"]["value"],
        html[parent..child]
    );
    assert_eq!(
        pieces[0]["fields"]["parent_body"],
        json!({"state":"absent"})
    );
    assert_eq!(
        pieces[1]["fields"]["parent_body"]["value"],
        html[parent..child]
    );
    assert_eq!(
        pieces[1]["fields"]["own_body"]["value"],
        html[child..sibling]
    );
    assert_eq!(pieces[2]["text"], html[sibling..]);
    assert!(
        pieces[0]["model_view"]["fields"]["own_body"]
            .as_str()
            .unwrap()
            .contains("[alt: Late diagram]")
    );
    assert_eq!(listing["usage"]["model_requests"], 0);
    assert_eq!(
        listing["file_counts"][0]["kinds"]["ast_heading"]["matches"], 1,
        "native ranges must not replace same-named AST rule matches"
    );
}

#[tokio::test]
async fn html_section_prepare_keeps_raw_fields_and_marks_model_cuts() {
    let root = tempfile::tempdir().unwrap();
    let (mut config, _) = outline_fixture(root.path());
    config["evidence"]["kinds"]["skill"]["max_chars"] = json!(1000);
    fixture::config_file(root.path(), &config);
    let long_body = "Long section explanation. ".repeat(160);
    std::fs::write(root.path().join("docs/page.html"), format!("<h2>Only section</h2><p>{long_body}</p><img alt='Last diagram'><ex-mermaid label='Flow'><script type='text/plain'>graph LR; A --> B</script></ex-mermaid>")).unwrap();
    fixture::entry_file(
        root.path(),
        &json!({"K001":{
            "name":"section_picture","evidence":"skill","severity":"warn",
            "questions":{
                "own":{"type":"noul","instructions":{"check":"own_body","task":"Does this explanation have a picture?"}},
                "parent":{"type":"noul","instructions":{"check":"parent_body","task":"Does the enclosing section supply a picture?"}},
                "complete":{"type":"noul","instructions":{"check":"complete","task":"Is this section complete?"}}
            },
            "decision":{"start":"raw","steps":{
                "raw":{"mechanical_check":{"tool":"contains","field":"own_body","text":"<h2>"},"yes":"own","no":{"finding":"Raw heading markup was lost."}},
                "own":{"ask":"own","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Add a picture."},"uncertain":"uncertain"}
            }}
        }}),
    );
    let saved = root.path().join("prepared.json");

    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;

    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    assert_eq!(report["requests"].as_array().unwrap().len(), 1, "{report}");
    let state = &report["requests"][0]["request"]["state"];
    assert!(state["parent_body"].is_null(), "{state}");
    assert!(
        state["cut-fields"].as_array().is_some_and(|fields| fields
            .iter()
            .any(|field| field == "own_body" || field == "complete")),
        "{state}"
    );
    assert!(state.to_string().contains("[cut]"), "{state}");
    let piece = report["piece_evidence"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert!(
        piece["piece"]["fields"]["own_body"]["value"]
            .as_str()
            .unwrap()
            .starts_with("<h2>Only section</h2>")
    );
    assert!(
        piece["piece"]["fields"]["own_body"]["value"]
            .as_str()
            .unwrap()
            .contains("<img alt='Last diagram'>")
    );
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn html_section_byte_budget_refuses_unread_content_without_preparing_judgments() {
    let root = tempfile::tempdir().unwrap();
    let (mut config, _) = outline_fixture(root.path());
    config["limits"] = json!({"max_file_bytes":20});
    fixture::config_file(root.path(), &config);
    let saved = root.path().join("limited.json");

    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        3,
    )
    .await;

    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    assert_eq!(report["status"]["kind"], "incomplete");
    assert!(report["requests"].as_array().unwrap().is_empty());
    assert!(!report["incomplete"].as_array().unwrap().is_empty());
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn filtered_html_section_keeps_its_unselected_enclosing_outline_context() {
    let root = tempfile::tempdir().unwrap();
    let (mut config, html) = outline_fixture(root.path());
    config["evidence"]["kinds"]["skill"]["html_units"]["starts_with"] = json!("Child");
    fixture::config_file(root.path(), &config);

    let output = invoke(root.path(), &["--all", "--evidence", "skill"], 0).await;

    let listing: Value = serde_json::from_slice(&output.stdout).unwrap();
    let pieces = listing["pieces"].as_array().unwrap();
    assert_eq!(pieces.len(), 1, "{listing}");
    let parent = html.find("<h2>Parent").unwrap();
    let child = html.find("<h3>").unwrap();
    assert_eq!(
        pieces[0]["fields"]["parent_body"]["value"],
        html[parent..child]
    );
    assert!(
        pieces[0]["model_view"]["fields"]["parent_body"]
            .as_str()
            .unwrap()
            .contains("[alt: Late diagram]")
    );
}

#[tokio::test]
async fn html_selection_candidate_and_shared_request_views_are_projected() {
    let root = tempfile::tempdir().unwrap();
    let (mut config, _) = outline_fixture(root.path());
    let rule_path = root.path().join("evidence/rules/skill.yml");
    let rules = std::fs::read_to_string(&rule_path).unwrap();
    std::fs::write(
        rule_path,
        format!("{rules}---\nid: page\nlanguage: Html\nrule: {{kind: document}}\n"),
    )
    .unwrap();
    config["evidence"]["kinds"]["skill"]["fields"]["shared_intro"] =
        json!({"from":"same_file","rule":"page","text":"full"});
    config["evidence"]["kinds"]["skill"]["select"] = json!({
        "question":{"type":"noul","instructions":{"check":"own_body","task":"Keep sections with a picture?"}},
        "yes_at_or_above":0.7,"no_at_or_below":0.3,"shared":["shared_intro"]
    });
    fixture::config_file(root.path(), &config);
    let saved = root.path().join("selection.json");

    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;

    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    let request = report["requests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|request| request["owner"]["kind"] == "selection")
        .unwrap();
    let state = &request["request"]["state"];
    let candidate = &state["candidates"]["c1"]["own_body"];
    assert!(
        candidate
            .as_str()
            .is_some_and(|text| text.contains("[alt: Late diagram]") && !text.contains("<h2>")),
        "{state}"
    );
    let shared = &state["shared"]["shared_intro"][0];
    assert!(
        shared
            .as_str()
            .is_some_and(|text| text.contains("[alt: Late diagram]") && !text.contains("<h2>")),
        "{state}"
    );
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn recovered_html_headings_keep_exact_fields_and_prepare_every_section() {
    let root = tempfile::tempdir().unwrap();
    outline_fixture(root.path());
    fixture::entry_file(
        root.path(),
        &json!({"K001": {
            "name":"section_picture", "evidence":"skill", "severity":"warn",
            "questions":{"picture":{"type":"noul","instructions":{
                "check":"complete","task":"Does this section explain its picture?"
            }}},
            "decision":{"start":"picture","steps":{"picture":{
                "ask":"picture","yes_at_or_above":0.8,"no_at_or_below":0.2,
                "yes":"pass","no":{"finding":"Add the picture."},"uncertain":"uncertain"
            }}}
        }}),
    );
    // Literal offsets belong to the original source, independently of parser recovery.
    for (name, html, second_start, child) in [
        ("both-unclosed", "<h2>Parent<h3>Child", 10, true),
        ("siblings-unclosed", "<h2>A<h2>B", 5, false),
        ("newline", "<h2>Parent\n<h3>Child\n", 11, true),
        ("parent-unclosed", "<h2>Parent<h3>Child</h3>", 10, true),
        ("child-unclosed", "<h2>Parent</h2><h3>Child", 15, true),
        ("proper", "<h2>Parent</h2><h3>Child</h3>", 15, true),
    ] {
        std::fs::write(root.path().join("docs/page.html"), html).unwrap();
        let output = invoke(root.path(), &["--all", "--evidence", "skill"], 0).await;
        let listing: Value = serde_json::from_slice(&output.stdout).unwrap();
        let pieces = listing["pieces"].as_array().unwrap();
        assert_eq!(pieces.len(), 2, "{name}: {listing}");
        let first_end = if child { html.len() } else { second_start };
        assert_eq!(pieces[0]["text"], html[..first_end], "{name}");
        assert_eq!(pieces[0]["where"]["span"]["byte_start"], 0);
        assert_eq!(pieces[0]["where"]["span"]["byte_end"], first_end);
        assert_eq!(
            pieces[0]["fields"]["own_body"]["value"],
            html[..second_start]
        );
        assert_eq!(pieces[1]["text"], html[second_start..]);
        assert_eq!(pieces[1]["where"]["span"]["byte_start"], second_start);
        assert_eq!(pieces[1]["where"]["span"]["byte_end"], html.len());
        if child {
            assert_eq!(
                pieces[1]["fields"]["parent_body"]["value"],
                html[..second_start]
            );
        } else {
            assert_eq!(
                pieces[1]["fields"]["parent_body"],
                json!({"state":"absent"})
            );
        }
        let saved = root.path().join(format!("{name}.json"));
        invoke(
            root.path(),
            &["--all", "--prepare", saved.to_str().unwrap()],
            6,
        )
        .await;
        let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
        assert_eq!(
            report["requests"].as_array().unwrap().len(),
            2,
            "{name}: {report}"
        );
        let state = &report["requests"][0]["request"]["state"];
        let asking_text = state["complete"].as_str().unwrap();
        if asking_text.contains("[cut: html_projection_unrecoverable_markup]") {
            assert_ne!(name, "proper", "complete headings must not be cut");
            assert!(
                asking_text.ends_with("[cut: html_projection_unrecoverable_markup]"),
                "{name}: {state}"
            );
            assert!(
                state["cut-fields"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("complete")),
                "{name}: ambiguous asking text must carry its containing field cut"
            );
        } else {
            assert_eq!(
                asking_text,
                if child { "Parent Child" } else { "A" },
                "{name}: complete asking text must keep every heading word"
            );
            assert!(
                state["cut-fields"]
                    .as_array()
                    .is_none_or(|fields| !fields.contains(&json!("complete"))),
                "{name}: complete asking text must not carry a field cut"
            );
        }
        assert_eq!(report["usage"]["model_requests"], 0);
        assert!(
            report["incomplete"].as_array().unwrap().is_empty(),
            "{name}: {report}"
        );
    }
}
