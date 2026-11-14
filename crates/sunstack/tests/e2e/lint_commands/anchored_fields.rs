use super::*;

#[tokio::test]
async fn island_tables_read_enclosing_html_disclosures_and_heading_context() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["docs/*.html"]);
    config["evidence"]["html"] =
        json!({"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]});
    config["evidence"]["kinds"] = json!({
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
        "disclosure":{"rule":"closed","fields":{}},
        "table":{"rule":"table","fields":{
            "placement":{"from":"enclosing","rule":"closed","text":"first_line"},
            "container":{"from":"enclosing","kind":"disclosure","text":"full"},
            "heading":{"from":"enclosing","kind":"outline","text":"heading"},
            "prefix":{"from":"section_prefix","kind":"outline"}
        }}
    });
    fixture::config_file(root.path(), &config);
    std::fs::write(root.path().join("evidence/rules/islands.yml"), "id: closed\nlanguage: Html\nrule:\n  kind: element\n  regex: '^<details(?:\\s|>)'\n  not:\n    has:\n      kind: start_tag\n      has: {kind: attribute, regex: '^open(?:\\s|=|$)'}\n---\nid: table\nlanguage: Markdown\nrule: {kind: pipe_table}\n").unwrap();
    let page = "<h2>Verification</h2>\n<details>\n<summary>Proof rows</summary>\n<script type='text/markdown'>\n| U | Proof |\n| - | ----- |\n| U1 | Shows work |\n</script>\n</details>\n";
    std::fs::write(root.path().join("docs/page.html"), page).unwrap();
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"placement","evidence":"table","questions":{"placed":{"type":"noul","instructions":{"check":"placement","context":["container","heading","prefix"],"task":"Is the table in a closed disclosure under Verification?"}}},"decision":{"start":"ask","steps":{"ask":{"ask":"placed","yes_at_or_above":0.8,"no_at_or_below":0.3,"yes":"pass","no":{"finding":"Move detail."},"uncertain":"uncertain"}}}}}),
    );

    invoke(root.path(), &["--check"], 0).await;
    let saved = root.path().join("prepared.json");
    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;
    let prepared: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    let state = &prepared["requests"][0]["request"]["state"];
    assert_eq!(state["placement"], json!(["<details>"]));
    assert_eq!(
        state["container"],
        json!([page
            .split_once("<details>")
            .unwrap()
            .1
            .split_once("</details>")
            .map(|(body, _)| format!("<details>{body}</details>"))
            .unwrap()])
    );
    assert_eq!(state["heading"], json!(["Verification"]));
    assert_eq!(
        state["prefix"],
        "<h2>Verification</h2>\n<details>\n<summary>Proof rows</summary>\n<script type='text/markdown'>\n"
    );
    assert_eq!(prepared["usage"]["model_requests"], 0);
    std::fs::write(
        root.path().join("docs/page.html"),
        page.replace("<details>", "<details open>"),
    )
    .unwrap();
    let saved = root.path().join("open.json");
    invoke(
        root.path(),
        &["--all", "--prepare", saved.to_str().unwrap()],
        6,
    )
    .await;
    let prepared: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
    let state = &prepared["requests"][0]["request"]["state"];
    assert_eq!(state["placement"], json!([]));
    assert_eq!(state["container"], json!([]));
    assert_eq!(state["heading"], json!(["Verification"]));
}
