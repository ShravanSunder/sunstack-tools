use super::case_runs::commit_tree;
use super::*;

#[cfg(unix)]
#[tokio::test]
async fn case_rule_directory_replacement_unlinks_historical_aliases_without_touching_their_target()
{
    use std::os::unix::fs::symlink;
    for parent_alias in [false, true] {
        let root = limit_fixture(
            json!({"body":{"from":"remainder"}}),
            mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
        );
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(outside.path().join("rules")).unwrap();
        let sentinel = outside.path().join("rules/keep.txt");
        std::fs::write(&sentinel, "Outside snapshot: preserve me.").unwrap();
        let pieces = std::fs::read(root.path().join("evidence/rules/pieces.yml")).unwrap();
        let alias = root.path().join(if parent_alias {
            "evidence"
        } else {
            "evidence/rules"
        });
        std::fs::remove_dir_all(&alias).unwrap();
        let outside_target = if parent_alias {
            outside.path().to_path_buf()
        } else {
            outside.path().join("rules")
        };
        symlink(&outside_target, &alias).unwrap();
        let source = "Historical source.\n";
        std::fs::write(root.path().join("docs/one.md"), source).unwrap();
        let revision = commit_tree(root.path());
        std::fs::remove_file(&alias).unwrap();
        std::fs::create_dir_all(root.path().join("evidence/rules")).unwrap();
        std::fs::write(
            root.path().join("evidence/sgconfig.yml"),
            "ruleDirs: [rules]\n",
        )
        .unwrap();
        std::fs::write(root.path().join("evidence/rules/pieces.yml"), pieces).unwrap();
        save(
            &root.path().join("cases.json"),
            &json!({"cases":[{"id":"alias","kind":"rule","rule":"document_standard","file":"docs/one.md","revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(source.as_bytes()),"expected":"warn","set":"development","label_from":"blind-agents"}],"bars":[]}),
        );

        let report = invoke(root.path(), &["cases", "cases.json"], 0).await;

        assert_eq!(report["counts"], json!({"right":1}));
        assert_eq!(
            std::fs::read(&sentinel).unwrap(),
            b"Outside snapshot: preserve me."
        );
    }
}

#[tokio::test]
async fn cases_replace_only_current_rule_directories_and_keep_historical_skill_text() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
    );
    let skill = root.path().join("skills/project");
    std::fs::create_dir_all(skill.join("rules")).unwrap();
    std::fs::rename(root.path().join("evidence"), skill.join("rules/evidence")).unwrap();
    std::fs::create_dir(skill.join("references")).unwrap();
    let historical = "Historical long source.\n";
    let reference = "skills/project/references/standards.md";
    std::fs::write(root.path().join("docs/one.md"), historical).unwrap();
    std::fs::write(root.path().join(reference), historical).unwrap();
    let rules = skill.join("rules/evidence/rules");
    std::fs::write(rules.join("obsolete.yml"), "not a rule mapping\n").unwrap();
    std::fs::write(
        rules.join("retired.yml"),
        "id: retired_unused\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["evidence"]["ast_grep"] = json!("skills/project/rules/evidence/sgconfig.yml");
    config["check"]["files"] = json!(["docs/*.md", "skills/**/*.md"]);
    save(&root.path().join("skill-rules.config.json"), &config);
    let revision = commit_tree(root.path());
    std::fs::remove_file(rules.join("obsolete.yml")).unwrap();
    std::fs::remove_file(rules.join("retired.yml")).unwrap();
    std::fs::write(root.path().join(reference), "x").unwrap();
    let cases = [("source", "docs/one.md"), ("historical-standard", reference)].map(|(id,file)| json!({"id":id,"kind":"rule","rule":"document_standard","file":file,"revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(historical.as_bytes()),"expected":"warn","set":"development","label_from":"blind-agents"}));
    save(
        &root.path().join("cases.json"),
        &json!({"cases":cases,"bars":[]}),
    );
    invoke(root.path(), &["--check"], 0).await;

    let report = invoke(root.path(), &["cases", "cases.json"], 0).await;

    assert_eq!(report["counts"], json!({"right":2}));
    assert_eq!(report["runs"][0]["report"]["usage"]["model_requests"], 0);
    assert!(
        report["runs"][0]["report"]["rulebook_files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| {
                !file["path"].as_str().unwrap().ends_with("obsolete.yml")
                    && !file["path"].as_str().unwrap().ends_with("retired.yml")
            })
    );
    assert_eq!(std::fs::read(root.path().join(reference)).unwrap(), b"x");
}

#[tokio::test]
async fn cases_refuse_rule_directories_that_cover_the_repository_or_checked_skill_text() {
    for root_directory in [true, false] {
        let root = limit_fixture(
            json!({"body":{"from":"remainder"}}),
            mechanical_graph(json!({"tool":"max_chars","field":"body","max":1})),
        );
        let source = "Historical long source.\n";
        std::fs::write(root.path().join("docs/one.md"), source).unwrap();
        let destination = if root_directory {
            "pieces.yml"
        } else {
            "docs/pieces.yml"
        };
        std::fs::rename(
            root.path().join("evidence/rules/pieces.yml"),
            root.path().join(destination),
        )
        .unwrap();
        std::fs::remove_file(root.path().join("evidence/sgconfig.yml")).unwrap();
        save(
            &root.path().join("sgconfig.json"),
            &json!({"ruleDirs":[if root_directory { "." } else { "docs" }]}),
        );
        let mut config = read(&root.path().join("skill-rules.config.json"));
        config["evidence"]["ast_grep"] = json!("sgconfig.json");
        save(&root.path().join("skill-rules.config.json"), &config);
        let revision = commit_tree(root.path());
        save(
            &root.path().join("cases.json"),
            &json!({"cases":[{"id":"protected","kind":"rule","rule":"document_standard","file":"docs/one.md","revision":revision,"lines":[1,1],"text_sha256":sunstack_evidence::content_digest(source.as_bytes()),"expected":"warn","set":"development","label_from":"blind-agents"}],"bars":[]}),
        );
        invoke(root.path(), &["--check"], 0).await;

        let report = invoke(root.path(), &["cases", "cases.json"], 1).await;

        let reason = report["errors"][&revision].as_str().unwrap();
        assert!(reason.contains("rule directory"), "{reason}");
        assert!(
            reason.contains(if root_directory {
                "repository root"
            } else {
                "checked file"
            }),
            "{reason}"
        );
        assert_eq!(report["counts"], json!({"incomplete":1}));
        assert_eq!(report["cases"][0]["outcome"]["reason"], reason);
        assert_eq!(report["cases"][0]["result_id"], Value::Null);
        assert_eq!(report["runs"], json!([]));
        assert_eq!(
            std::fs::read(root.path().join("docs/one.md")).unwrap(),
            source.as_bytes()
        );
    }
}
