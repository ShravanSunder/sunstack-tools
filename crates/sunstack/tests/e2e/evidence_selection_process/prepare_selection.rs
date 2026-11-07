use super::support::raw_request_fields::{RawRequestFields, compact_json_value};
use super::*;

#[tokio::test]
async fn recorded_selection_prepares_exact_chosen_requests_and_changed_inputs_stay_unknown() {
    // Arrange: six known steps under one admitted workflow, with the peer workflow rejected.
    let root = fixture();
    let mut rules = read(&root.path().join("rules.json"));
    rules["K001"]["questions"] = json!({"quality":{"type":"noul","instructions":{
        "check":"body","context":["where"],"task":"Does this step meet the standard?"}}});
    rules["K001"]["decision"] = json!({"start":"quality","steps":{"quality":{
        "ask":"quality","yes_at_or_above":0.8,"no_at_or_below":0.2,
        "yes":"pass","no":{"finding":"Needs work."},"uncertain":"uncertain"}}});
    save(&root.path().join("rules.json"), &rules);
    let mut server = controlled::ControlledServer::start().await;
    let mut config = read(&root.path().join("skill-rules.config.json"));
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    config["decision_model"]["model"] = json!("typesafe/controlled");
    save(&root.path().join("skill-rules.config.json"), &config);
    let mut ordinary = tokio::spawn(
        command(root.path())
            .current_dir(root.path())
            .args(["--all", "--report", "ordinary.json"])
            .env("OPENROUTER_API_KEY", "SYNTHETIC_LOOPBACK_ONLY")
            .output(),
    );
    let mut captures = Vec::new();
    let output = tokio::time::timeout(Duration::from_secs(10), async { loop { tokio::select! {
        output = &mut ordinary => break output.unwrap().unwrap(),
        request = server.receive() => {
            let probability = if request.body["state"]["where"]["file"].as_str()
                .is_some_and(|file| file.contains("presentation-tui")) { 0.12 } else { 0.93 };
            captures.push(request.raw_body.clone()); request.noul(probability);
        }
    }}}).await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(captures.len(), 8);
    let normal = read(&root.path().join("ordinary.json"));
    let selections = normal["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|request| request["owner"]["kind"] == "selection")
        .collect::<Vec<_>>();
    assert_eq!(
        selections.len(),
        2,
        "ordinary saved report must retain actual selection payloads"
    );

    let selection_id = selections[0]["id"].as_u64().unwrap().to_string();
    let shown = command(root.path())
        .current_dir(root.path())
        .args(["show", "ordinary.json", &selection_id])
        .output()
        .await
        .unwrap();
    assert_eq!(shown.status.code(), Some(0));
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["request"]["owner"]["kind"], "selection");

    // Act: reuse only the recorded matching selection, with no authenticated dispatch.
    let mut prepared = tokio::spawn(
        command(root.path())
            .current_dir(root.path())
            .args([
                "--all",
                "--prepare",
                "prepared.json",
                "--selections-from",
                "ordinary.json",
            ])
            .output(),
    );
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::select! {
            output = &mut prepared => output.unwrap().unwrap(),
            _ = server.receive() => panic!("selection replay dispatched a request"),
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
    assert_eq!(server.finish().await, 8);

    // Assert: exact persisted values match the real HTTP values and six source-coordinate sentinels.
    let prepared_bytes = std::fs::read(root.path().join("prepared.json")).unwrap();
    let prepared = read(&root.path().join("prepared.json"));
    let requests = prepared["requests"].as_array().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request["status"] == "from_report")
            .count(),
        2
    );
    let mut lines = requests
        .iter()
        .filter(|request| request["owner"]["kind"] == "rule")
        .map(|request| request["owner"]["piece"]["first_line"].as_u64().unwrap())
        .collect::<Vec<_>>();
    lines.sort_unstable();
    assert_eq!(lines, [21, 22, 24, 25, 26, 27]);
    #[derive(serde::Deserialize)]
    struct Saved<'a> {
        #[serde(borrow)]
        requests: Vec<Entry<'a>>,
    }
    #[derive(serde::Deserialize)]
    struct Entry<'a> {
        #[serde(borrow)]
        request: RawRequestFields<'a>,
    }
    let raw: Saved<'_> = serde_json::from_slice(&prepared_bytes).unwrap();
    let normal_bytes = std::fs::read(root.path().join("ordinary.json")).unwrap();
    let normal_raw: Saved<'_> = serde_json::from_slice(&normal_bytes).unwrap();
    for entry in &raw.requests {
        let state = compact_json_value(entry.request.state.get());
        let questions = compact_json_value(entry.request.questions.get());
        let matching_wire = captures
            .iter()
            .map(|body| serde_json::from_slice::<RawRequestFields<'_>>(body).unwrap())
            .filter(|wire| {
                compact_json_value(wire.state.get()) == state
                    && compact_json_value(wire.questions.get()) == questions
            })
            .count();
        assert_eq!(matching_wire, 1);
        assert_eq!(
            normal_raw
                .requests
                .iter()
                .filter(
                    |normal| compact_json_value(normal.request.state.get()) == state
                        && compact_json_value(normal.request.questions.get()) == questions
                )
                .count(),
            1
        );
    }
    assert_eq!(prepared["usage"]["model_requests"], 0);
    assert_eq!(prepared["needs_answer"], json!([]));

    // Each refusal changes one selection record, while its unchanged peer can still replay.
    let selected_index = normal["requests"]
        .as_array()
        .unwrap()
        .iter()
        .position(|request| {
            request["owner"]["kind"] == "selection"
                && request["owner"]["file"]
                    .as_str()
                    .unwrap()
                    .contains("practices-research")
        })
        .unwrap();
    for refusal in ["unanswered", "question", "model", "conflicting_duplicate"] {
        let mut ledger = normal.clone();
        match refusal {
            "unanswered" => {
                ledger["requests"][selected_index]["state"] = json!("prepared");
                ledger["requests"][selected_index]["status"] = json!("prepared");
                ledger["requests"][selected_index]
                    .as_object_mut()
                    .unwrap()
                    .remove("outcome");
            }
            "question" => {
                let questions = ledger["requests"][selected_index]["request"]["questions"]
                    .as_object_mut()
                    .unwrap();
                let question = questions.values_mut().next().unwrap();
                question["instructions"]["task"] = json!("Different selection question");
            }
            "model" => {
                ledger["requests"][selected_index]["outcome"]["receipt"]["resolved_model"] =
                    json!("typesafe/other-version");
            }
            "conflicting_duplicate" => {
                let mut duplicate = ledger["requests"][selected_index].clone();
                duplicate["id"] = json!(9999);
                for answer in duplicate["outcome"]["answers"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                {
                    answer["yes_probability"] = json!(0.12);
                }
                ledger["requests"].as_array_mut().unwrap().push(duplicate);
            }
            _ => unreachable!(),
        }
        save(&root.path().join("source-control.json"), &ledger);
        let output = command(root.path())
            .current_dir(root.path())
            .args([
                "--all",
                "--prepare",
                "control.json",
                "--selections-from",
                "source-control.json",
            ])
            .output()
            .await
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(6),
            "{refusal}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = read(&root.path().join("control.json"));
        assert!(
            report["needs_answer"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["kind"] == "step"
                    && entry["file"]
                        .as_str()
                        .unwrap()
                        .contains("practices-research")),
            "{refusal}"
        );
        assert!(
            !report["requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["owner"]["kind"] == "rule"),
            "{refusal}"
        );
    }
    // A source case ledger may reuse an identical request across revisions. Agreeing duplicates
    // retain the first recorded source id and authored revision, without requiring current Git state.
    let case_source = json!({"runs":[{"revision":"historical-source-a", "report":normal},
        {"revision":"historical-source-b", "report":normal}]});
    save(&root.path().join("source-cases.json"), &case_source);
    let output = command(root.path())
        .current_dir(root.path())
        .args([
            "--all",
            "--prepare",
            "case-replay.json",
            "--selections-from",
            "source-cases.json",
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(6));
    let case_replay = read(&root.path().join("case-replay.json"));
    let reused = case_replay["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["status"] == "from_report")
        .collect::<Vec<_>>();
    assert_eq!(reused.len(), 2);
    for entry in reused {
        assert_eq!(entry["source_revision"], "historical-source-a");
        assert!(
            selections
                .iter()
                .any(|original| original["id"] == entry["source_request_id"])
        );
    }
    assert_eq!(case_replay["needs_answer"], json!([]));

    let source = root.path().join("skills/practices-research/SKILL.md");
    let text = std::fs::read_to_string(&source).unwrap();
    std::fs::write(
        &source,
        text.replace("Ordered Workflow", "Changed candidate workflow"),
    )
    .unwrap();
    let output = command(root.path())
        .current_dir(root.path())
        .args([
            "--all",
            "--prepare",
            "changed.json",
            "--selections-from",
            "ordinary.json",
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(6));
    let changed = read(&root.path().join("changed.json"));
    assert!(
        changed["needs_answer"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["kind"] == "step")
    );
    assert!(
        !changed["requests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["owner"]["kind"] == "rule")
    );
}

#[tokio::test]
async fn unknown_selection_is_deferred_while_independent_kinds_still_build() {
    // Arrange: a kind after selection in the plan has no selection dependency.
    let root = fixture();
    let path = root.path().join("skill-rules.config.json");
    let mut config = read(&path);
    config["evidence"]["kinds"]["z_independent"] = json!({
        "rule":"heading", "fields":{"body":{"from":"first_line"}}
    });
    save(&path, &config);
    let mut rules = read(&root.path().join("rules.json"));
    rules["K002"] = json!({"name":"independent_heading", "evidence":"z_independent",
        "decision":{"start":"check","steps":{"check":{"mechanical_check":{
            "tool":"regex","field":"body","pattern":"\\S"},"yes":"pass","no":"not_applicable"}}}});
    save(&root.path().join("rules.json"), &rules);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check step text. <!-- K001 -->\n\nCheck independent headings. <!-- K002 -->\n",
    )
    .unwrap();

    // Act: no key or synthetic selection answer is provided.
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        command(root.path())
            .current_dir(root.path())
            .args(["--all", "--prepare", "prepared.json"])
            .output(),
    )
    .await
    .unwrap()
    .unwrap();

    // Assert: unknown is not rejection, and unrelated work has not been dropped.
    assert_eq!(
        output.status.code(),
        Some(6),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read(&root.path().join("prepared.json"));
    assert!(
        report["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == "independent_heading")
    );
    assert!(
        !report["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == "step_body")
    );
    let pending = report["needs_answer"].as_array().unwrap();
    for kind in ["workflow_section", "step", "load"] {
        assert!(
            pending.iter().any(|row| row["kind"] == kind),
            "missing deferred kind {kind}: {pending:?}"
        );
    }
    for dependency in pending {
        let id = dependency["waits_on"]["request_id"].as_u64().unwrap();
        assert!(
            report["requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|request| request["id"] == id
                    && request["owner"]["kind"] == "selection"
                    && request["owner"]["file"] == dependency["file"])
        );
    }
    assert_eq!(report["usage"]["model_requests"], 0);
    assert!(
        report["selections"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["outcome"]["kind"] == "prepared")
    );
}
