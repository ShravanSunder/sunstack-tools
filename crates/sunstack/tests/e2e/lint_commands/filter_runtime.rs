use super::*;
use sunstack_lint::{LintReport, OutputFormat};

#[tokio::test]
async fn unused_description_cannot_change_a_successful_heading_only_cli_run() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    std::fs::create_dir_all(root.path().join("packages/unused")).unwrap();
    std::fs::write(
        root.path().join("packages/unused/member.md"),
        "# Heading only\n",
    )
    .unwrap();
    config["check"]["files"] = json!(["packages/*/member.md"]);
    config["limits"] = json!({"max_files":1});
    config["evidence"]["kinds"]["heading"] = json!({"inline_rule":"{language: Markdown, rule: {kind: atx_heading}}","fields":{"body":{"from":"first_line"}}});
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["a","b"],"questions":{"filter":{"type":"choice","instructions":{"task":"Classify","check":"description"},"criteria":{"a":"A","b":"B"}}},"decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
    config["decision_model"]["endpoint"] = json!("http://127.0.0.1:1/v1/systemone");
    fixture::config_file(root.path(), &config);
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K001"]["only_when"] = json!({"purpose":["a"]});
    entries["K002"] = json!({"name":"heading","evidence":"heading","decision":{"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"body","text":"Heading"},"yes":"pass","no":"not_applicable"}}}});
    fixture::entry_file(root.path(), &entries);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check descriptions. <!-- K001 -->\n\nCheck headings. <!-- K002 -->\n",
    )
    .unwrap();
    let mut outcomes = Vec::new();
    for present in [false, true] {
        if present {
            std::fs::write(
                root.path().join("packages/unused/about.md"),
                "---\ndescription: Unused\n---\n",
            )
            .unwrap();
        }
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            command(root.path())
                .args(["--all", "--format", "json"])
                .env("OPENROUTER_API_KEY", "SYNTHETIC")
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"]["kind"], "clean");
        assert_eq!(report["filter_answers"], json!([]));
        assert_eq!(report["filter_skips"], json!([]));
        assert_eq!(report["incomplete"], json!([]));
        assert_eq!(report["usage"]["checked_files_acquired"], 1);
        outcomes.push(report["results"].clone());
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn real_cli_filter_answers_skips_status_schema_and_keyless_surfaces() {
    for scenario in [
        "unsure",
        "failed",
        "no_folder",
        "allowed",
        "missing",
        "too_large",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        std::fs::create_dir_all(root.path().join("packages/one/references")).unwrap();
        std::fs::write(
            root.path().join("packages/one/about.md"),
            if scenario == "missing" {
                "---\nname: Missing\n---\n"
            } else {
                "---\ndescription: Purpose one\n---\n"
            },
        )
        .unwrap();
        std::fs::write(
            root.path().join("packages/one/references/member.md"),
            "---\ndescription: Use when valid\n---\n",
        )
        .unwrap();
        config["check"]["files"] = json!(["packages/*/references/*.md"]);
        config["filters"] = json!({"purpose":{"per":if scenario=="no_folder" {"other/{name}"}else{"packages/{name}"},"reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["a","b"],"questions":{"filter":{"type":"choice","instructions":{"task":"Classify","check":"description"},"criteria":{"a":"A","b":"B"}}},"decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
        let mut entries: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap())
                .unwrap();
        entries["K001"]["only_when"] = json!({"purpose":["a"]});
        fixture::entry_file(root.path(), &entries);
        let mut server = controlled::ControlledServer::start().await;
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        if scenario == "too_large" {
            config["limits"] = json!({"request_max_chars":100});
        }
        fixture::config_file(root.path(), &config);
        invoke(root.path(), &["--check"], 0).await;
        let listing = report(root.path(), &["--all", "--evidence"], 0).await;
        assert!(listing.get("pieces").is_some());
        let no_key = report(root.path(), &["--all"], 3).await;
        assert_eq!(no_key["status"]["kind"], "incomplete");
        assert_eq!(no_key["diagnostics"], json!([]));
        assert_eq!(no_key["usage"]["model_requests"], 0);
        assert_eq!(no_key["requests"], json!([]));
        assert_eq!(no_key["findings"], json!([]));
        assert_eq!(
            no_key["file_counts"],
            json!([{
                "file":"packages/one/references/member.md",
                "kinds":{"skill":{"matches":1,"pieces":1,"picked":1,"limited":false}}
            }])
        );
        if scenario == "no_folder" {
            assert_eq!(no_key["usage"]["checked_files_acquired"], 1);
            assert_eq!(no_key["results"], json!([]));
            assert_eq!(no_key["filter_answers"], json!([]));
            assert_eq!(no_key["filter_skips"].as_array().unwrap().len(), 1);
            assert_eq!(no_key["filter_skips"][0]["why"], "no_folder");
            assert_eq!(
                no_key["incomplete"],
                json!([{
                    "rule":"description", "file":"packages/one/references/member.md",
                    "reason":{"kind":"no_filter_folder","filter":"purpose"}
                }])
            );
        } else if scenario == "missing" {
            assert_eq!(no_key["usage"]["checked_files_acquired"], 2);
            assert_eq!(no_key["results"], json!([]));
            assert_eq!(no_key["filter_answers"].as_array().unwrap().len(), 1);
            let answer = &no_key["filter_answers"][0];
            assert_eq!(answer["state"], "incomplete");
            assert_eq!(answer["reason"], json!({"kind":"reads_absent"}));
            assert_eq!(answer["field"], "description");
            assert_eq!(answer["request_ids"], json!([]));
            assert_eq!(no_key["filter_skips"].as_array().unwrap().len(), 1);
            assert_eq!(no_key["filter_skips"][0]["why"], "incomplete");
            assert_eq!(
                no_key["incomplete"],
                json!([{
                    "rule":null, "file":"packages/one",
                    "reason":{"kind":"filter","filter":"purpose","reason":{"kind":"reads_absent"}}
                }])
            );
        } else {
            assert_eq!(no_key["usage"]["checked_files_acquired"], 2);
            assert_eq!(no_key["filter_skips"], json!([]));
            assert_eq!(no_key["filter_answers"].as_array().unwrap().len(), 1);
            let answer = &no_key["filter_answers"][0];
            assert_eq!(answer["filter"], "purpose");
            assert_eq!(answer["folder"], "packages/one");
            assert_eq!(answer["state"], "unavailable");
            assert_eq!(answer["reason"], json!({"kind":"credential_unavailable"}));
            assert_eq!(answer["path"], json!(["filter"]));
            assert_eq!(answer["answers"], json!({}));
            assert_eq!(answer["request_ids"], json!([]));
            assert_eq!(no_key["results"].as_array().unwrap().len(), 1);
            let result = &no_key["results"][0];
            assert_eq!(result["marker"], "K001");
            assert_eq!(result["file"], "packages/one/references/member.md");
            assert_eq!(result["outcome"], "unavailable");
            assert_eq!(
                result["reason"],
                json!({
                    "kind":"filter_unavailable", "filter":"purpose", "folder":"packages/one"
                })
            );
            assert_eq!(result["request_ids"], json!([]));
            let result_id = result["id"].as_u64().unwrap();
            let answer_id = answer["id"].as_u64().unwrap();
            assert!(result_id > 0 && answer_id > 0);
            assert_ne!(result_id, answer_id);
            assert_eq!(
                no_key["piece_evidence"][result_id.to_string()]["piece"]["fields"]["description"]["value"],
                "Use when valid"
            );
            assert_eq!(
                no_key["piece_evidence"][answer_id.to_string()]["piece"]["where"]["file"],
                "packages/one/about.md"
            );
            let incomplete = no_key["incomplete"].as_array().unwrap();
            assert_eq!(incomplete.len(), 2);
            assert!(incomplete.contains(&json!({
                "rule":null,"file":"packages/one",
                "reason":{"kind":"unavailable","reason":{"kind":"credential_unavailable"}}
            })));
            assert!(incomplete.contains(&json!({
                "rule":"description","file":"packages/one/references/member.md",
                "reason":{"kind":"unavailable","reason":{"kind":"filter_unavailable","filter":"purpose","folder":"packages/one"}}
            })));
        }
        let child = command(root.path())
            .args(["--all", "--format", "json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut task = tokio::spawn(child.wait_with_output());
        let mut calls = 0;
        let output = loop {
            tokio::select! {biased;
                output=&mut task=>break output.unwrap().unwrap(),
                pending=server.receive()=>{calls+=1;assert_eq!(pending.body["state"]["description"],"Purpose one");let probability=if scenario=="unsure" {0.3}else{0.9};pending.respond(200,json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":if scenario=="failed" {"undefined"}else{"a"},"confidence":0.99,"probabilities":{"a":probability,"b":1.0-probability}}},"usage":{"input_tokens":2,"output_tokens":1}}));}
            }
        };
        let count = server.finish().await;
        assert_eq!(count, calls);
        assert_eq!(
            calls,
            if ["no_folder", "missing", "too_large"].contains(&scenario) {
                0
            } else {
                1
            }
        );
        let expected = match scenario {
            "unsure" => 5,
            "allowed" => 0,
            _ => 3,
        };
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["file_counts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|count| count["file"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["packages/one/references/member.md"]
        );
        let report: LintReport = serde_json::from_value(value.clone()).unwrap();
        let human = sunstack_lint::render(&report, OutputFormat::Default);
        assert!(
            human.contains(if scenario == "no_folder" {
                "skipped"
            } else {
                "Filters:"
            }),
            "{human}"
        );
        if scenario != "allowed" {
            assert!(value["results"].as_array().unwrap().is_empty());
            assert_eq!(value["filter_skips"].as_array().unwrap().len(), 1);
        }
        if scenario == "no_folder" {
            assert_eq!(value["filter_skips"][0]["why"], "no_folder");
        } else {
            assert_eq!(value["filter_answers"].as_array().unwrap().len(), 1);
        }
        let sarif: Value =
            serde_json::from_str(&sunstack_lint::render(&report, OutputFormat::Sarif)).unwrap();
        assert!(sarif["runs"][0]["results"].as_array().unwrap().is_empty());
    }
}
