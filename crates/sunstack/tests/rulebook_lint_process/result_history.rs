use super::*;

#[tokio::test]
async fn judged_pass_drop_and_incomplete_keep_the_walks_answers_not_the_requests() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"},"description":{"from":"yaml","key":"description"}}),
        json!({"start":"gate","steps":{
            "gate":{"ask":"gate","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"check","no":"not_applicable"},
            "check":{"mechanical_check":{"tool":"contains","field":"description","text":"finding"},"yes":{"finding":"Read the standard."},"no":"pass"}
        }}),
    );
    let mut entries = read(&root.path().join("rules.json"));
    entries["K001"]["questions"] = json!({
        "gate":{"type":"noul","instructions":{"check":"body","task":"Does it apply?"}},
        "unused":{"type":"noul","instructions":{"check":"body","task":"Not read on this path."}}
    });
    save(&root.path().join("rules.json"), &entries);
    for (file, description) in [
        ("a-drop.md", "pass"),
        ("b-pass.md", "pass"),
        ("c-finding.md", "finding"),
        ("d-incomplete.md", "{key: value}"),
    ] {
        std::fs::write(
            root.path().join("docs").join(file),
            format!("---\ndescription: {description}\n---\nBody.\n"),
        )
        .unwrap();
    }
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&root.path().join("skill-rules.config.json"), &config);
    let mut operation = tokio::spawn(
        command(root.path())
            .args(["--all", "--format", "json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_RULEBOOK_KEY")
            .output(),
    );
    let mut requests = 0;
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop { tokio::select! {
            output = &mut operation => break output.unwrap().unwrap(),
            request = server.receive() => {
                requests += 1;
                assert_eq!(request.body["questions"].as_object().unwrap().len(), 2);
                let probability = if request.body["state"]["where"]["file"] == "docs/a-drop.md" { 0.1 } else { 0.9 };
                request.respond(200, json!({"model":"typesafe/controlled","answers":{"gate":{"type":"noul","noul":probability},"unused":{"type":"noul","noul":0.5}},"usage":{"input_tokens":20,"output_tokens":5}}));
            }
        }}
    }).await.unwrap();
    assert_eq!(server.finish().await, 4);
    assert_eq!(requests, 4);
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected = [
        ("a-drop.md", "not_applicable", "gate", json!(["gate"]), 0.1),
        ("b-pass.md", "pass", "check", json!(["gate", "check"]), 0.9),
        ("c-finding.md", "finding", "check", json!(["gate", "check"]), 0.9),
        ("d-incomplete.md", "incomplete", "check", json!(["gate", "check"]), 0.9),
    ].map(|(file, outcome, step, path, probability)| {
        let mut row = json!({"name":"document_standard","marker":"K001","file":format!("docs/{file}"),"first_line":1,"last_line":4,"outcome":outcome,"step":step,"path":path,"answers":{"gate":{"type":"noul","yes_probability":probability}}});
        if outcome != "incomplete" { row["basis"] = json!("judged"); }
        row
    });
    assert_eq!(report["results"], json!(expected));
    assert_eq!(report["findings"][0]["basis"], "judged");
    assert_eq!(
        report["incomplete"][0]["reason"]["kind"],
        "evidence_unavailable"
    );
}
