use super::*;

#[tokio::test]
async fn readme_captures_chain_collects_blocks_and_cap_prevents_a_pass() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["docs/requirements.md"]);
    let readme = include_str!("../../../README.md");
    let fields: Value = serde_json::from_str(
        readme
            .split_once("<!-- example: captures-attached-blocks -->\n```json\n")
            .unwrap()
            .1
            .split_once("\n```")
            .unwrap()
            .0,
    )
    .unwrap();
    config["evidence"]["kinds"] = json!({"subject":{"rule":"subject","fields":fields}});
    std::fs::write(root.path().join("evidence/rules/attached.yml"),"id: subject\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^U2'}\n---\nid: obligation-row\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^R'}\n---\nid: realization-lead\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^\\*\\*D'}\n---\nid: block-boundary\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^\\*\\*D'}\n").unwrap();
    std::fs::write(
        root.path().join("docs/requirements.md"),
        "U2, U3 delivery\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("docs/specification.md"),
        "R2 covers U2\n\nR10 covers U3\n\nR99 covers U30\n",
    )
    .unwrap();
    std::fs::write(root.path().join("docs/program-design.md"),"**D10b R2, R10.** classifier\n\n```rust\nenum AttemptOutcome { NoResponse }\n```\n\n- rule one\n- rule two\n\nClosing paragraph.\n\n**D11 R99.** unrelated\n").unwrap();
    fixture::entry_file(
        root.path(),
        &json!({"K001":{"name":"complete_blocks","evidence":"subject","severity":"warn","decision":{"start":"complete","steps":{"complete":{"mechanical_check":{"tool":"complete","field":"realizations"},"yes":"pass","no":"uncertain"}}}}}),
    );
    fixture::config_file(root.path(), &config);

    invoke(root.path(), &["--check"], 0).await;
    let full = report(root.path(), &["--all"], 0).await;

    assert_eq!(full["results"][0]["outcome"], "pass", "{full}");
    let piece = &full["piece_evidence"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["piece"];
    assert_eq!(
        piece["fields"]["requirement_ids"]["values"],
        json!(["U2", "U3"])
    );
    assert_eq!(
        piece["fields"]["obligation_ids"]["values"],
        json!(["R2", "R10"])
    );
    let realization = piece["fields"]["realizations"]["values"][0]
        .as_str()
        .unwrap();
    assert!(
        realization.contains("NoResponse")
            && realization.contains("rule two")
            && realization.contains("Closing paragraph")
    );
    assert!(!realization.contains("D11"));
    config["evidence"]["kinds"]["subject"]["fields"]["realizations"]["attach"]["max_siblings"] =
        json!(1);
    fixture::config_file(root.path(), &config);
    let partial = report(root.path(), &["--all"], 5).await;
    assert_eq!(partial["results"][0]["outcome"], "uncertain", "{partial}");
    assert_eq!(
        partial["uncertain"].as_array().unwrap().len(),
        1,
        "{partial}"
    );
    for (field, value) in [
        ("max_siblings", json!(0)),
        ("max_siblings", json!(-1)),
        ("max_siblings", json!(1.5)),
        ("until", json!("undefined")),
        ("until", json!(null)),
    ] {
        config["evidence"]["kinds"]["subject"]["fields"]["realizations"]["attach"] =
            json!({"until":"block-boundary","max_siblings":12});
        config["evidence"]["kinds"]["subject"]["fields"]["realizations"]["attach"][field] = value;
        fixture::config_file(root.path(), &config);
        invoke(root.path(), &["--check"], 2).await;
    }
    config["evidence"]["kinds"]["subject"]["fields"]["realizations"]["attach"] =
        json!({"until":"block-boundary","max_siblings":12});
    config["evidence"]["kinds"]["subject"]["fields"]["requirement_ids"]["regex"] = json!("(");
    fixture::config_file(root.path(), &config);
    invoke(root.path(), &["--check"], 2).await;
}
