use super::*;
use sunstack_rules::test_support::controlled_http::ControlledServer;

#[tokio::test]
async fn locator_omission_survives_report_projection_schema_and_fallback_rendering() {
    for mode in [
        "over_limit",
        "exact_limit",
        "low_confidence",
        "no_locator",
        "mechanical",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        std::fs::write(
            root.path().join("evidence/rules/skill.yml"),
            "id: skill\nlanguage: Markdown\nrule: {kind: document}\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("docs/one.md"),
            "First.\nSecond.\nThird.\nFourth.\n",
        )
        .unwrap();
        config["evidence"]["kinds"]["skill"]["fields"] = json!({"body":{"from":"remainder"}});
        config["limits"] = json!({"max_locate_lines":if mode=="over_limit" {3}else{4}});
        let mut finding = json!({"finding":"Read this line.","at":"body"});
        if mode != "no_locator" {
            finding["locate"] = json!({"name":"focus","task":"Choose line","min_probability":0.7});
        }
        fixture::entry_file(
            root.path(),
            &json!({"K001":{"name":"description","evidence":"skill","questions":{"q":{"type":"noul","instructions":{"task":"Check body","check":"body"}}},"decision":{"start":"q","steps":{"q":{"ask":"q","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":finding,"no":"pass"}}}}}),
        );
        if mode == "mechanical" {
            let path = root.path().join("rules.json");
            let mut entries: Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            entries["K001"]["questions"] = json!({});
            entries["K001"]["decision"]["steps"]["q"] = json!({"mechanical_check":{"tool":"contains","field":"body","text":"First"},"yes":finding,"no":"pass"});
            fixture::entry_file(root.path(), &entries);
            config["limits"]["max_locate_lines"] = json!(3);
        }
        let mut server = ControlledServer::start().await;
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        fixture::config_file(root.path(), &config);
        let request = fixture::request(root.path());
        let mut task = tokio::spawn(sunstack_lint::run(
            request,
            CancellationToken::new(),
            Some("SYNTHETIC".into()),
        ));
        let mut bodies = Vec::new();
        let report = loop {
            tokio::select! {biased;result=&mut task=>break result.unwrap(),pending=server.receive()=>{
                let offered=pending.body["questions"].get("focus").is_some();assert_eq!(offered,["exact_limit","low_confidence"].contains(&mode));
                let mut answers=json!({"q":{"type":"noul","noul":0.9}});
                if offered {let probability=if mode=="low_confidence" {0.6}else{0.9};answers["focus"]=json!({"type":"choice","choice":"L4","confidence":0.9,"probabilities":{"L1":(1.0-probability)/3.0,"L2":(1.0-probability)/3.0,"L3":(1.0-probability)/3.0,"L4":probability}});}
                bodies.push(pending.body.clone());pending.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":2,"output_tokens":1}}));
            }}
        };
        assert_eq!(server.finish().await, bodies.len());
        assert_eq!(bodies.len(), usize::from(mode != "mechanical"));
        let value: Value = serde_json::from_str(&render(&report, OutputFormat::Json)).unwrap();
        assert_eq!(
            value["findings"][0]["anchor"],
            if mode == "exact_limit" {
                "resolved"
            } else {
                "default"
            }
        );
        assert_eq!(
            value["findings"][0]["line"],
            if mode == "exact_limit" { 4 } else { 1 }
        );
        let omitted = ["over_limit", "mechanical"].contains(&mode);
        if omitted {
            let expected = json!({"locator":"focus","field":"body","reason":"line_limit","candidate_lines":4,"limit":3});
            assert_eq!(value["results"][0]["locator_omitted"], expected);
            assert_eq!(value["findings"][0]["locator_omitted"], expected);
            assert!(value["results"][0]["answers"].get("focus").is_none());
        } else {
            assert!(value["results"][0].get("locator_omitted").is_none());
        }
        let validator = jsonschema::validator_for(report_schema().as_value()).unwrap();
        assert!(validator.is_valid(&value));
        if omitted {
            for key in ["reason", "candidate_lines", "limit"] {
                let mut bad = value.clone();
                bad["results"][0]["locator_omitted"]
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                assert!(!validator.is_valid(&bad));
            }
        }
        let human = render(&report, OutputFormat::Default);
        assert!(human.contains(if mode == "exact_limit" {
            "docs/one.md:4:"
        } else {
            "docs/one.md:1:"
        }));
        let sarif: Value = serde_json::from_str(&render(&report, OutputFormat::Sarif)).unwrap();
        assert_eq!(
            sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"],
            if mode == "exact_limit" { 4 } else { 1 }
        );
    }
}
