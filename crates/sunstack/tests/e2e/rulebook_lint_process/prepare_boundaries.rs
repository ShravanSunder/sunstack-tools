use super::support::prepared_cli;
use super::*;

fn reply(body: &Value, admit_all: bool) -> Value {
    let answers = body["questions"].as_object().unwrap().iter().map(|(name, question)| {
        let answer = if question["type"] == "choice" {
            let labels = question["criteria"].as_object().unwrap();
            let chosen = if name == "purpose" {
                if admit_all || body["state"]["description"].as_str().unwrap().contains("ACCEPT") { "a" } else { "b" }
            } else { labels.keys().next().unwrap() };
            let probabilities = labels.keys().map(|label| (label.clone(), json!(if label == chosen {1.0} else {0.0})))
                .collect::<serde_json::Map<_,_>>();
            json!({"type":"choice","choice":chosen,"confidence":1.0,"probabilities":probabilities})
        } else { json!({"type":"noul","noul":0.9}) };
        (name.clone(), answer)
    }).collect::<serde_json::Map<_,_>>();
    json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
}

#[tokio::test]
async fn prepare_preserves_character_token_and_request_allowance_refusals_without_dispatch() {
    for limits in [
        json!({"request_max_chars":128}),
        json!({"max_input_tokens_per_request":1}),
        json!({"max_model_requests":1}),
    ] {
        let root = limit_fixture(
            json!({"body":{"from":"remainder"}}),
            json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
                "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
        );
        let mut rules = read(&root.path().join("rules.json"));
        rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","context":["reference"],"task":"Does this meet the standard?"}}});
        rules["K001"]["reference"] = json!({"text":"x".repeat(1000)});
        save(&root.path().join("rules.json"), &rules);
        for file in ["one", "two"] {
            std::fs::write(
                root.path().join(format!("docs/{file}.md")),
                format!("{file}_SENTINEL\n"),
            )
            .unwrap();
        }
        let mut server = controlled::ControlledServer::start().await;
        let mut config = read(&root.path().join("skill-rules.config.json"));
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        config["limits"] = limits.clone();
        save(&root.path().join("skill-rules.config.json"), &config);
        let output = prepared_cli::keyless(
            root.path(),
            &mut server,
            &["--all", "--prepare", "prepared.json"],
        )
        .await;
        assert_eq!(
            output.status.code(),
            Some(3),
            "limits {limits}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = read(&root.path().join("prepared.json"));
        assert_eq!(report["mode"], "prepare");
        assert_eq!(report["status"]["kind"], "incomplete");
        assert_eq!(report["usage"]["model_requests"], 0);
        assert!(!report["incomplete"].as_array().unwrap().is_empty());
        let prepared = report["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["status"] == "prepared")
            .count();
        assert_eq!(
            prepared,
            usize::from(limits.get("max_model_requests").is_some())
        );
        assert!(
            report["requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["state"] == "not_sent")
        );
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn prepare_invalid_endpoint_and_conflicting_cli_modes_keep_invalid_exit() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
            "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    std::fs::write(root.path().join("docs/one.md"), "Source.\n").unwrap();
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","task":"Does this meet the standard?"}}});
    save(&root.path().join("rules.json"), &rules);
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!("invalid endpoint without a scheme");
    save(&root.path().join("skill-rules.config.json"), &config);
    let output = command(root.path())
        .args(["--all", "--prepare", "invalid.json"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = read(&root.path().join("invalid.json"));
    assert_eq!(report["status"]["kind"], "invalid");
    assert_eq!(report["usage"]["model_requests"], 0);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["message"].as_str().unwrap().contains("endpoint"))
    );
    let conflict = command(root.path())
        .args([
            "--all",
            "--prepare",
            "conflict.json",
            "--report",
            "run.json",
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(conflict.status.code(), Some(2));
    assert!(!root.path().join("conflict.json").exists());
    let requires = command(root.path())
        .args(["--all", "--selections-from", "run.json"])
        .output()
        .await
        .unwrap();
    assert_eq!(requires.status.code(), Some(2));
}

#[tokio::test]
async fn already_cancelled_prepare_retains_cancelled_precedence() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        mechanical_graph(json!({"tool":"contains","field":"body","text":"Source"})),
    );
    std::fs::write(root.path().join("docs/one.md"), "Source.\n").unwrap();
    let cancellation = sunstack_decisions::CancellationToken::new();
    cancellation.cancel();
    let report = sunstack_lint::prepare(
        sunstack_lint::LintRequest {
            config_path: Some(root.path().join("skill-rules.config.json")),
            paths: vec![],
            scope: sunstack_lint::ScopeChoice::All,
            max_warnings: None,
            evidence_listing: None,
        },
        cancellation,
        vec![],
    )
    .await;
    assert_eq!(report.status, sunstack_lint::LintStatus::Cancelled);
    assert_eq!(report.status.exit_code(), 4);
    assert_eq!(report.usage.model_requests, 0);
    assert!(report.requests.is_empty());
}

#[tokio::test]
async fn prepare_does_not_offer_a_rule_when_its_gate_has_no_possible_folder() {
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
            "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    std::fs::write(root.path().join("docs/one.md"), "Checked text.\n").unwrap();
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","task":"Does this meet the standard?"}}});
    rules["K001"]["only_when"] = json!({"purpose":["a"]});
    save(&root.path().join("rules.json"), &rules);
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["evidence"]["kinds"]["metadata"] =
        json!({"rule":"document","fields":{"description":{"from":"yaml","key":"description"}}});
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"metadata","fields":["description"]},
        "labels":["a","b"],"questions":{"purpose":{"type":"choice","instructions":{"check":"description","task":"Classify"},"criteria":{"a":"A","b":"B"}}},
        "decision":{"start":"purpose","steps":{"purpose":{"ask":"purpose","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
    save(&root.path().join("skill-rules.config.json"), &config);
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "prepared.json"],
    )
    .await;
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read(&root.path().join("prepared.json"));
    assert_eq!(report["requests"], json!([]));
    assert!(!report["filter_skips"].as_array().unwrap().is_empty());
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn prepared_cut_and_locator_only_field_match_the_actual_wire_projection() {
    // Arrange: the locator reads a field absent from the rule's own question.
    let root = limit_fixture(
        json!({"body":{"from":"same_file","rule":"paragraph"},"anchor_text":{"from":"first_line"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
            "no_at_or_below":0.2,"yes":"pass","uncertain":"uncertain","no":{"finding":"Needs work.",
                "at":"anchor_text","locate":{"name":"anchor_line","task":"Which anchor line needs work?","min_probability":0.6}}}}}),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","context":["reference"],"task":"Does the text meet the standard?"}}});
    rules["K001"]["reference"] = json!({"10":"ten","2":"two","text":"é尾 \\ \"quoted\""});
    save(&root.path().join("rules.json"), &rules);
    std::fs::write(root.path().join("docs/one.md"), format!("ANCHOR_ONLY_SENTINEL\n\nPREFIX {} RAW_CUT_MIDDLE_SENTINEL {} SUFFIX\n\nORDERED_TAIL_SENTINEL.\n", "x".repeat(1800), "y".repeat(1800))).unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    save(&root.path().join("skill-rules.config.json"), &config);

    // Act: ordinary dispatch observes the real projection; prepare does not dispatch.
    let (ordinary, captures) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "ordinary.json"],
        |body| reply(body, true),
    )
    .await;
    assert_eq!(
        ordinary.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ordinary.stderr)
    );
    assert_eq!(captures.len(), 1);
    let prepared = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "prepared.json"],
    )
    .await;
    assert_eq!(
        prepared.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    assert_eq!(server.finish().await, 1);

    // Assert: raw evidence survives while cuts, locator choices and all request bytes agree.
    prepared_cli::assert_wire_parity(
        &std::fs::read(root.path().join("prepared.json")).unwrap(),
        &std::fs::read(root.path().join("ordinary.json")).unwrap(),
        &captures,
    );
    let wire: Value = serde_json::from_slice(&captures[0]).unwrap();
    assert!(wire["questions"].get("anchor_line").is_some());
    assert!(
        wire["state"]["anchor_text"]
            .as_str()
            .unwrap()
            .contains("ANCHOR_ONLY_SENTINEL")
    );
    let ordered = wire["state"]["body"].as_array().unwrap();
    assert_eq!(ordered.len(), 3);
    assert!(
        ordered[0]
            .as_str()
            .unwrap()
            .contains("ANCHOR_ONLY_SENTINEL")
    );
    assert!(
        ordered[2]
            .as_str()
            .unwrap()
            .contains("ORDERED_TAIL_SENTINEL")
    );
    assert!(
        !wire["state"]["body"]
            .to_string()
            .contains("RAW_CUT_MIDDLE_SENTINEL")
    );
    let report = read(&root.path().join("prepared.json"));
    assert!(
        report["piece_evidence"]
            .to_string()
            .contains("RAW_CUT_MIDDLE_SENTINEL")
    );
    assert!(
        report["piece_evidence"]
            .as_object()
            .unwrap()
            .values()
            .any(|piece| piece["piece"]["model_view"]["cut"]
                .as_array()
                .is_some_and(|cut| cut.contains(&json!("body"))))
    );
}

#[tokio::test]
async fn prepare_keeps_every_potentially_admitted_rule_with_gate_metadata() {
    // Arrange: one actual excluded folder and one admitted folder share the same gate.
    let root = limit_fixture(
        json!({"body":{"from":"remainder"}}),
        json!({"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
            "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","context":["where"],"task":"Does this meet the standard?"}}});
    rules["K001"]["only_when"] = json!({"purpose":["a"]});
    save(&root.path().join("rules.json"), &rules);
    for (folder, label) in [("one", "ACCEPT"), ("two", "REJECT")] {
        std::fs::create_dir(root.path().join("docs").join(folder)).unwrap();
        std::fs::write(
            root.path().join(format!("docs/{folder}/member.md")),
            format!("RULE_{folder}_SENTINEL.\n"),
        )
        .unwrap();
        std::fs::write(
            root.path().join(format!("docs/{folder}/about.md")),
            format!("---\ndescription: {label}_FILTER_SENTINEL\n---\n"),
        )
        .unwrap();
    }
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["check"]["files"] = json!(["docs/*/member.md"]);
    config["evidence"]["kinds"]["metadata"] =
        json!({"rule":"document","fields":{"description":{"from":"yaml","key":"description"}}});
    config["filters"] = json!({"purpose":{"per":"docs/{name}","reads":{"file":"about.md","kind":"metadata","fields":["description"]},
        "labels":["a","b"],"questions":{"purpose":{"type":"choice","instructions":{"check":"description","task":"Classify"},"criteria":{"a":"A","b":"B"}}},
        "decision":{"start":"purpose","steps":{"purpose":{"ask":"purpose","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
    save(&root.path().join("skill-rules.config.json"), &config);

    // Act: exclusion is observed normally; admission of both supplies all byte comparators.
    let (excluded, excluded_wire) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "excluded.json"],
        |body| reply(body, false),
    )
    .await;
    assert_eq!(
        excluded.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&excluded.stderr)
    );
    assert_eq!(excluded_wire.len(), 3);
    let (admitted, admitted_wire) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "admitted.json"],
        |body| reply(body, true),
    )
    .await;
    assert_eq!(admitted.status.code(), Some(0));
    assert_eq!(admitted_wire.len(), 4);
    let prepared = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "prepared.json"],
    )
    .await;
    assert_eq!(
        prepared.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    assert_eq!(server.finish().await, 7);

    // Assert: neither a guessed label nor a static tree route suppresses a request.
    let bytes = std::fs::read(root.path().join("prepared.json")).unwrap();
    prepared_cli::assert_wire_parity(
        &bytes,
        &std::fs::read(root.path().join("admitted.json")).unwrap(),
        &admitted_wire,
    );
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["requests"].as_array().unwrap().len(), 4);
    for request in report["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|request| request["owner"]["kind"] == "rule")
    {
        assert_eq!(request["gate"], json!({"filter":"purpose","labels":["a"]}));
    }
    assert_eq!(report["filter_answers"], json!([]));
    assert_eq!(report["needs_answer"], json!([]));
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn failed_prepare_admission_reserves_the_same_allowance_as_ordinary_execution() {
    let root = limit_fixture(
        json!({"body":{"from":"first_line"}}),
        json!({"start":"quality","steps":{"quality":{
        "ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}),
    );
    std::fs::write(root.path().join("docs/one.md"), "Plain text.\n").unwrap();
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{"check":"body","context":["reference"],"task":"Check text"}}});
    rules["K001"]["reference"] = json!({"long":"x".repeat(3000)});
    rules["K002"] = rules["K001"].clone();
    rules["K002"]["name"] = json!("small_second");
    rules["K002"].as_object_mut().unwrap().remove("reference");
    rules["K002"]["questions"]["quality"]["instructions"]
        .as_object_mut()
        .unwrap()
        .remove("context");
    save(&root.path().join("rules.json"), &rules);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Large first. <!-- K001 -->\nSmall second. <!-- K002 -->\n",
    )
    .unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["limits"] = json!({"max_model_requests":2,"max_input_tokens_per_request":1000,"request_max_chars":20000});
    save(&root.path().join("skill-rules.config.json"), &config);
    let (positive, captures) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "two-slots.json"],
        |body| reply(body, true),
    )
    .await;
    assert_eq!(positive.status.code(), Some(3));
    assert_eq!(captures.len(), 1);
    config["limits"]["max_model_requests"] = json!(1);
    save(&root.path().join("skill-rules.config.json"), &config);
    let (ordinary, captured) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "one-slot.json"],
        |body| reply(body, true),
    )
    .await;
    assert_eq!(ordinary.status.code(), Some(3));
    assert!(captured.is_empty());
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "prepared.json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(3));
    let normal = read(&root.path().join("one-slot.json"));
    let prepared = read(&root.path().join("prepared.json"));
    assert!(
        !prepared["requests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["status"] == "prepared"),
        "oversized first request did not reserve the sole allowance"
    );
    for report in [&normal, &prepared] {
        let second = report["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == "small_second")
            .unwrap();
        assert_eq!(second["gap"], "request_limit_reached");
    }
    assert_eq!(server.finish().await, 1);
}
