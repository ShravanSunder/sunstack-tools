use super::*;

#[tokio::test]
async fn section_own_scope_stops_at_child_headings_and_uses_configured_text_attributes() {
    let root = tempfile::tempdir().unwrap();
    let mut config = measurement_fixture(root.path(), "html");
    config["evidence"]["html"]["text_attributes"] = json!({"p":["title"]});
    config["evidence"]["kinds"]["markdown_outline"] =
        json!({"markdown_units":{"mode":"unit","units":["section"]},"fields":{}});
    config["evidence"]["kinds"]["section"]["fields"]["prose"]["scope"] =
        json!({"section_own":["outline","markdown_outline"]});
    std::fs::write(root.path().join("docs/page.html"), "<h2>Parent</h2><p title='Seven'>One two three.</p><p>Four five six.</p><h3>Child</h3><p>Only child.</p><h2>Peer</h2><p>Only peer.</p>").unwrap();
    fixture::config_file(root.path(), &config);
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"own_section","evidence":"section","severity":"warn","decision":{"start":"count","steps":{"count":{"mechanical_check":{"tool":"word_count","field":"prose","bound":{"at_least":8}},"yes":{"finding":"{{word_count:prose}} words in {{piece_count:prose}} pieces."},"no":"pass"}}}}}),
    );
    let observed = report(root.path(), &["--all"], 0).await;
    assert_eq!(observed["counts"]["warn"], 1);
    assert_eq!(observed["findings"][0]["message"], "8 words in 2 pieces.");
    assert_eq!(observed["results"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn section_picture_readme_example_executes_the_exact_documented_block() {
    let readme = include_str!("../../../README.md");
    let script = readme
        .split_once("<!-- runnable-example: section-pictures -->\n````sh\n")
        .unwrap()
        .1
        .split_once("\n````")
        .unwrap()
        .0;
    let root = tempfile::tempdir().unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        support::documented_shell_command(root.path(), script).output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &std::fs::read(root.path().join("section-example/report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["results"].as_array().unwrap().len(), 7);
    assert_eq!(
        report["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["outcome"] == "pass")
            .count(),
        5
    );
    assert_eq!(report["counts"]["warn"], 2);
    assert_eq!(report["incomplete"], json!([]));
    assert_eq!(report["usage"]["model_requests"], 0);
    for finding in report["findings"].as_array().unwrap() {
        assert!(
            finding["message"]
                .as_str()
                .unwrap()
                .starts_with("100 words, no picture.")
        );
    }
}

fn measurement_fixture(root: &Path, surface: &str) -> Value {
    let mut config = fixture::write(root);
    let markdown = surface == "markdown";
    config["check"]["files"] = json!([if markdown { "docs/*.md" } else { "docs/*.html" }]);
    let outline = if markdown {
        json!({"markdown_units":{"mode":"unit","units":["section"]},"fields":{}})
    } else {
        json!({"html_units":{"mode":"unit","units":["section"]},"fields":{}})
    };
    config["evidence"]["html"] =
        json!({"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]});
    config["evidence"]["kinds"] = json!({
        "outline":outline,
        "prose":{"rule":"prose","fields":{}},
        "section":{"fields":{
            "prose":{"from":"inside","kind":"prose","scope":{"section":"outline"}}
        }}
    });
    config["evidence"]["kinds"]["section"][if markdown {
        "markdown_units"
    } else {
        "html_units"
    }] = outline[if markdown {
        "markdown_units"
    } else {
        "html_units"
    }]
    .clone();
    let (language, selector, page) = match surface {
        "markdown" => (
            "Markdown",
            "paragraph",
            "# First\n\n**One** [two](https://example.com/not-a-word) `three`.\n\nFour five six.\n\n# Empty\n",
        ),
        "island" => (
            "Markdown",
            "paragraph",
            "<h2>First</h2><script type=\"text/markdown\">\n**One** [two](https://example.com/not-a-word) `three`.\n\nFour five six.\n</script><h2>Empty</h2>",
        ),
        _ => (
            "Html",
            "element, regex: '^<p\\b'",
            "<h2>First</h2><p class=\"many attribute words do not count\"><strong>One</strong> <a href=\"https://example.com/not-a-word\">two</a> three.</p><p>Four five six.</p><h2>Empty</h2>",
        ),
    };
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        format!("id: prose\nlanguage: {language}\nrule: {{kind: {selector}}}\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(if markdown {
            "docs/one.md"
        } else {
            "docs/page.html"
        }),
        page,
    )
    .unwrap();
    fixture::config_file(root, &config);
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"section_measure", "evidence":"section", "severity":"warn",
            "decision":{"start":"words","steps":{
                "words":{"mechanical_check":{"tool":"word_count","field":"prose","bound":{"at_least":6}},"yes":"pieces","no":"pass"},
                "pieces":{"mechanical_check":{"tool":"piece_count","field":"prose","bound":{"greater_than":1}},"yes":{"finding":"{{word_count:prose}} words in {{piece_count:prose}} pieces."},"no":"pass"}
            }}
        }}),
    );
    config
}

#[tokio::test]
async fn section_measures_count_content_across_html_markdown_and_islands() {
    for surface in ["html", "markdown", "island"] {
        // Arrange: six visible words in two pieces; markup/URLs add no words.
        let root = tempfile::tempdir().unwrap();
        measurement_fixture(root.path(), surface);

        // Act: the complete config travels through the real CLI.
        let report = report(root.path(), &["--all"], 0).await;

        // Assert: the empty section stays zero and measured values reach the finding.
        assert_eq!(report["counts"]["warn"], 1, "{surface}: {report}");
        assert_eq!(report["findings"][0]["message"], "6 words in 2 pieces.");
        assert_eq!(report["results"].as_array().unwrap().len(), 2);
        assert_eq!(report["incomplete"], json!([]));
        assert_eq!(report["usage"]["model_requests"], 0);
    }
}

#[tokio::test]
async fn measures_combine_mixed_fields_and_keep_prefix_endpoints() {
    let root = tempfile::tempdir().unwrap();
    let mut config = measurement_fixture(root.path(), "island");
    std::fs::write(
        root.path().join("evidence/rules/html.yml"),
        "id: html-prose\nlanguage: Html\nrule: {kind: element, regex: '^<p\\b'}\n",
    )
    .unwrap();
    let page = root.path().join("docs/page.html");
    let text = std::fs::read_to_string(&page)
        .unwrap()
        .replace("<h2>Empty", "<p>Seven eight nine.</p><h2>Empty");
    std::fs::write(page, text).unwrap();
    config["evidence"]["kinds"]["html_prose"] = json!({"rule":"html-prose","fields":{}});
    config["evidence"]["kinds"]["section"]["fields"]["html_prose"] =
        json!({"from":"inside","kind":"html_prose","scope":{"section":"outline"}});
    fixture::config_file(root.path(), &config);
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"mixed","evidence":"section","severity":"warn","decision":{"start":"count","steps":{"count":{"mechanical_check":{"tool":"word_count","field":["prose","html_prose"],"bound":{"at_least":9}},"yes":{"finding":"{{word_count:prose,html_prose}} words in {{piece_count:prose,html_prose}} pieces."},"no":"pass"}}}}}),
    );
    let mixed = report(root.path(), &["--all"], 0).await;
    assert_eq!(mixed["findings"][0]["message"], "9 words in 3 pieces.");
    assert_eq!(mixed["counts"]["warn"], 1);

    let mut config = measurement_fixture(root.path(), "html");
    config["evidence"]["kinds"]["section"] = json!({"rule":"prose","fields":{"prefix":{"from":"inside","kind":"prose","scope":{"section_prefix":"outline"}}}});
    fixture::config_file(root.path(), &config);
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"prefix","evidence":"section","severity":"warn","decision":{"start":"count","steps":{"count":{"mechanical_check":{"tool":"word_count","field":"prefix","bound":{"at_least":3}},"yes":{"finding":"{{word_count:prefix}} words in {{piece_count:prefix}} pieces."},"no":"pass"}}}}}),
    );
    let prefix = report(root.path(), &["--all"], 0).await;
    assert_eq!(prefix["findings"][0]["message"], "3 words in 1 pieces.");
    assert_eq!(prefix["results"].as_array().unwrap().len(), 2);
    assert_eq!(prefix["counts"]["warn"], 1);
}

#[tokio::test]
async fn piece_counts_measure_table_rows_and_columns_and_admit_only_compatible_fields() {
    let root = tempfile::tempdir().unwrap();
    let mut config = measurement_fixture(root.path(), "html");
    std::fs::write(root.path().join("docs/page.html"), "<table><tr><th>A</th><th>B</th><th>C</th></tr><tr><td>D</td><td>E</td><td>F</td></tr><tr><td>G</td><td>H</td><td>I</td></tr></table>").unwrap();
    std::fs::write(root.path().join("evidence/rules/skill.yml"), "id: table\nlanguage: Html\nrule: {kind: element, regex: '^<table\\b'}\n---\nid: row\nlanguage: Html\nrule: {kind: element, regex: '^<tr\\b'}\n---\nid: column\nlanguage: Html\nrule: {kind: element, regex: '^<th\\b'}\n").unwrap();
    config["evidence"]["kinds"] = json!({"table":{"rule":"table","fields":{"rows":{"from":"inside","rule":"row"},"columns":{"from":"inside","rule":"column"},"body":{"from":"remainder"}}}});
    fixture::config_file(root.path(), &config);
    let baseline = json!({"K001":{"name":"table","evidence":"table","severity":"warn","decision":{"start":"rows","steps":{
        "rows":{"mechanical_check":{"tool":"piece_count","field":"rows","bound":{"greater_than":2}},"yes":"columns","no":"pass"},
        "columns":{"mechanical_check":{"tool":"piece_count","field":"columns","bound":{"at_most":2}},"yes":"pass","no":{"finding":"{{piece_count:rows}} rows, {{piece_count:columns}} columns."}}
    }}}});
    fixture::entry_file(root.path(), &baseline);
    let counted = report(root.path(), &["--all"], 0).await;
    assert_eq!(counted["findings"][0]["message"], "3 rows, 3 columns.");
    for field in [
        json!([]),
        json!(["rows", "rows"]),
        json!("missing"),
        json!("body"),
    ] {
        let mut invalid = baseline.clone();
        invalid["K001"]["decision"]["steps"]["rows"]["mechanical_check"]["field"] = field;
        fixture::entry_file(root.path(), &invalid);
        invoke(root.path(), &["--check"], 2).await;
    }
    let mut invalid = baseline;
    invalid["K001"]["decision"]["steps"]["columns"]["no"]["finding"] =
        json!("{{word_count:missing}}");
    fixture::entry_file(root.path(), &invalid);
    invoke(root.path(), &["--check"], 2).await;
}
