use super::*;

use super::support::raw_request_fields::{RawRequestFields, compact_json_value};

#[tokio::test]
async fn prepare_cases_keeps_historical_accounting_and_shows_one_case_after_source_removal() {
    // Arrange: today's rulebook is overlaid, while checked text stays at its revision.
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{
        "ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass",
        "no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","task":"Does this meet the standard?"}}});
    save(&root.path().join("rules.json"), &rules);
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&root.path().join("skill-rules.config.json"), &config);
    let historic = "HISTORICAL_CASE_SENTINEL.\n";
    std::fs::write(root.path().join("docs/one.md"), historic).unwrap();
    std::fs::write(root.path().join("outside.md"), historic).unwrap();
    let obsolete = root.path().join("evidence/rules/obsolete.yml");
    std::fs::write(&obsolete, "obsolete invalid historical rule\n").unwrap();
    let revision = super::case_runs::commit_tree(root.path());
    std::fs::remove_file(obsolete).unwrap();
    let case = |id: &str, file: &str, hash: &str| {
        json!({"id":id,"kind":"rule",
        "rule":"document_standard","file":file,"revision":revision,"lines":[1,1],
        "text_sha256":hash,"expected":"pass","set":"development","label_from":"blind-agents"})
    };
    let hash = sunstack_evidence::content_digest(historic.as_bytes());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[
        case("prepared", "docs/one.md", &hash), case("stale", "docs/one.md", &"0".repeat(64)),
        case("unmatched", "outside.md", &hash)],"bars":[]}),
    );
    std::fs::write(
        root.path().join("docs/one.md"),
        "CURRENT_TEXT_MUST_NOT_REPLACE_HISTORY\n",
    )
    .unwrap();

    // Act: mixed stale/unmatched cases keep exit1 instead of prepare-only6.
    let (ordinary, captures) = support::prepared_cli::answered(
        root.path(),
        &mut server,
        &["cases", "cases.json"],
        |body| model_response(body, 0.9),
    )
    .await;
    assert_eq!(ordinary.status.code(), Some(1));
    assert_eq!(captures.len(), 1);
    let output = support::prepared_cli::keyless(
        root.path(),
        &mut server,
        &["cases", "cases.json", "--prepare", "cases-prepared.json"],
    )
    .await;
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        root.path().join("cases-prepared.json").is_file(),
        "cases --prepare did not persist its report"
    );
    let prepared = read(&root.path().join("cases-prepared.json"));
    assert_eq!(prepared["mode"], "prepare");
    assert_eq!(
        prepared["counts"],
        json!({"prepared":1,"stale":1,"no_matching_piece":1})
    );
    assert_eq!(prepared["runs"][0]["report"]["usage"]["model_requests"], 0);
    assert_eq!(server.finish().await, 1);
    assert_eq!(
        prepared["runs"][0]["report"]["piece_evidence"]["1"]["piece"]["text"],
        historic
    );

    // Assert: case ordinal readback is self-contained, including a reason-only stale entry.
    std::fs::remove_file(root.path().join("skill-rules.config.json")).unwrap();
    std::fs::remove_file(root.path().join("rules.json")).unwrap();
    std::fs::remove_file(root.path().join("docs/one.md")).unwrap();
    for (ordinal, id) in [(1, "prepared"), (2, "stale")] {
        let output = support::isolated_command(root.path())
            .current_dir(root.path())
            .args(["lint", "show", "cases-prepared.json", &ordinal.to_string()])
            .output()
            .await
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let shown: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(shown["case"]["id"], id);
        assert!(shown.get("cases").is_none());
        if ordinal == 2 {
            assert!(shown.get("detail").is_none());
        }
    }
}

#[test]
fn raw_field_comparison_preserves_order_and_escaped_content() {
    let left = r#"{ "state": {"10":"ten","2":"two","text":"é \\ \" tail"}, "questions": {} }"#;
    let left: RawRequestFields<'_> = serde_json::from_str(left).unwrap();
    assert_eq!(
        compact_json_value(left.state.get()),
        r#"{"10":"ten","2":"two","text":"é \\ \" tail"}"#.as_bytes()
    );
    assert_ne!(
        compact_json_value(r#"{"10":1,"2":2}"#),
        compact_json_value(r#"{"2":2,"10":1}"#)
    );
}

#[tokio::test]
async fn prepare_mechanical_run_is_keyless_and_has_distinct_exit() {
    // Arrange: the real loader and mechanical walker need no provider.
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"contains","field":"body","text":"Sentinel"})),
    );
    std::fs::write(root.path().join("docs/one.md"), "Sentinel.\n").unwrap();

    // Act: the new front-door operation persists the report before returning.
    let output = command(root.path())
        .args(["--all", "--prepare", "prepared.json"])
        .output()
        .await
        .unwrap();

    // Assert: prepare is neither a judged pass nor a missing-credential failure.
    assert_eq!(
        output.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read(&root.path().join("prepared.json"));
    assert_eq!(report["mode"], "prepare");
    assert_eq!(report["usage"]["model_requests"], 0);
    assert_eq!(report["requests"], json!([]));
    assert_eq!(report["results"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn prepare_persisted_state_matches_actual_http_and_saved_request_without_dispatch() {
    // Arrange: the endpoint and all source bytes stay identical across both operations.
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({
            "start":"quality", "steps":{"quality":{"ask":"quality",
            "yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}
        }),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["reference"] =
        json!({"10":"ten", "2":"two", "text":"é尾 \"quoted\" \\ line\nend"});
    rules["K001"]["questions"] = json!({"quality":{"type":"noul",
        "instructions":{"check":"body","context":["reference"],"task":"Does the text meet this standard?"}}});
    save(&root.path().join("rules.json"), &rules);
    std::fs::write(
        root.path().join("docs/one.md"),
        "COLLECTION_SENTINEL é尾.\n",
    )
    .unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&root.path().join("skill-rules.config.json"), &config);
    let mut ordinary = tokio::spawn(
        command(root.path())
            .args(["--all", "--report", "ordinary.json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_LOOPBACK_ONLY")
            .output(),
    );
    let mut captures = Vec::new();
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                output = &mut ordinary => break output.unwrap().unwrap(),
                request = server.receive() => {
                    captures.push(request.raw_body.clone());
                    let response = model_response(&request.body, 0.9);
                    request.respond(200, response);
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        captures.len(),
        1,
        "the real ordinary dispatch must be observable"
    );

    // Act: the live listener catches even a hidden request in the prepare operation.
    let mut prepared = tokio::spawn(
        command(root.path())
            .args(["--all", "--prepare", "prepared.json"])
            .output(),
    );
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::select! {
            output = &mut prepared => output.unwrap().unwrap(),
            _ = server.receive() => panic!("prepare dispatched a provider request"),
        }
    })
    .await
    .unwrap();
    assert_eq!(
        output.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(server.finish().await, 1);

    // Assert: persisted values, not reconstructed expected objects, match the actual wire.
    let prepared_bytes = std::fs::read(root.path().join("prepared.json")).unwrap();
    let ordinary_bytes = std::fs::read(root.path().join("ordinary.json")).unwrap();
    #[derive(serde::Deserialize)]
    struct SavedRequests<'a> {
        #[serde(borrow)]
        requests: Vec<SavedRequest<'a>>,
    }
    #[derive(serde::Deserialize)]
    struct SavedRequest<'a> {
        #[serde(borrow)]
        request: RawRequestFields<'a>,
    }
    let prepared_raw: SavedRequests<'_> = serde_json::from_slice(&prepared_bytes).unwrap();
    let ordinary_raw: SavedRequests<'_> = serde_json::from_slice(&ordinary_bytes).unwrap();
    let wire: RawRequestFields<'_> = serde_json::from_slice(&captures[0]).unwrap();
    assert_eq!(prepared_raw.requests.len(), 1);
    assert_eq!(ordinary_raw.requests.len(), 1);
    for request in [
        &prepared_raw.requests[0].request,
        &ordinary_raw.requests[0].request,
    ] {
        assert_eq!(
            compact_json_value(request.state.get()),
            compact_json_value(wire.state.get())
        );
        assert_eq!(
            compact_json_value(request.questions.get()),
            compact_json_value(wire.questions.get())
        );
    }
    let prepared: Value = serde_json::from_slice(&prepared_bytes).unwrap();
    assert_eq!(prepared["mode"], "prepare");
    assert_eq!(prepared["requests"][0]["status"], "prepared");
    assert!(prepared["requests"][0].get("outcome").is_none());
    assert_eq!(prepared["usage"]["model_requests"], 0);
    let ordinary: Value = serde_json::from_slice(&ordinary_bytes).unwrap();
    assert!(ordinary.get("mode").is_none());
    assert!(ordinary.get("needs_answer").is_none());
    assert!(wire.state.get().contains("COLLECTION_SENTINEL"));
}

#[tokio::test]
async fn prepare_cases_reports_unavailable_field_without_request_or_dispatch() {
    // Arrange: a historical target exists but cannot be decoded as checked text.
    let root = limit_fixture(
        json!({"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":1000}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
            "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"targets","task":"Does the target meet the standard?"}}});
    save(&root.path().join("rules.json"), &rules);
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["check"]["files"] = json!(["docs/one.md"]);
    save(&root.path().join("skill-rules.config.json"), &config);
    let source = "Load `bad.md`.\n";
    std::fs::write(root.path().join("docs/one.md"), source).unwrap();
    std::fs::write(root.path().join("docs/bad.md"), [0xff, 0xfe]).unwrap();
    let revision = super::case_runs::commit_tree(root.path());
    save(
        &root.path().join("cases.json"),
        &json!({"cases":[{"id":"unavailable","kind":"rule",
        "rule":"document_standard","file":"docs/one.md","revision":revision,"lines":[1,1],
        "text_sha256":sunstack_evidence::content_digest(source.as_bytes()),"expected":"pass",
        "set":"development","label_from":"blind-agents"}],"bars":[]}),
    );

    // Act: the real case CLI overlays today's rulebook, then refuses the unavailable input.
    let output = support::prepared_cli::keyless(
        root.path(),
        &mut server,
        &["cases", "cases.json", "--prepare", "unavailable.json"],
    )
    .await;

    // Assert: an explicit gap, not a prepared request or inferred verdict.
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read(&root.path().join("unavailable.json"));
    assert_eq!(report["counts"], json!({"incomplete":1}));
    assert!(
        report["cases"][0]["outcome"]["reason"]
            .as_str()
            .unwrap()
            .contains("EvidenceUnavailable")
    );
    assert_eq!(report["runs"][0]["report"]["usage"]["model_requests"], 0);
    assert!(
        report["runs"][0]["report"]["requests"]
            .as_array()
            .unwrap()
            .iter()
            .all(|request| request["status"] != "prepared")
    );
    assert_eq!(server.finish().await, 0);
}
