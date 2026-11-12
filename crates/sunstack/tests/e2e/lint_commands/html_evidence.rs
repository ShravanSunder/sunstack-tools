use super::*;

#[tokio::test]
async fn html_cross_document_checks_and_prepared_asking_use_distinct_raw_and_text_views() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["docs/*.html"]);
    config["evidence"]["kinds"] = json!({"paragraph":{"rule":"paragraph","fields":{
        "body":{"from":"remainder"},
        "identifier":{"from":"capture","regex":"id=\"(R[0-9]+)\""},
        "coverage":{"from":"document","path":"../coverage.html","rule":"whole"},
        "links":{"from":"target","regex":"href=\"([^\"]+)\"","resolve":"path"}
    }}});
    fixture::config_file(root.path(), &config);
    std::fs::write(root.path().join("evidence/rules/html.yml"),"id: paragraph\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n---\nid: whole\nlanguage: Html\nrule: {kind: document}\n").unwrap();
    let raw = "<p id=\"R1\"><a href=\"../coverage.html#covered\">R1</a> <b>Café &amp; 文</b></p>";
    std::fs::write(root.path().join("docs/source.html"), raw).unwrap();
    std::fs::write(
        root.path().join("coverage.html"),
        "<p id=\"covered\">R10 only</p>",
    )
    .unwrap();
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"coverage","evidence":"paragraph","severity":"error","decision":{"start":"anchor","steps":{
            "anchor":{"mechanical_check":{"tool":"anchor_exists","field":"links"},"yes":"coverage","no":{"finding":"Dangling anchor."}},
            "coverage":{"mechanical_check":{"tool":"contains_field","field":"coverage","needle":"identifier"},"yes":"raw","no":{"finding":"Missing coverage."}},
            "raw":{"mechanical_check":{"tool":"regex","field":"body","pattern":"<b>"},"yes":"pass","no":{"finding":"Mechanical text lost source markup."}}
        }}}}),
    );
    invoke(root.path(), &["--check"], 0).await;
    let prepared = root.path().join("prepared.json");
    invoke(
        root.path(),
        &["--all", "--prepare", prepared.to_str().unwrap()],
        1,
    )
    .await;
    let report: Value = serde_json::from_slice(&std::fs::read(&prepared).unwrap()).unwrap();
    assert_eq!(report["findings"][0]["message"], "Missing coverage.");
    assert_eq!(report["findings"][0]["file"], "docs/source.html");
    std::fs::write(
        root.path().join("coverage.html"),
        "<p id=\"covered\">(R1)</p>",
    )
    .unwrap();
    invoke(
        root.path(),
        &["--all", "--prepare", prepared.to_str().unwrap()],
        6,
    )
    .await;
    std::fs::remove_file(root.path().join("coverage.html")).unwrap();
    invoke(
        root.path(),
        &["--all", "--prepare", prepared.to_str().unwrap()],
        1,
    )
    .await;
    std::fs::write(
        root.path().join("coverage.html"),
        "<p id=\"covered\">R1</p>",
    )
    .unwrap();
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"asking","evidence":"paragraph","severity":"error","questions":{"clear":{"type":"noul","instructions":{"check":"body","task":"Is the text clear?"}}},"decision":{"start":"ask","steps":{"ask":{"ask":"clear","yes_at_or_above":0.8,"no_at_or_below":0.3,"yes":"pass","no":{"finding":"Unclear."},"uncertain":"uncertain"}}}}}),
    );
    invoke(
        root.path(),
        &["--all", "--prepare", prepared.to_str().unwrap()],
        6,
    )
    .await;
    let report: Value = serde_json::from_slice(&std::fs::read(prepared).unwrap()).unwrap();
    assert_eq!(report["requests"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["requests"][0]["request"]["state"]["body"],
        "R1 Café & 文"
    );
    assert_eq!(report["requests"][0]["status"], "prepared");
    assert_eq!(report["usage"]["model_requests"], 0);
    assert!(report["piece_evidence"].to_string().contains("<b>"));
}
