#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
use candidate_fixture::fixture;

fn outcome(output: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["kind"], "operation_result");
    assert_eq!(wire["result"]["operation"], "evaluate_candidates");
    wire["result"]["outcome"].clone()
}

#[tokio::test]
async fn two_sessions_get_separate_requests_answers_and_attributed_occurrences() {
    let fixture_root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let input = fixture(fixture_root.path(), &server.endpoint);
    for (session, text) in [
        ("first", "MATCH FIRST_OCCURRENCE"),
        ("second", "MATCH SECOND_OCCURRENCE"),
    ] {
        std::fs::write(fixture_root.path().join(format!("{session}.jsonl")), format!("{}\n", json!({"type":"assistant","message":{"id":session,"content":[{"type":"text","text":text}]}}))).unwrap();
    }
    let path = fixture_root.path().to_owned();
    let child =
        tokio::spawn(async move { support::run(&path, "evaluate_candidates", input).await });
    for _ in 0..2 {
        let request = server.next().await;
        let rendered = request.body.to_string();
        let first = rendered.contains("FIRST_OCCURRENCE");
        let second = rendered.contains("SECOND_OCCURRENCE");
        assert_ne!(
            first, second,
            "each request must carry exactly its own occurrence"
        );
        request.noul(if first { 0.95 } else { 0.1 });
    }
    let output = child.await.unwrap();
    let result = outcome(&output, 0);
    assert_eq!(result["kind"], "candidate_report");
    let entries = result["report"]["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let mut kinds = entries
        .iter()
        .map(|entry| entry["outcome"]["kind"].as_str().unwrap())
        .collect::<Vec<_>>();
    kinds.sort();
    assert_eq!(kinds, ["dropped", "finding"]);
    for entry in entries {
        assert_eq!(entry["occurrence"]["author"], "assistant");
        assert_eq!(entry["occurrence"]["positions"][0]["line"], 1);
        assert_eq!(entry["occurrence"]["matched_at"]["byte_offset"], 0);
        assert_eq!(entry["answers"].as_array().unwrap().len(), 1);
        let record = entry["occurrence"]["record"].as_str().unwrap();
        let input_report = result["report"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|input| record.starts_with(input["input_id"].as_str().unwrap()))
            .unwrap();
        let source_path = std::path::Path::new(input_report["location"]["path"].as_str().unwrap());
        let line: Value = serde_json::from_str(
            std::fs::read_to_string(source_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        let is_first = line["message"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("FIRST_OCCURRENCE");
        assert_eq!(
            entry["outcome"]["kind"],
            if is_first { "finding" } else { "dropped" }
        );
        assert_eq!(
            entry["answers"][0]["answer"]["yes_probability"],
            json!(if is_first { 0.95 } else { 0.1 })
        );
    }
    assert_ne!(
        entries[0]["occurrence"]["record"],
        entries[1]["occurrence"]["record"]
    );
    assert_eq!(result["report"]["usage"]["requests"], 2);
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn malformed_fields_and_unknown_providers_are_invalid_before_requests_or_bundle() {
    let fixture_root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = fixture(fixture_root.path(), &server.endpoint);
    let mut misspelt = input.clone();
    let sources = misspelt.as_object_mut().unwrap().remove("sources").unwrap();
    misspelt["sourcse"] = sources;
    let mut unknown = input;
    unknown["sources"][0]["source"]["provider"] = json!("unknown_provider");
    for invalid in [misspelt, unknown] {
        let output = support::run(fixture_root.path(), "evaluate_candidates", invalid).await;
        let result = outcome(&output, 2);
        assert_eq!(result["kind"], "invalid");
        assert!(result.get("report").is_none());
        assert!(
            !result["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            result["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["location"]["field_path"]
                    .as_array()
                    .is_some_and(|path| path.contains(&json!("sources"))))
        );
    }
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn tool_names_do_not_decide_a_verdict_and_json_encoded_arguments_match_at_the_call() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"][0]["source"]["provider"] = json!("codex");
    input["sources"][1]["source"]["provider"] = json!("codex");
    input["recipes"][0]["provider"] = json!("codex");
    input["recipes"][0]["read"]["format"] = json!("codex_rollout_jsonl");
    let rules_path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    rule["mechanical_pass"]["predicate"]["field"] = json!("/body/input/cmd");
    std::fs::write(&rules_path, rule.to_string()).unwrap();
    for (session, tool) in [("first", "Read"), ("second", "Bash")] {
        std::fs::write(root.path().join(format!("{session}.jsonl")),format!("{}\n{}\n",json!({"type":"response_item","payload":{"type":"function_call","call_id":"c1","name":tool,"arguments":"{\"cmd\":\"MATCH decoded argument\"}"}}),json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"a distinct result"}}))).unwrap();
    }
    let path = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&path, "evaluate_candidates", input).await });
    for _ in 0..2 {
        let request = server.next().await;
        let body = request.body.to_string();
        assert!(body.contains("MATCH decoded argument"));
        assert!(!body.contains("a distinct result"));
        request.noul(0.9);
    }
    let output = run.await.unwrap();
    let result = outcome(&output, 0);
    for candidate in result["report"]["candidates"].as_array().unwrap() {
        assert_eq!(candidate["outcome"]["kind"], "finding");
        assert_eq!(candidate["occurrence"]["matched_at"]["line"], 1);
    }
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn mechanical_verdicts_are_refused_without_sending_a_request() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = fixture(root.path(), &server.endpoint);
    let rules_path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    rule["verdict"] = json!({"kind":"mechanical","explanation":"mechanics cannot decide the tree","check":{"kind":"expect","expect":{"matches":"Bash"}}});
    std::fs::write(&rules_path, rule.to_string()).unwrap();
    let output = support::run(root.path(), "evaluate_candidates", input).await;
    let result = outcome(&output, 2);
    assert_eq!(result["kind"], "invalid");
    // Existing definition admission already rejects a mechanical conversation target.
    assert_eq!(result["diagnostics"]["items"][0]["code"], "invalid_target");
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn request_limit_preserves_every_candidate_and_never_reports_nothing_found() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    input["execution"]["max_concurrent_requests"] = json!(1);
    input["request_budget"]["max_requests"] = json!(2);
    let lines = (0..5).map(|index| json!({"type":"assistant","message":{"id":format!("candidate-{index}"),"content":[{"type":"text","text":format!("MATCH candidate {index}")}]}}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
    std::fs::write(root.path().join("first.jsonl"), lines).unwrap();
    let path = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&path, "evaluate_candidates", input).await });
    for _ in 0..2 {
        server.next().await.noul(0.9);
    }
    let output = run.await.unwrap();
    let result = outcome(&output, 3);
    assert_eq!(result["kind"], "incomplete");
    assert_eq!(result["report"]["completeness"]["kind"], "partial");
    assert_eq!(result["report"]["usage"]["requests"], 2);
    let candidates = result["report"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 5);
    assert_eq!(
        candidates
            .iter()
            .filter(|entry| entry["outcome"]["kind"] == "not_processed"
                && entry["outcome"]["reason"] == "run_stopped")
            .count(),
        3
    );
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn failed_backend_keeps_a_terminal_candidate_and_does_not_invent_cost() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"assistant","message":{"id":"failed","content":[{"type":"text","text":"MATCH backend failure"}]}}))).unwrap();
    let path = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&path, "evaluate_candidates", input).await });
    server
        .next()
        .await
        .respond(500, json!({"error":"scripted failure"}));
    let output = run.await.unwrap();
    let result = outcome(&output, 0);
    assert_eq!(result["report"]["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["report"]["candidates"][0]["outcome"],
        json!({"kind":"not_processed","reason":"evaluation_failed"})
    );
    assert_eq!(result["report"]["completeness"]["kind"], "inconclusive");
    assert!(result["report"]["usage"]["cost_usd"].is_null());
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn existing_evaluate_rules_still_selects_thinking_from_the_same_capture() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    let path = root.path().join("first.jsonl");
    std::fs::write(&path,format!("{}\n",json!({"type":"assistant","message":{"id":"old-path","content":[{"type":"text","text":"MATCH visible"},{"type":"thinking","thinking":"LEGACY_THINKING_SENTINEL"}]}}))).unwrap();
    input.as_object_mut().unwrap().remove("sources");
    input.as_object_mut().unwrap().remove("recipes");
    input["origin"] = json!({"kind":"direct"});
    input["cursors"] = json!([]);
    input["definitions"] = json!({"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"conversation","transcript_path":path,"format":"claude_code_jsonl"}}}]}]});
    let directory = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&directory, "evaluate_rules", input).await });
    let request = server.next().await;
    assert!(
        request
            .body
            .to_string()
            .contains("LEGACY_THINKING_SENTINEL")
    );
    request.noul(0.9);
    let output = run.await.unwrap();
    let result = support::response(&output, 0);
    assert_eq!(result["result"]["outcome"]["kind"], "complete_report");
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn a_match_inside_thinking_is_retained_but_not_sent_without_its_rules_grant() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rules_path = root.path().join("rules.jsonl");
    let mut granted: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    granted["rule_id"] = json!("granted.rule");
    granted["private_access"] = json!(["thinking"]);
    granted["mechanical_pass"]["predicate"]["field"] = json!("/body/blocks/1/block/text");
    let mut denied = granted.clone();
    denied["rule_id"] = json!("denied.rule");
    denied["private_access"] = json!([]);
    std::fs::write(&rules_path, format!("{granted}\n{denied}\n")).unwrap();
    std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"assistant","message":{"id":"private-match","content":[{"type":"text","text":"public text"},{"type":"thinking","thinking":"MATCH PRIVATE_THINKING"}]}}))).unwrap();
    let path = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&path, "evaluate_candidates", input).await });
    let request = server.next().await;
    assert!(request.body.to_string().contains("PRIVATE_THINKING"));
    request.noul(0.9);
    let result = outcome(&run.await.unwrap(), 0);
    let entries = result["report"]["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry["occurrence"]["field"], "/body/blocks/1/block/text");
        if entry["rule"]["rule_id"] == "denied.rule" {
            assert_eq!(
                entry["outcome"],
                json!({"kind":"not_processed","reason":"left_out_for_privacy"})
            );
            assert!(entry["answers"].as_array().unwrap().is_empty());
        } else {
            assert_eq!(entry["outcome"]["kind"], "finding");
        }
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn a_false_local_gate_drops_every_public_candidate_without_http_or_a_credential() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rules_path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    rule.as_object_mut().unwrap().remove("mechanical_pass");
    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"NEVER_PRESENT_IN_THE_PUBLIC_RECORD"},"max_scan_bytes":100000}}]);
    std::fs::write(&rules_path, rule.to_string()).unwrap();
    let lines=(0..3).map(|index|json!({"type":"assistant","message":{"id":format!("public-{index}"),"content":"public source text"}}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
    std::fs::write(root.path().join("first.jsonl"), lines).unwrap();
    let output = support::run_without_credential(root.path(), "evaluate_candidates", input).await;
    let result = outcome(&output, 0);
    assert_eq!(result["report"]["candidates"].as_array().unwrap().len(), 3);
    for entry in result["report"]["candidates"].as_array().unwrap() {
        assert_eq!(entry["outcome"]["kind"], "dropped");
        assert_eq!(entry["answers"][0]["answer"], Value::Null);
        assert_eq!(entry["answers"][0]["settlement"], "did_not_pass");
        assert_eq!(entry["answers"][0]["answer_id"], entry["outcome"]["answer"]);
        assert_eq!(entry["answers"][0]["question"], entry["outcome"]["at"]);
    }
    assert_eq!(result["report"]["completeness"]["kind"], "nothing_found");
    assert_eq!(result["report"]["usage"]["requests"], 0);
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn local_pattern_gates_read_only_their_candidates_record() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rules_path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    rule.as_object_mut().unwrap().remove("mechanical_pass");
    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"GATE_PERMITTED"},"max_scan_bytes":100000}}]);
    std::fs::write(&rules_path, rule.to_string()).unwrap();
    let lines = [
        "GATE_PERMITTED first record",
        "second record without permission",
    ]
    .map(|text| json!({"type":"assistant","message":{"content":text}}).to_string())
    .join("\n")
        + "\n";
    std::fs::write(root.path().join("first.jsonl"), lines).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = server.next().await;
    assert!(
        request
            .body
            .to_string()
            .contains("GATE_PERMITTED first record")
    );
    request.noul(0.9);
    let output = tokio::select! {
        output=&mut run=>output.unwrap(),
        unexpected=server.next()=>panic!("a second record incorrectly reached the judge: {}", unexpected.body),
    };
    let result = outcome(&output, 0);
    let entries = result["report"]["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(
            entry["outcome"]["kind"],
            if entry["occurrence"]["positions"][0]["line"] == 1 {
                "finding"
            } else {
                "dropped"
            }
        );
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn a_local_pattern_gate_observes_only_its_rules_private_grants() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rules_path = root.path().join("rules.jsonl");
    let mut granted: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    granted.as_object_mut().unwrap().remove("mechanical_pass");
    granted["rule_id"] = json!("granted.rule");
    granted["private_access"] = json!(["thinking"]);
    granted["verdict"]["question"]["instructions"] = json!("GRANTED_PRIVATE_GATE");
    granted["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"PRIVATE_GATE_PERMISSION"},"max_scan_bytes":100000}}]);
    let mut denied = granted.clone();
    denied["rule_id"] = json!("denied.rule");
    denied["private_access"] = json!([]);
    denied["verdict"]["question"]["instructions"] = json!("DENIED_PRIVATE_GATE");
    std::fs::write(&rules_path, format!("{granted}\n{denied}\n")).unwrap();
    std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"assistant","message":{"id":"mixed","content":[{"type":"text","text":"visible public text"},{"type":"thinking","thinking":"PRIVATE_GATE_PERMISSION"}]}}))).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = server.next().await;
    let body = request.body.to_string();
    assert!(body.contains("GRANTED_PRIVATE_GATE"));
    assert!(!body.contains("DENIED_PRIVATE_GATE"));
    assert!(body.contains("PRIVATE_GATE_PERMISSION"));
    request.noul(0.9);
    let output = tokio::select! {
        output=&mut run=>output.unwrap(),
        unexpected=server.next()=>panic!("a private gate leaked into an ungranted rule: {}",unexpected.body),
    };
    let result = outcome(&output, 0);
    for entry in result["report"]["candidates"].as_array().unwrap() {
        assert_eq!(
            entry["outcome"]["kind"],
            if entry["rule"]["rule_id"] == "granted.rule" {
                "finding"
            } else {
                "dropped"
            }
        );
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn orphan_gap_metadata_cannot_bypass_credential_projection() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    input["sources"][0]["source"]["provider"] = json!("codex");
    input["recipes"][0]["provider"] = json!("codex");
    input["recipes"][0]["read"]["format"] = json!("codex_rollout_jsonl");
    let rules_path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    rule["mechanical_pass"]["predicate"]["field"] = json!("/body/result/text");
    std::fs::write(&rules_path, rule.to_string()).unwrap();
    let secret = "sk-PRIVATE_CREDENTIAL_SENTINEL";
    std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"response_item","payload":{"type":"function_call_output","call_id":secret,"output":"MATCH public result"}}))).unwrap();
    let directory = root.path().to_owned();
    let run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = server.next().await;
    assert!(!request.body.to_string().contains(secret));
    request.noul(0.9);
    let output = run.await.unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
    let result = outcome(&output, 0);
    assert_eq!(
        result["report"]["inputs"][0]["unfit"][0]["kind"],
        "orphan_result"
    );
    assert_eq!(
        result["report"]["candidates"][0]["occurrence"]["author"],
        "tool"
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn mechanical_drops_are_sampled_from_real_records_deterministically() {
    let fixture_root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut input = fixture(fixture_root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let lines = (0..50).map(|index| json!({"type":"assistant","message":{"id":format!("drop-{index}"),"content":[{"type":"text","text":format!("nonmatching record {index}")}]}}).to_string()).collect::<Vec<_>>().join("\n") + "\n";
    std::fs::write(fixture_root.path().join("first.jsonl"), lines).unwrap();
    let mut samples = Vec::new();
    for _ in 0..2 {
        let output = support::run(fixture_root.path(), "evaluate_candidates", input.clone()).await;
        let result = outcome(&output, 0);
        assert_eq!(result["report"]["completeness"]["kind"], "nothing_found");
        assert_eq!(result["report"]["candidates"], json!([]));
        let dropped = result["report"]["dropped_samples"].as_array().unwrap();
        assert_eq!(dropped.len(), 20);
        for item in dropped {
            assert!(
                item["excerpt"]
                    .as_str()
                    .unwrap()
                    .contains("nonmatching record")
            );
            assert!(
                item["occurrence"]["positions"][0]["line"]
                    .as_u64()
                    .is_some_and(|line| (1..=50).contains(&line))
            );
            let line = item["occurrence"]["positions"][0]["line"].as_u64().unwrap() as usize;
            let source = std::fs::read_to_string(fixture_root.path().join("first.jsonl")).unwrap();
            let source_line: Value =
                serde_json::from_str(source.lines().nth(line - 1).unwrap()).unwrap();
            let excerpt: Value = serde_json::from_str(item["excerpt"].as_str().unwrap()).unwrap();
            assert_eq!(
                excerpt["blocks"][0]["block"]["text"],
                source_line["message"]["content"][0]["text"]
            );
            assert_eq!(
                item["occurrence"]["matched_at"]["byte_offset"],
                json!(
                    source
                        .split_inclusive('\n')
                        .take(line - 1)
                        .map(str::len)
                        .sum::<usize>()
                )
            );
        }
        samples.push(dropped.clone());
    }
    assert_eq!(samples[0], samples[1]);
    assert_eq!(server.finish().await, 0);
}
