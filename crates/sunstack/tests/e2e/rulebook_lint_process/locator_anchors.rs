use super::*;
use std::collections::BTreeSet;

fn located_fixture(case: &str) -> (tempfile::TempDir, String) {
    let finding = json!({"finding":"Read this line.","at":if ["enclosing", "outside"].contains(&case) { "enclosing" } else { "body" }});
    let decision = if case == "mechanical" {
        json!({"start":"check","steps":{"check":{"mechanical_check":{"tool":"contains","field":"heading","text":"Subject"},"yes":finding,"no":"pass"}}})
    } else {
        json!({"start":"gate","steps":{"gate":{"ask":"gate","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":finding,"no":"pass"}}})
    };
    let root = limit_fixture(
        json!({"heading":{"from":"first_line"},"body":{"from":"remainder"},"enclosing":{"from":"enclosing","rule":"document","text":"full"}}),
        decision,
    );
    std::fs::write(root.path().join("evidence/rules/pieces.yml"), "id: subject\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^## Subject', stopBy: neighbor}}\n---\nid: document\nlanguage: Markdown\nrule: {kind: document}\n").unwrap();
    let mut lines = vec![
        "# Root".to_owned(),
        "".into(),
        "Preface.".into(),
        "".into(),
        "".into(),
        "## Subject".into(),
        "".into(),
    ];
    lines.extend((8..=19).map(|line| {
        if line == 17 {
            "Target line.".into()
        } else {
            format!("Content line {line}.")
        }
    }));
    lines.extend(["## After".into(), "Outside.".into()]);
    let text = format!("{}\n", lines.join("\n"));
    std::fs::write(root.path().join("docs/one.md"), &text).unwrap();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]["review"]["rule"] = json!("subject");
    if case == "outside" {
        config["evidence"]["kinds"]["review"]["fields"]["enclosing"]
            .as_object_mut()
            .unwrap()
            .remove("text");
    }
    if case == "limit" {
        config["limits"] = json!({"max_locate_lines":12});
    }
    if case == "size" {
        config["limits"] = json!({"request_max_chars":2000});
    }
    save(&path, &config);
    let path = root.path().join("rules.json");
    let mut entries = read(&path);
    entries["K001"]["questions"] =
        json!({"gate":{"type":"noul","instructions":{"check":"body","task":"Does it comply?"}}});
    if case == "questions" {
        for position in 0..31 {
            entries["K001"]["questions"][format!("unused_{position}")] =
                json!({"type":"noul","instructions":{"check":"body","task":"Unused."}});
        }
    }
    if case != "plain" {
        let step = if case == "mechanical" {
            "check"
        } else {
            "gate"
        };
        entries["K001"]["decision"]["steps"][step]["yes"]["locate"] = json!({"name":"focus_line","task":if case == "size" { "choose ".repeat(1000) } else { "Choose the line that needs attention.".into() },"min_probability":0.7});
    }
    save(&path, &entries);
    (root, text)
}

async fn observe(root: &Path, case: &str) -> (Value, Vec<Value>) {
    let mut server = controlled::ControlledServer::start().await;
    let path = root.join("skill-rules.config.json");
    let mut config = read(&path);
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&path, &config);
    let mut operation = tokio::spawn(
        command(root)
            .args(["--all", "--format", "json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_RULEBOOK_KEY")
            .output(),
    );
    let mut bodies = Vec::new();
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop { tokio::select! {
            output = &mut operation => break output.unwrap().unwrap(),
            request = server.receive() => {
                let mut answers = serde_json::Map::new();
                for (name, question) in request.body["questions"].as_object().unwrap() {
                    if question["type"] == "choice" {
                        if case == "missing" { continue; }
                        let selected = if case == "invalid" { "L999" } else { "L17" };
                        let probability = if case == "low" { 0.6 } else { 0.8 };
                        let probabilities = question["criteria"].as_object().unwrap().keys().map(|label| (label.clone(), json!(if label == selected { probability } else { 0.01 }))).collect::<serde_json::Map<_, _>>();
                        answers.insert(name.clone(), json!({"type":"choice","choice":selected,"confidence":0.99,"probabilities":probabilities}));
                    } else { answers.insert(name.clone(), json!({"type":"noul","noul":0.91})); }
                }
                bodies.push(request.body.clone());
                request.respond(200, json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5}}));
            }
        }}
    }).await.unwrap();
    assert_eq!(server.finish().await, bodies.len());
    assert_eq!(
        output.status.code(),
        Some(if ["invalid", "missing"].contains(&case) {
            3
        } else {
            1
        }),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (serde_json::from_slice(&output.stdout).unwrap(), bodies)
}

#[tokio::test]
async fn locator_anchors_follow_requested_probabilities_and_leave_the_field_span_unchanged() {
    for case in [
        "plain",
        "resolved",
        "low",
        "enclosing",
        "outside",
        "limit",
        "questions",
        "size",
        "mechanical",
        "invalid",
        "missing",
    ] {
        let (root, text) = located_fixture(case);
        let (report, bodies) = observe(root.path(), case).await;
        let sent = ![
            "plain",
            "limit",
            "questions",
            "size",
            "mechanical",
            "outside",
        ]
        .contains(&case);
        assert_eq!(bodies.len(), usize::from(case != "mechanical"), "{case}");
        if let Some(request) = bodies.first() {
            if sent {
                let question = &request["questions"]["focus_line"];
                assert_eq!(question["type"], "choice");
                let field = if case == "enclosing" {
                    "enclosing"
                } else {
                    "body"
                };
                assert_eq!(question["instructions"]["check"], field);
                assert_eq!(question["criteria"]["L17"], "Target line.");
                let offered = question["criteria"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                assert_eq!(
                    offered,
                    ((if case == "enclosing" { 6 } else { 7 })..=19)
                        .map(|line| format!("L{line}"))
                        .collect()
                );
            } else {
                assert!(request["questions"].get("focus_line").is_none());
            }
            if case == "questions" {
                assert_eq!(request["questions"].as_object().unwrap().len(), 32);
            }
        }
        if ["invalid", "missing"].contains(&case) {
            assert_eq!(
                report["incomplete"][0]["reason"]["kind"],
                "invalid_response"
            );
            assert_eq!(report["findings"], json!([]));
            assert_eq!(report["results"][0]["path"], json!(["gate"]));
            assert!(report["results"][0].get("answers").is_none());
            continue;
        }
        let resolved = ["resolved", "enclosing"].contains(&case);
        let finding = &report["findings"][0];
        let default_line = if case == "outside" { 1 } else { 7 };
        assert_eq!(
            finding["line"],
            if resolved { 17 } else { default_line },
            "{case}"
        );
        assert_eq!(
            finding["anchor"],
            if resolved { "resolved" } else { "default" },
            "{case}"
        );
        assert_eq!(
            finding["span"]["byte_start"],
            if ["enclosing", "outside"].contains(&case) {
                0
            } else {
                text.find("## Subject\n").unwrap() + "## Subject\n".len()
            }
        );
        assert_eq!(
            finding["span"]["byte_end"],
            if case == "outside" {
                "# Root\n".len()
            } else if case == "enclosing" {
                text.len()
            } else {
                text.find("## After").unwrap()
            }
        );
        assert_eq!(
            finding["basis"],
            if case == "mechanical" {
                "mechanical"
            } else {
                "judged"
            }
        );
        assert_eq!(
            report["results"][0]["answers"].get("focus_line").is_some(),
            sent
        );
        let typed: sunstack_lint::LintReport = serde_json::from_value(report).unwrap();
        let sarif: Value = serde_json::from_str(&sunstack_lint::render(
            &typed,
            sunstack_lint::OutputFormat::Sarif,
        ))
        .unwrap();
        let region = &sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        if resolved {
            assert_eq!(region, &json!({"startLine":17,"endLine":17}));
        } else {
            assert_eq!(region["startLine"], default_line);
        }
    }
}

#[tokio::test]
async fn locator_admission_names_the_entry_route_and_refused_field() {
    for case in [
        "without_at",
        "question_name",
        "locator_name",
        "zero",
        "negative",
        "above_one",
        "unknown_key",
    ] {
        let (root, _) = located_fixture("resolved");
        let path = root.path().join("rules.json");
        let mut entries = read(&path);
        let finding = &mut entries["K001"]["decision"]["steps"]["gate"]["yes"];
        match case {
            "without_at" => {
                finding.as_object_mut().unwrap().remove("at");
            }
            "question_name" => finding["locate"]["name"] = json!("gate"),
            "locator_name" => {
                let duplicate = finding.clone();
                entries["K001"]["decision"]["steps"]["gate"]["no"] = duplicate;
            }
            "zero" => finding["locate"]["min_probability"] = json!(0.0),
            "negative" => finding["locate"]["min_probability"] = json!(-0.1),
            "above_one" => finding["locate"]["min_probability"] = json!(1.01),
            "unknown_key" => finding["locate"]["unknown"] = json!(true),
            _ => unreachable!(),
        }
        save(&path, &entries);
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            command(root.path()).arg("--check").output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(output.status.code(), Some(2), "{case}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(text.contains("rules.json"), "{case}: {text}");
        if case != "unknown_key" {
            for part in ["document_standard", "gate", "locate"] {
                assert!(text.contains(part), "{case} missing {part}: {text}");
            }
            assert!(
                text.contains(if case == "without_at" {
                    "at"
                } else if ["zero", "negative", "above_one"].contains(&case) {
                    "min_probability"
                } else {
                    "name"
                }),
                "{text}"
            );
        }
    }
    let (root, _) = located_fixture("resolved");
    let path = root.path().join("rules.json");
    let mut entries = read(&path);
    entries["K001"]["decision"]["steps"]["gate"]["yes"]["locate"]["min_probability"] = json!(1.0);
    save(&path, &entries);
    assert_eq!(
        invoke(root.path(), &["--check"], 0)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
