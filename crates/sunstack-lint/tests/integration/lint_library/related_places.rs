use super::*;

#[tokio::test]
async fn related_places_preserve_standard_target_and_heading_order_and_only_read_ranges() {
    for case in ["standard", "path", "skill", "missing", "zero", "empty"] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        std::fs::write(root.path().join("evidence/rules/skill.yml"), "id: review\nlanguage: Markdown\nfiles: ['docs/*.md']\nrule: {kind: document}\n---\nid: reference\nlanguage: Markdown\nfiles: ['references/**']\nrule: {kind: document}\n").unwrap();
        let target = if case == "skill" {
            "reference"
        } else if case == "missing" {
            "../missing.md"
        } else {
            "../target.md"
        };
        std::fs::write(
            root.path().join("docs/one.md"),
            format!("# Checked\n\nRead `{target}`.\n"),
        )
        .unwrap();
        std::fs::write(
            root.path().join("target.md"),
            if case == "empty" {
                ""
            } else {
                "# Reference\n\nRead me.\n"
            },
        )
        .unwrap();
        let mut target_field = json!({"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":if case == "zero" { 0 } else { 100 }});
        if case == "skill" {
            std::fs::create_dir_all(root.path().join("references/reference")).unwrap();
            std::fs::write(
                root.path().join("references/reference/SKILL.md"),
                "# Reference\n\nRead me.\n",
            )
            .unwrap();
            target_field["resolve"] = json!("skill");
            target_field["as_kind"] = json!("reference");
        }
        config["check"]["files"] = json!(["docs/*.md", "references/**/SKILL.md"]);
        config["evidence"]["kinds"] = json!({"review":{"rule":"review","fields":{"heading":{"from":"first_line"},"targets":target_field}},"reference":{"rule":"reference","fields":{"heading":{"from":"first_line"}}}});
        fixture::config_file(root.path(), &config);
        let mut entries = json!({"K001":{"name":"standard","evidence":"review","severity":"error","decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"heading","text":"required"},"yes":"pass","no":{"finding":"Read the standard."}}}}}});
        if case != "standard" {
            entries["K001"]["related"] = json!(["targets", "heading"]);
        }
        fixture::entry_file(root.path(), &entries);
        let report = run(fixture::request(root.path()), CancellationToken::new()).await;
        assert_eq!(report.status, LintStatus::Findings, "{case}: {report:?}");
        assert_eq!(report.findings.len(), 1);
        let standard_file = root.path().canonicalize().unwrap().join("rulebook.md");
        let mut expected =
            vec![json!({"label":"standard","file":standard_file,"first_line":1,"last_line":1})];
        if case == "path" {
            expected
                .push(json!({"label":"targets","file":"target.md","first_line":1,"last_line":3}));
        }
        if case != "standard" {
            expected
                .push(json!({"label":"heading","file":"docs/one.md","first_line":1,"last_line":1}));
        }
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["findings"][0]["related"], json!(expected), "{case}");
        let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
        let expected_sarif = expected.iter().enumerate().map(|(position, place)| json!({"id":position+1,"message":{"text":place["label"]},"physicalLocation":{"artifactLocation":{"uri":place["file"]},"region":{"startLine":place["first_line"],"endLine":place["last_line"]}}})).collect::<Vec<_>>();
        assert_eq!(
            sarif["runs"][0]["results"][0]["relatedLocations"],
            json!(expected_sarif)
        );
        for format in [OutputFormat::Default, OutputFormat::Github] {
            let output = render(&report, format);
            for place in &expected {
                assert!(
                    output.contains(&format!(
                        "  {}: {}:{}",
                        place["label"].as_str().unwrap(),
                        place["file"].as_str().unwrap(),
                        place["first_line"]
                    )),
                    "{case} {format:?}: {output}"
                );
            }
        }
    }
}
