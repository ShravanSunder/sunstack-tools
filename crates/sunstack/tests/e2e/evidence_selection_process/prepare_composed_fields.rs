use super::support::prepared_cli;
use super::*;

#[tokio::test]
async fn composed_selected_fields_wait_without_blocking_independent_fields_and_replay_matches_wire()
{
    // Arrange: document existence and its plain field are independent of selected sections.
    let root = fixture();
    let rules_path = root.path().join("evidence/rules/selection.yml");
    let mut matchers = std::fs::read_to_string(&rules_path).unwrap();
    matchers.push_str("\n---\nid: document\nlanguage: Markdown\nrule: {kind: document}\n");
    std::fs::write(rules_path, matchers).unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["decision_model"]["model"] = json!("typesafe/controlled");
    config["check"]["files"] = json!([
        "skills/practices-research/SKILL.md",
        "skills/presentation-tui/SKILL.md"
    ]);
    config["evidence"]["kinds"]["document"] = json!({"rule":"document","max_chars":20000,"fields":{
        "selected_sections":{"from":"same_file","kind":"workflow_section","text":"full"},"inside_steps":{"from":"inside","kind":"step"},"rest":{"from":"remainder"},"plain":{"from":"first_line"}}});
    config["evidence"]["kinds"]["independent"] =
        json!({"rule":"skill","fields":{"plain":{"from":"first_line"}}});
    config["evidence"]["kinds"]["selected_document"] =
        config["evidence"]["kinds"]["document"].clone();
    config["evidence"]["kinds"]["selected_document"]["select"] = json!({"question":{"type":"noul","instructions":{
        "check":"selected_sections","task":"Is this document's selected context sufficient?"}},"yes_at_or_above":0.8,"no_at_or_below":0.2});
    save(&root.path().join("skill-rules.config.json"), &config);
    let mut rules = read(&root.path().join("rules.json"));
    for (marker, name, kind, field) in [
        ("K002", "selected_context", "document", "selected_sections"),
        ("K003", "plain_document", "document", "plain"),
        ("K004", "independent", "independent", "plain"),
        ("K005", "inside_context", "document", "inside_steps"),
        ("K006", "enclosing_context", "load", "step"),
        ("K007", "remainder_context", "document", "rest"),
    ] {
        rules[marker] = json!({"name":name,"evidence":kind,"questions":{"quality":{"type":"noul",
            "instructions":{"check":field,"context":["where"],"task":format!("Does {name} text meet the standard?")}}},
            "decision":{"start":"quality","steps":{"quality":{"ask":"quality","yes_at_or_above":0.8,
                "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}}});
    }
    save(&root.path().join("rules.json"), &rules);
    std::fs::write(root.path().join("rulebook.md"), "Check steps. <!-- K001 -->\nCheck selected context. <!-- K002 -->\nCheck plain document. <!-- K003 -->\nCheck independent text. <!-- K004 -->\nCheck inside context. <!-- K005 -->\nCheck enclosing context. <!-- K006 -->\nCheck remainder context. <!-- K007 -->\n").unwrap();

    // Act: an unanswered selected field must not become an empty prepared input.
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "unknown.json"],
    )
    .await;
    assert_eq!(
        output.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let unknown = read(&root.path().join("unknown.json"));
    let requests = unknown["requests"].as_array().unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request["owner"]["rule"] == "selected_context"),
        "deferred selected field was offered as fabricated empty/partial evidence"
    );
    for dependent in ["inside_context", "remainder_context"] {
        assert!(
            !requests
                .iter()
                .any(|request| request["owner"]["rule"] == dependent),
            "{dependent}"
        );
    }
    assert!(
        !requests
            .iter()
            .any(|request| request["owner"]["rule"] == "enclosing_context"
                && request["owner"]["piece"]["first_line"] == 24),
        "the load inside the unknown selected step must wait"
    );
    assert!(
        requests
            .iter()
            .any(|request| request["owner"]["rule"] == "enclosing_context"
                && request["owner"]["piece"]["first_line"] == 29),
        "the load outside every candidate step remains known"
    );
    assert!(
        unknown["selections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["outcome"]["kind"] == "deferred")
    );
    for name in ["plain_document", "independent"] {
        assert_eq!(
            requests
                .iter()
                .filter(|request| request["owner"]["rule"] == name)
                .count(),
            2,
            "{name}"
        );
    }
    assert!(
        unknown["needs_answer"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["kind"] == "document")
    );

    // Assert: answered replay goes through the same builders, with actual HTTP as comparator.
    let (ordinary,captures)=prepared_cli::answered(root.path(),&mut server,
        &["--all","--report","ordinary.json"], |body| json!({"model":"typesafe/controlled","answers":body["questions"].as_object().unwrap().keys().map(|name|(name.clone(),json!({"type":"noul","noul":0.93}))).collect::<serde_json::Map<_,_>>(),"usage":{"input_tokens":1,"output_tokens":1}})).await;
    assert_eq!(ordinary.status.code(), Some(0));
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &[
            "--all",
            "--prepare",
            "replayed.json",
            "--selections-from",
            "ordinary.json",
        ],
    )
    .await;
    assert_eq!(output.status.code(), Some(6));
    prepared_cli::assert_wire_parity(
        &std::fs::read(root.path().join("replayed.json")).unwrap(),
        &std::fs::read(root.path().join("ordinary.json")).unwrap(),
        &captures,
    );
    let ordinary = read(&root.path().join("ordinary.json"));
    let mut partial = ordinary.clone();
    let source = partial["requests"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| {
            entry["owner"]["kind"] == "selection"
                && entry["owner"]["file"]
                    .as_str()
                    .unwrap()
                    .contains("presentation-tui")
        })
        .unwrap();
    source["state"] = json!("prepared");
    source["status"] = json!("prepared");
    source.as_object_mut().unwrap().remove("outcome");
    save(&root.path().join("partial.json"), &partial);
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &[
            "--all",
            "--prepare",
            "partial-prepared.json",
            "--selections-from",
            "partial.json",
        ],
    )
    .await;
    assert_eq!(output.status.code(), Some(6));
    let partial = read(&root.path().join("partial-prepared.json"));
    let documents = partial["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["owner"]["rule"] == "selected_context")
        .collect::<Vec<_>>();
    assert_eq!(documents.len(), 1);
    assert!(
        documents[0]["owner"]["piece"]["file"]
            .as_str()
            .unwrap()
            .contains("practices-research")
    );
    // Projection shares the kind's field budget: an unknown sibling can affect even a
    // question that reads only plain text. It must wait rather than invent a bounded view.
    config["evidence"]["kinds"]["document"]["max_chars"] = json!(1000);
    save(&root.path().join("skill-rules.config.json"), &config);
    let (bounded, bounded_captures) = prepared_cli::answered(root.path(), &mut server,
        &["--all", "--report", "bounded-ordinary.json"], |body| json!({"model":"typesafe/controlled",
        "answers":body["questions"].as_object().unwrap().keys().map(|name|(name.clone(),json!({"type":"noul","noul":0.93}))).collect::<serde_json::Map<_,_>>(),
        "usage":{"input_tokens":1,"output_tokens":1}})).await;
    assert_eq!(
        bounded.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&bounded.stdout),
        String::from_utf8_lossy(&bounded.stderr)
    );
    let normal = read(&root.path().join("bounded-ordinary.json"));
    assert!(normal["requests"].as_array().unwrap().iter().any(|entry| {
        entry["owner"]["rule"] == "plain_document"
            && !entry["request"]["state"]["cut-fields"]
                .as_array()
                .unwrap()
                .is_empty()
    }));
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &["--all", "--prepare", "bounded-unknown.json"],
    )
    .await;
    assert_eq!(output.status.code(), Some(6));
    let bounded = read(&root.path().join("bounded-unknown.json"));
    assert!(
        !bounded["requests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["owner"]["rule"] == "plain_document"),
        "unknown composed fields also defer the shared model-view budget"
    );
    assert_eq!(
        server.finish().await,
        captures.len() + bounded_captures.len()
    );
}
