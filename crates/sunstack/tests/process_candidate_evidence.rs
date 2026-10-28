#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
use evidence_fixture::{fixture, report};
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::json;

#[tokio::test]
async fn absent_and_explicitly_unavailable_facts_are_unknown_without_model_work() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = fixture(root.path(), &server.endpoint, false);
    for binding in [
        None,
        Some(json!({"kind":"unavailable","reason":"withheld"})),
    ] {
        let mut input = input.clone();
        input["payload_schemas"] = json!([]);
        if let Some(ref binding) = binding {
            input["evidence"] = json!({"fact":binding});
        }
        let outcome = report(
            &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
            0,
        );
        let candidate = &outcome["report"]["candidates"][0];
        assert_eq!(candidate["outcome"]["kind"], "unresolved");
        assert_eq!(candidate["outcome"]["at"], json!({"kind":"verdict"}));
        assert_eq!(
            candidate["outcome"]["missing"]["kind"],
            "evidence_unavailable"
        );
        assert_eq!(candidate["outcome"]["missing"]["evidence"], "fact");
        assert_eq!(
            candidate["outcome"]["missing"]["reason"],
            if binding.is_some() {
                "withheld"
            } else {
                "not_recorded"
            }
        );
        assert_eq!(candidate["answers"][0]["settlement"], "unresolved");
        assert_eq!(candidate["evidence"][0]["input"], "fact");
        assert_eq!(candidate["evidence"][0]["binding"]["kind"], "unavailable");
        assert!(candidate["evidence"][0]["content_identity"].is_null());
        assert_eq!(outcome["report"]["usage"]["requests"], 0);
    }
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn valid_payload_and_document_facts_reach_the_actual_request() {
    for document in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = fixture(root.path(), &server.endpoint, document);
        let binding = if document {
            std::fs::write(root.path().join("owner.md"), "OWNER_DOCUMENT_FACT\n").unwrap();
            json!({"kind":"document","repository":"facts","path":"owner.md"})
        } else {
            json!({"kind":"payload","value":{"decision":"OWNER_PAYLOAD_FACT"}})
        };
        input["evidence"] = json!({"fact":{"kind":"bound","binding":binding}});
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let request = tokio::select! {
            result = &mut run => panic!("valid fact refused: {}", String::from_utf8_lossy(&result.unwrap().stdout)),
            request = server.next() => request,
        };
        assert!(request.body.to_string().contains(if document {
            "OWNER_DOCUMENT_FACT"
        } else {
            "OWNER_PAYLOAD_FACT"
        }));
        request.noul(0.9);
        let output = run.await.unwrap();
        let outcome = report(&output, 0);
        let entry = &outcome["report"]["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], "finding");
        assert_eq!(entry["evidence"][0]["input"], "fact");
        assert_eq!(entry["evidence"][0]["binding"]["kind"], "bound");
        let bytes = if document {
            b"OWNER_DOCUMENT_FACT\n".to_vec()
        } else {
            serde_json::to_vec(&json!({"decision":"OWNER_PAYLOAD_FACT"})).unwrap()
        };
        assert_eq!(
            entry["evidence"][0]["content_identity"],
            sunstack_evidence::content_digest(&bytes)
        );
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn supplied_bad_payloads_are_invalid_instead_of_unknown() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = fixture(root.path(), &server.endpoint, false);
    for (schemas, value) in [
        (json!([]), json!({"decision":"valid"})),
        (input["payload_schemas"].clone(), json!({"decision":false})),
    ] {
        for matched in [true, false] {
            std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"assistant","message":{"content":if matched {"MATCH validated"} else {"ordinary source"}}}))).unwrap();
            let mut input = input.clone();
            input["payload_schemas"] = schemas.clone();
            input["evidence"] =
                json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":value.clone()}}});
            let outcome = report(
                &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
                2,
            );
            assert_eq!(outcome["kind"], "invalid");
            assert!(outcome.get("report").is_none());
            let codes = outcome["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|item| item["code"].as_str())
                .collect::<Vec<_>>();
            assert!(
                codes.contains(&"unknown_payload_schema")
                    || codes.contains(&"payload_schema_mismatch"),
                "{outcome}"
            );
            assert!(
                outcome["diagnostics"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["location"]["field_path"]
                        .as_array()
                        .is_some_and(|path| !path.is_empty())),
                "bad payload must name its field: {outcome}"
            );
        }
    }
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn a_bound_document_that_cannot_be_captured_is_unresolved() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, true);
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}});
    let outcome = report(
        &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
        0,
    );
    let candidate = &outcome["report"]["candidates"][0];
    assert_eq!(candidate["outcome"]["kind"], "unresolved");
    assert_eq!(
        candidate["outcome"]["missing"]["kind"],
        "evidence_not_captured"
    );
    assert_eq!(candidate["outcome"]["missing"]["evidence"], "fact");
    assert!(candidate["outcome"]["missing"]["diagnostic"].is_string());
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn identical_candidate_text_uses_each_entries_own_fact_override() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, false);
    std::fs::copy(
        root.path().join("first.jsonl"),
        root.path().join("second.jsonl"),
    )
    .unwrap();
    input["sources"].as_array_mut().unwrap().push(json!({"source":{"kind":"session_pair","provider":"claude_code","session":"second"},"evidence":{"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"ALLOW_OWN_CONTEXT"}}}}}));
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"DENY_RUN_CONTEXT"}}}});
    let identities = input["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let source: sunstack_evidence::SourceInput =
                serde_json::from_value(entry["source"].clone()).unwrap();
            source.source_identity().unwrap().as_str().to_owned()
        })
        .collect::<Vec<_>>();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    for _ in 0..2 {
        let request = tokio::select! {
            result = &mut run => panic!("paired evidence refused: {}",String::from_utf8_lossy(&result.unwrap().stdout)),
            request = server.next() => request,
        };
        let body = request.body.to_string();
        let allowed = body.contains("ALLOW_OWN_CONTEXT");
        assert_ne!(
            allowed,
            body.contains("DENY_RUN_CONTEXT"),
            "contexts mixed: {body}"
        );
        request.noul(if allowed { 0.9 } else { 0.1 });
    }
    let outcome = report(&run.await.unwrap(), 0);
    let entries = outcome["report"]["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    for (identity, kind) in identities.iter().zip(["dropped", "finding"]) {
        let entry = entries
            .iter()
            .find(|entry| {
                entry["occurrence"]["record"]
                    .as_str()
                    .unwrap()
                    .starts_with(identity)
            })
            .unwrap();
        assert_eq!(entry["outcome"]["kind"], kind);
    }
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn an_unavailable_fact_does_not_stop_an_independent_candidate() {
    for explicit in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = fixture(root.path(), &server.endpoint, false);
        std::fs::copy(
            root.path().join("first.jsonl"),
            root.path().join("second.jsonl"),
        )
        .unwrap();
        if explicit {
            input["evidence"] = json!({"fact":{"kind":"unavailable","reason":"withheld"}});
        }
        input["sources"].as_array_mut().unwrap().push(json!({"source":{"kind":"session_pair","provider":"claude_code","session":"second"},"evidence":{"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"AVAILABLE_SECOND_CANDIDATE"}}}}}));
        let identities = input["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                let source: sunstack_evidence::SourceInput =
                    serde_json::from_value(entry["source"].clone()).unwrap();
                source.source_identity().unwrap().as_str().to_owned()
            })
            .collect::<Vec<_>>();
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let request = tokio::select! {
            output = &mut run => panic!("the independent candidate stopped: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request = server.next() => request,
        };
        assert!(
            request
                .body
                .to_string()
                .contains("AVAILABLE_SECOND_CANDIDATE")
        );
        request.noul(0.9);
        let output = tokio::select! {
            output = &mut run => output.unwrap(),
            request = server.next() => panic!("unavailable question dispatched: {}",request.body),
        };
        let outcome = report(&output, 0);
        assert_eq!(
            outcome["report"]["completeness"],
            json!({"kind":"findings","count":1,"inconclusive":1})
        );
        assert_eq!(
            outcome["report"]["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|entry| entry["outcome"]["kind"] == "unresolved")
                .count(),
            1
        );
        let entries = outcome["report"]["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        for (identity, kind) in identities.iter().zip(["unresolved", "finding"]) {
            let entry = entries
                .iter()
                .find(|entry| {
                    entry["occurrence"]["record"]
                        .as_str()
                        .unwrap()
                        .starts_with(identity)
                })
                .unwrap();
            assert_eq!(entry["outcome"]["kind"], kind);
            if kind == "unresolved" {
                assert_eq!(
                    entry["outcome"]["missing"]["reason"],
                    if explicit { "withheld" } else { "not_recorded" }
                );
            }
        }
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn a_known_gate_runs_before_its_verdicts_unavailable_fact_stops_that_question() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, false);
    input["evidence"] = json!({"fact":{"kind":"unavailable","reason":"withheld"}});
    let path = root.path().join("rules.jsonl");
    let mut rule: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut evidence = rule["evidence"][0].clone();
    evidence["id"] = json!("gate_subject");
    rule["applicability"] = json!([{"kind":"decision","question":{"type":"noul","instructions":"KNOWN_GATE_ONLY"},"evidence":[evidence],"threshold":0.5}]);
    std::fs::write(path, rule.to_string()).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = tokio::select! {
        output = &mut run => panic!("known gate never ran: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
        request = server.next() => request,
    };
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
    assert!(
        request.body["questions"]
            .as_object()
            .unwrap()
            .values()
            .all(|question| question["instructions"]
                .to_string()
                .contains("KNOWN_GATE_ONLY"))
    );
    request.noul(0.9);
    let output = tokio::select! {
        output = &mut run => output.unwrap(),
        request = server.next() => panic!("unavailable verdict was asked: {}",request.body),
    };
    let outcome = report(&output, 0);
    let entry = &outcome["report"]["candidates"][0];
    assert_eq!(entry["outcome"]["kind"], "unresolved");
    assert_eq!(entry["outcome"]["at"], json!({"kind":"verdict"}));
    assert_eq!(entry["answers"].as_array().unwrap().len(), 2);
    assert!(
        entry["answers"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |answer| answer["question"] == json!({"kind":"rule_gate","position":0})
                    && answer["answer"]["yes_probability"] == 0.9
            )
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn unavailable_and_uncaptured_facts_in_local_gates_stay_unknown() {
    for document in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut input = fixture(root.path(), &server.endpoint, document);
        input["evidence"] = if document {
            json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}})
        } else {
            json!({"fact":{"kind":"unavailable","reason":"withheld"}})
        };
        let path = root.path().join("rules.jsonl");
        let mut rule: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let test = if document {
            json!({"kind":"pattern_present","input":"fact","pattern":{"kind":"literal","text":"anything"},"max_scan_bytes":10000})
        } else {
            json!({"kind":"payload_field_present","input":"fact","pointer":"/decision"})
        };
        rule["applicability"] = json!([{"kind":"condition","test":test}]);
        std::fs::write(path, rule.to_string()).unwrap();
        let outcome = report(
            &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
            0,
        );
        let entry = &outcome["report"]["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], "unresolved");
        assert_eq!(
            entry["outcome"]["at"],
            json!({"kind":"rule_gate","position":0})
        );
        assert_eq!(
            entry["outcome"]["missing"]["kind"],
            if document {
                "evidence_not_captured"
            } else {
                "evidence_unavailable"
            }
        );
        if document {
            assert_eq!(
                entry["outcome"]["missing"]["diagnostic"],
                "source_unavailable"
            );
        }
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn the_same_record_can_be_evaluated_again_with_only_its_evidence_context_changed() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let input = fixture(root.path(), &server.endpoint, false);
    let mut candidates = Vec::new();
    let mut bodies = Vec::new();
    for (decision, probability, kind) in [
        ("PAIR_DENY", 0.1, "dropped"),
        ("PAIR_ALLOW", 0.9, "finding"),
    ] {
        let mut request_input = input.clone();
        request_input["sources"][0]["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":decision}}}});
        let directory = root.path().to_owned();
        let mut run = tokio::spawn(async move {
            support::run(&directory, "evaluate_candidates", request_input).await
        });
        let request = tokio::select! {
            output = &mut run => panic!("paired same-record run refused: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request = server.next() => request,
        };
        assert!(request.body.to_string().contains(decision));
        // Compare the full sent states after normalizing only the deliberately varied fact.
        let mut normalized = request.body.clone();
        for content in normalized["state"]["contents"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            if content.get("decision").is_some() {
                content["decision"] = json!("PAIRED_CONTEXT");
            }
        }
        bodies.push(normalized);
        request.noul(probability);
        let output = tokio::select! {
            output = &mut run => output.unwrap(),
            request = server.next() => panic!("unexpected additional paired request: {}",request.body),
        };
        let outcome = report(&output, 0);
        let entry = &outcome["report"]["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], kind);
        assert_eq!(
            entry["answers"][0]["answer"]["yes_probability"],
            probability
        );
        candidates.push(entry["candidate_id"].clone());
    }
    assert_eq!(
        candidates[0], candidates[1],
        "physical record and rule are unchanged across runs"
    );
    assert_eq!(
        bodies[0], bodies[1],
        "only the fact may differ in the judge input"
    );
    assert_eq!(server.finish().await, 2);
}
