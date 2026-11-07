use super::support::{
    prepared_cli,
    raw_request_fields::{RawRequestFields, compact_json_value},
};
use super::*;

fn configure(root: &Path, endpoint: &str, files: Value) -> Value {
    let mut config = read(&root.join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(endpoint);
    config["decision_model"]["model"] = json!("typesafe/controlled");
    config["check"]["files"] = files;
    let mut rules = read(&root.join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{
        "check":"body","context":["where"],"task":"Does this step meet the standard?"}}});
    rules["K001"]["decision"] = json!({"start":"quality","steps":{"quality":{"ask":"quality",
        "yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}});
    save(&root.join("rules.json"), &rules);
    config
}
fn response(body: &Value) -> Value {
    json!({"model":"typesafe/controlled","answers":body["questions"].as_object().unwrap().keys()
        .map(|name|(name.clone(),json!({"type":"noul","noul":0.93}))).collect::<serde_json::Map<_,_>>(),
        "usage":{"input_tokens":1,"output_tokens":1}})
}
fn refused_rule_owners(report: &Value) -> Vec<Value> {
    report["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["owner"]["kind"] == "rule" && entry["state"] == "not_sent")
        .map(|entry| json!({"owner":entry["owner"],"reason":entry["reason"]}))
        .collect()
}
fn assert_sent_payloads_match(prepared: &[u8], ordinary: &[u8], captures: &[Vec<u8>]) {
    #[derive(serde::Deserialize)]
    struct Report<'a> {
        #[serde(borrow)]
        requests: Vec<Entry<'a>>,
    }
    #[derive(serde::Deserialize)]
    struct Entry<'a> {
        #[serde(borrow)]
        request: RawRequestFields<'a>,
        #[serde(default)]
        status: Option<String>,
    }
    let prepared: Report<'_> = serde_json::from_slice(prepared).unwrap();
    let ordinary: Report<'_> = serde_json::from_slice(ordinary).unwrap();
    let offered = prepared
        .requests
        .iter()
        .filter(|entry| matches!(entry.status.as_deref(), Some("prepared" | "from_report")))
        .collect::<Vec<_>>();
    assert_eq!(offered.len(), captures.len());
    for entry in offered {
        let state = compact_json_value(entry.request.state.get());
        let questions = compact_json_value(entry.request.questions.get());
        assert_eq!(
            captures
                .iter()
                .map(|bytes| serde_json::from_slice::<RawRequestFields<'_>>(bytes).unwrap())
                .filter(|wire| compact_json_value(wire.state.get()) == state
                    && compact_json_value(wire.questions.get()) == questions)
                .count(),
            1
        );
        assert_eq!(
            ordinary
                .requests
                .iter()
                .filter(
                    |source| compact_json_value(source.request.state.get()) == state
                        && compact_json_value(source.request.questions.get()) == questions
                )
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn replayed_selection_spends_the_same_downstream_allowance_as_ordinary_selection() {
    // Arrange: one selector precedes six known judged step requests.
    let root = fixture();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = configure(
        root.path(),
        &server.endpoint,
        json!(["skills/practices-research/SKILL.md"]),
    );
    let mut total_http = 0;
    for allowance in [2, 1] {
        config["limits"]["max_model_requests"] = json!(allowance);
        save(&root.path().join("skill-rules.config.json"), &config);
        let normal_file = format!("ordinary-{allowance}.json");
        let prepared_file = format!("prepared-{allowance}.json");
        let (normal, captures) = prepared_cli::answered(
            root.path(),
            &mut server,
            &["--all", "--report", &normal_file],
            response,
        )
        .await;
        assert_eq!(normal.status.code(), Some(3));
        assert_eq!(captures.len(), allowance);
        total_http += captures.len();
        // Act: source is this exact limited ordinary run, with a genuinely answered selector.
        let output = prepared_cli::keyless(
            root.path(),
            &mut server,
            &[
                "--all",
                "--prepare",
                &prepared_file,
                "--selections-from",
                &normal_file,
            ],
        )
        .await;
        assert_eq!(output.status.code(), Some(3));
        let ordinary = read(&root.path().join(&normal_file));
        let prepared = read(&root.path().join(&prepared_file));
        let offered = prepared["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["owner"]["kind"] == "rule" && entry["status"] == "prepared")
            .count();
        // Assert: a saved selector still consumes its logical slot, although no inference is sent.
        assert_eq!(
            offered,
            allowance - 1,
            "replayed selection bypassed the logical request allowance"
        );
        assert_eq!(
            prepared["requests"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|entry| entry["status"] == "from_report")
                .count(),
            1
        );
        assert_eq!(
            refused_rule_owners(&prepared),
            refused_rule_owners(&ordinary)
        );
        assert_eq!(prepared["usage"]["model_requests"], 0);
        assert_sent_payloads_match(
            &std::fs::read(root.path().join(prepared_file)).unwrap(),
            &std::fs::read(root.path().join(normal_file)).unwrap(),
            &captures,
        );
    }
    assert_eq!(server.finish().await, total_http);
}

#[tokio::test]
async fn replay_refuses_another_selector_after_the_logical_allowance_is_exhausted() {
    // Arrange: save two actual answered selectors without downstream model requests.
    let root = fixture();
    let mut server = controlled::ControlledServer::start().await;
    let mut config = configure(
        root.path(),
        &server.endpoint,
        json!([
            "skills/practices-research/SKILL.md",
            "skills/presentation-tui/SKILL.md"
        ]),
    );
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"].as_object_mut().unwrap().remove("questions");
    rules["K001"]["decision"] = json!({"start":"body","steps":{"body":{"mechanical_check":{
        "tool":"regex","field":"body","pattern":".*"},"yes":"pass","no":{"finding":"Missing body."}}}});
    save(&root.path().join("rules.json"), &rules);
    config["limits"]["max_model_requests"] = json!(2);
    save(&root.path().join("skill-rules.config.json"), &config);
    let (wide, wide_captures) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "two-selectors.json"],
        response,
    )
    .await;
    assert_eq!(wide.status.code(), Some(0));
    assert_eq!(wide_captures.len(), 2);
    config["limits"]["max_model_requests"] = json!(1);
    save(&root.path().join("skill-rules.config.json"), &config);
    let (limited, captures) = prepared_cli::answered(
        root.path(),
        &mut server,
        &["--all", "--report", "one-slot.json"],
        response,
    )
    .await;
    assert_eq!(limited.status.code(), Some(3));
    assert_eq!(captures.len(), 1);
    // Act: matching receipts exist for both, but only the ordinary first slot is available.
    let output = prepared_cli::keyless(
        root.path(),
        &mut server,
        &[
            "--all",
            "--prepare",
            "replayed.json",
            "--selections-from",
            "two-selectors.json",
        ],
    )
    .await;
    let replayed = read(&root.path().join("replayed.json"));
    // Assert: checking the limit is required as well as incrementing the counter.
    assert_eq!(
        replayed["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["status"] == "from_report")
            .count(),
        1,
        "replay accepted a second selector after its request allowance was exhausted"
    );
    assert_eq!(output.status.code(), Some(3));
    let ordinary = read(&root.path().join("one-slot.json"));
    assert_eq!(replayed["selections"], ordinary["selections"]);
    assert_eq!(replayed["usage"]["model_requests"], 0);
    assert_eq!(server.finish().await, 3);
}
