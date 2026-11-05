use super::*;
use sunstack_lint::{LintReport, OutputFormat};

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
        config["filters"] = json!({"purpose":{"per":if scenario=="no_folder" {"other/{name}"}else{"packages/{name}"},"reads":{"file":"about.md","kind":"skill","field":"description"},"question":{"type":"choice","instructions":{"task":"Classify"},"criteria":{"a":"A","b":"B"}},"min_probability":0.7}});
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
        let no_key = report(root.path(), &["--all"], 2).await;
        assert_eq!(no_key["usage"]["checked_files_acquired"], 0);
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
        let report: LintReport = serde_json::from_value(value.clone()).unwrap();
        let human = sunstack_lint::render(&report, OutputFormat::Default);
        assert!(
            human.contains(if scenario == "no_folder" {
                "Skipped"
            } else {
                "Filter purpose"
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
