use super::*;

#[tokio::test]
async fn admission_details_survive_ordinary_and_case_preparation_show_and_success_identity() {
    for (setting, dimension) in [
        ("max_input_tokens_per_request", "input_tokens"),
        ("request_max_chars", "request_chars"),
    ] {
        let root = limit_fixture(
            json!({"body":{"from":"remainder"}}),
            json!({"start":"ask","steps":{"ask":{
                "ask":"first","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Missing explanation."},"uncertain":"uncertain"
            }}}),
        );
        let source = "A precise source with é and a literal \\\" quote.\n";
        std::fs::write(root.path().join("docs/one.md"), source).unwrap();
        let path = root.path().join("rules.json");
        let mut rules = read(&path);
        rules["K001"]["questions"] = json!({
            "first":{"type":"noul","instructions":{"check":"body","task":"Is the first property present?"}},
            "second":{"type":"noul","instructions":{"check":"body","task":"Is the second property present?"}}
        });
        save(&path, &rules);
        let revision = super::case_runs::commit_tree(root.path());
        save(
            &root.path().join("cases.json"),
            &json!({"cases":[{
            "id":"admission","kind":"rule","rule":"document_standard","file":"docs/one.md","revision":revision,"lines":[1,1],
            "text_sha256":sunstack_evidence::content_digest(source.as_bytes()),"expected":"pass","set":"development","label_from":"blind-agents"
        }],"bars":[]}),
        );
        let config_path = root.path().join("skill-rules.config.json");
        let baseline = read(&config_path);
        let mut config = baseline.clone();
        config["limits"] = json!({setting:1});
        save(&config_path, &config);

        let ordinary = invoke(
            root.path(),
            &["--all", "--prepare", "ordinary.json", "--format", "json"],
            3,
        )
        .await;
        let cases = invoke(
            root.path(),
            &["--prepare", "cases-report.json", "cases", "cases.json"],
            3,
        )
        .await;
        let human = command(root.path())
            .args(["--all", "--prepare", "human.json"])
            .output()
            .await
            .unwrap();
        assert_eq!(human.status.code(), Some(3));
        let summary = String::from_utf8(human.stdout).unwrap();
        assert!(
            summary.contains(if dimension == "input_tokens" {
                "input tokens"
            } else {
                "request characters"
            }),
            "{summary}"
        );
        assert!(summary.contains("cap 1"), "{summary}");
        for (saved, report, selector) in [
            ("ordinary.json", &ordinary, "1"),
            (
                "cases-report.json",
                &cases["runs"][0]["report"],
                "admission",
            ),
        ] {
            let request = &report["requests"][0];
            let detail =
                &request["reason"]["reason"]["admission_refused"]["diagnostics"]["items"][0];
            assert_eq!(
                detail["code"],
                if dimension == "input_tokens" {
                    "context_budget_exceeded"
                } else {
                    "request_too_large"
                },
                "{report}"
            );
            assert_eq!(detail["admission"]["dimension"], dimension);
            assert_eq!(detail["admission"]["cap"], 1);
            let body = serde_json::to_string(&json!({"model":"jev-latest","state":request["request"]["state"],"questions":request["request"]["questions"]})).unwrap();
            let expected = if dimension == "input_tokens" {
                body.len()
            } else {
                body.chars().count()
            };
            assert_eq!(detail["admission"]["measurement"]["observed"], expected);
            assert_eq!(
                detail["admission"]["measurement"]["basis"],
                if dimension == "input_tokens" {
                    "estimated"
                } else {
                    "exact"
                }
            );
            let typed: sunstack_decisions::StructuredJudgmentRequest =
                serde_json::from_value(request["request"].clone()).unwrap();
            assert_eq!(request["request"]["state"]["body"], source);
            assert_eq!(
                request["input_sha256"],
                sunstack_evidence::content_digest(&serde_json::to_vec(&typed).unwrap())
            );
            assert_eq!(
                report["results"][0]["gap"]["admission_refused"]["diagnostics"]["items"][0],
                *detail
            );
            assert_eq!(
                report["piece_evidence"]["1"]["piece"]["where"]["span"]["source"]["content_identity"],
                sunstack_evidence::content_digest(source.as_bytes())
            );
            assert_eq!(report["usage"]["model_requests"], 0);
            let shown = invoke(root.path(), &["show", saved, selector], 0).await;
            assert!(shown.to_string().contains(dimension));
        }
        save(&config_path, &baseline);
        let success = invoke(
            root.path(),
            &["--all", "--prepare", "success.json", "--format", "json"],
            6,
        )
        .await;
        assert_eq!(
            success["requests"][0]["input_sha256"],
            ordinary["requests"][0]["input_sha256"]
        );
        assert_eq!(
            success["requests"][0]["request"],
            ordinary["requests"][0]["request"]
        );
        assert_eq!(success["usage"]["model_requests"], 0);
    }
}
