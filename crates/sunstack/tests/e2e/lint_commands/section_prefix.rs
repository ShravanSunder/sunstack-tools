use super::*;

fn prefix_fixture(root: &Path, html: &str) -> Value {
    let mut config = fixture::write(root);
    config["check"]["files"] = json!(["docs/*.html"]);
    config["evidence"]["html"] = json!({"text_attributes":{"figure":["caption"]}});
    config["evidence"]["kinds"] = json!({
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
        "subject":{"rule":"skill","fields":{
            "prefix":{"from":"section_prefix","kind":"outline"},
            "full":{"from":"enclosing","kind":"outline","text":"full"}
        }}
    });
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        "id: skill\nlanguage: Html\nrule: {kind: element, regex: '^<aside'}\n",
    )
    .unwrap();
    std::fs::write(root.join("docs/page.html"), html).unwrap();
    fixture::config_file(root, &config);
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"prefix_content","evidence":"subject","severity":"error",
            "questions":{"content":{"type":"noul","instructions":{"check":"prefix","context":["full"],"task":"Is the preceding explanation sufficient?"}}},
            "decision":{"start":"content","steps":{"content":{"ask":"content","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Explain it."},"uncertain":"uncertain"}}}
        }}),
    );
    config
}

#[tokio::test]
async fn section_prefix_preparation_keeps_per_subject_raw_spans_and_projected_views() {
    // Arrange: wrappers do not define the heading outline, and the second subject has a later end.
    let root = tempfile::tempdir().unwrap();
    let html = "猫\n<h2>Outer</h2><figure caption='Before &amp; now'>Picture</figure><div><h3>Inner</h3><p>é brief</p><aside>FIRST</aside><p>Between</p><aside>SECOND</aside></div><h2>Peer</h2><p>LATER</p>";
    prefix_fixture(root.path(), html);
    let saved = root.path().join("prepared.json");

    // Act: exercise the public CLI without credentials or inference.
    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;
    let report: Value = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();

    // Assert: exact raw bytes and provenance, independent from HTML asking projection.
    let pieces = report["piece_evidence"]
        .as_object()
        .unwrap()
        .values()
        .map(|entry| &entry["piece"])
        .collect::<Vec<_>>();
    assert_eq!(pieces.len(), 2, "{report}");
    for (piece, subject) in pieces.iter().zip(["<aside>FIRST", "<aside>SECOND"]) {
        let start = html.find("<h3>").unwrap();
        let end = html.find(subject).unwrap();
        assert_eq!(piece["fields"]["prefix"]["value"], html[start..end]);
        let span = &piece["fields"]["prefix"]["spans"][0];
        assert_eq!(span["byte_start"], start);
        assert_eq!(span["byte_end"], end);
        assert_eq!(span["start_line"], 2);
        assert_eq!(span["start_column"], html[4..start].chars().count() + 1);
        assert_eq!(span["source"], piece["where"]["span"]["source"]);
        assert_eq!(
            span["source"]["content_identity"],
            sunstack_evidence::content_digest(html.as_bytes())
        );
    }
    let requests = report["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["request"]["state"]["prefix"], "Inner é brief");
    assert_eq!(
        requests[1]["request"]["state"]["prefix"],
        "Inner é brief FIRST Between"
    );
    assert!(
        requests[0]["request"]["state"]["full"]
            .to_string()
            .contains("SECOND")
    );
    assert!(
        !requests[0]["request"]["state"]["prefix"]
            .to_string()
            .contains("FIRST")
    );
    assert_eq!(report["usage"]["model_requests"], 0);

    // The caller can deliberately select a broader governing section kind.
    let mut config = prefix_fixture(root.path(), html);
    config["evidence"]["kinds"]["outline"]["html_units"]["starts_with"] = json!("Outer");
    fixture::config_file(root.path(), &config);
    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;
    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    assert!(
        report["requests"][0]["request"]["state"]["prefix"]
            .as_str()
            .unwrap()
            .contains("[caption: Before & now]")
    );
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn section_prefix_complete_rejects_a_cut_or_missing_section_but_accepts_empty() {
    for scenario in ["cut", "missing", "empty"] {
        let root = tempfile::tempdir().unwrap();
        let html = match scenario {
            "cut" => format!(
                "<h2>Start</h2><p>{}</p><aside>Subject</aside>",
                "long ".repeat(2000)
            ),
            "missing" => "<aside>Subject without any configured section</aside>".into(),
            _ => "<h2>Start</h2>".into(),
        };
        let mut config = prefix_fixture(root.path(), &html);
        config["limits"] = json!({"evidence_max_chars":1000});
        if scenario == "empty" {
            std::fs::write(
                root.path().join("evidence/rules/skill.yml"),
                "id: skill\nlanguage: Html\nrule: {kind: element, regex: '^<h2'}\n",
            )
            .unwrap();
        }
        fixture::config_file(root.path(), &config);
        fixture::entry_file(
            root.path(),
            &json!({"K001":{
                "name":"prefix_complete","evidence":"subject","severity":"error",
                "decision":{"start":"complete","steps":{"complete":{
                    "mechanical_check":{"tool":"complete","field":"prefix"},"yes":"pass","no":{"finding":"Prefix incomplete."}
                }}}
            }}),
        );

        let report = report(
            root.path(),
            &["--all"],
            if scenario == "empty" {
                0
            } else if scenario == "cut" {
                1
            } else {
                3
            },
        )
        .await;

        assert_eq!(
            report["results"][0]["outcome"],
            if scenario == "empty" {
                "pass"
            } else if scenario == "cut" {
                "finding"
            } else {
                "incomplete"
            },
            "{report}"
        );
        let piece = &report["piece_evidence"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()["piece"];
        if scenario == "empty" {
            assert_eq!(piece["fields"]["prefix"]["value"], "");
            assert_eq!(piece["fields"]["prefix"]["spans"][0]["byte_start"], 0);
            assert_eq!(piece["fields"]["prefix"]["spans"][0]["byte_end"], 0);
        } else if scenario == "cut" {
            assert!(
                piece["model_view"]["cut"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("prefix"))
            );
        } else {
            assert_eq!(piece["fields"]["prefix"]["state"], "unavailable");
        }
        assert_eq!(report["usage"]["model_requests"], 0);
    }
}

#[tokio::test]
async fn section_prefix_waits_for_selected_containers_and_refuses_bad_kind_contracts() {
    let root = tempfile::tempdir().unwrap();
    let baseline = prefix_fixture(root.path(), "<h2>Heading</h2><aside>Subject</aside>");
    let mut config = baseline.clone();
    config["evidence"]["kinds"]["outline"]["fields"] =
        json!({"heading":{"from":"section","text":"heading"}});
    config["evidence"]["kinds"]["outline"]["select"] = json!({
        "question":{"type":"noul","instructions":{"check":"heading","task":"Keep this section?"}},
        "yes_at_or_above":0.7,"no_at_or_below":0.3
    });
    fixture::config_file(root.path(), &config);
    let saved = root.path().join("pending.json");

    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;

    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    assert_eq!(report["usage"]["model_requests"], 0);
    assert_eq!(report["requests"].as_array().unwrap().len(), 1);
    assert_eq!(report["requests"][0]["owner"]["kind"], "selection");
    assert!(!report["needs_answer"].as_array().unwrap().is_empty());
    for (kind, expected) in [("missing", "undefined"), ("subject", "cycle")] {
        let mut config = baseline.clone();
        config["evidence"]["kinds"]["subject"]["fields"]["prefix"]["kind"] = json!(kind);
        fixture::config_file(root.path(), &config);
        let output = invoke(root.path(), &["--check"], 2).await;
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    let mut config = baseline;
    config["evidence"]["kinds"]["outline"] =
        json!({"inline_rule":"{language: Markdown, rule: {kind: section}}","fields":{}});
    fixture::config_file(root.path(), &config);
    let output = invoke(root.path(), &["--check"], 2).await;
    assert!(String::from_utf8_lossy(&output.stderr).contains("another evidence language"));
}

#[tokio::test]
async fn section_prefix_markdown_preserves_literal_markup_and_nearest_section() {
    let root = tempfile::tempdir().unwrap();
    let mut config = prefix_fixture(root.path(), "");
    config["check"]["files"] = json!(["docs/*.md"]);
    config["evidence"]["kinds"]["outline"] =
        json!({"markdown_units":{"mode":"unit","units":["section"]},"fields":{}});
    std::fs::write(
        root.path().join("evidence/rules/skill.yml"),
        "id: skill\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^Subject'}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("docs/one.md"),
        "# Outer\n\n## Inner\n\nBefore <em>literal</em>.\n\nSubject text.\n",
    )
    .unwrap();
    fixture::config_file(root.path(), &config);
    let saved = root.path().join("markdown.json");

    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;

    let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    assert_eq!(
        report["requests"][0]["request"]["state"]["prefix"],
        "## Inner\n\nBefore <em>literal</em>.\n\n"
    );
    assert_eq!(report["usage"]["model_requests"], 0);
}
