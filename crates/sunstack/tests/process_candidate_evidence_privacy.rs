#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
mod support;
use controlled_http::ControlledServer;
use evidence_fixture::{fixture, report};
use serde_json::{Value, json};
#[tokio::test]
async fn document_privacy_grants_stay_in_their_own_rule_request() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, true);
    let secret = "sk-SYNTHETICoutside123456789";
    std::fs::write(
        root.path().join("owner.md"),
        format!("Owner fact: {secret}\n"),
    )
    .unwrap();
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}});
    let path = root.path().join("rules.jsonl");
    let mut ordinary: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    ordinary["verdict"]["question"]["instructions"] = json!("OUTSIDE_DEFAULT");
    let mut granted = ordinary.clone();
    granted["rule_id"] = json!("candidate.granted");
    granted["private_access"] = json!(["credential_like"]);
    granted["verdict"]["question"]["instructions"] = json!("OUTSIDE_GRANTED");
    std::fs::write(path, format!("{ordinary}\n{granted}\n")).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let mut observed = Vec::new();
    for _ in 0..2 {
        let request = tokio::select! {
            result = &mut run => panic!("outside document refused: {}",String::from_utf8_lossy(&result.unwrap().stdout)),
            request = server.next() => request,
        };
        let body = request.body.to_string();
        let has_grant = body.contains("OUTSIDE_GRANTED");
        assert_eq!(
            body.contains(secret),
            has_grant,
            "rule grant crossed a request: {body}"
        );
        observed.push(has_grant);
        request.noul(0.9);
    }
    observed.sort();
    assert_eq!(observed, [false, true]);
    let outcome = report(&run.await.unwrap(), 0);
    assert_eq!(outcome["report"]["candidates"].as_array().unwrap().len(), 2);
    let denied = outcome["report"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["rule"]["rule_id"] == "candidate.one")
        .unwrap();
    let allowed = outcome["report"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["rule"]["rule_id"] == "candidate.granted")
        .unwrap();
    assert_eq!(denied["privacy_view"]["grants"], json!([]));
    assert_eq!(
        allowed["privacy_view"]["grants"],
        json!(["credential_like"])
    );
    assert!(
        outcome["report"]["privacy"]["granted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|grant| grant["rule"] == allowed["rule"]
                && grant["private_kind"] == "credential_like")
    );
    let mut denied_scopes = outcome.clone();
    denied_scopes["report"]["candidates"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["rule"]["rule_id"] != "candidate.granted");
    assert!(!denied_scopes.to_string().contains(secret));
    assert!(
        outcome["report"]["privacy"]["left_out"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count["route"] == "outside_evidence"
                && count["private_kind"] == "credential_like"
                && count["count"].as_u64().unwrap() > 0)
    );
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn local_conditions_use_the_projected_outside_document() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, true);
    let secret = "sk-SYNTHETIClocaloutside123456789";
    std::fs::write(
        root.path().join("owner.md"),
        format!("Owner fact: {secret}\n"),
    )
    .unwrap();
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}});
    let path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"fact","pattern":{"kind":"literal","text":secret},"max_scan_bytes":100000}}]);
    std::fs::write(path, rule.to_string()).unwrap();
    let outcome = report(
        &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
        0,
    );
    assert_eq!(
        outcome["report"]["candidates"][0]["outcome"]["kind"],
        "dropped"
    );
    assert_eq!(
        outcome["report"]["candidates"][0]["outcome"]["at"],
        json!({"kind":"rule_gate","position":0})
    );
    assert_eq!(outcome["report"]["usage"]["requests"], 0);
    assert!(!outcome.to_string().contains(secret));
    assert_eq!(
        outcome["report"]["candidates"][0]["evidence"][0]["content_identity"],
        sunstack_evidence::content_digest(format!("Owner fact: {secret}\n").as_bytes())
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn local_payload_conditions_and_bundle_bindings_use_the_projected_value() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut input = fixture(root.path(), &server.endpoint, false);
    let secret = "sk-SYNTHETICpayloadoutside123456789";
    input["evidence"] =
        json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":secret}}}});
    let path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"payload_field_equals","input":"fact","pointer":"/decision","value":secret}}]);
    std::fs::write(path, rule.to_string()).unwrap();
    let outcome = report(
        &support::run_without_credential(root.path(), "evaluate_candidates", input).await,
        0,
    );
    assert_eq!(
        outcome["report"]["candidates"][0]["outcome"]["kind"],
        "dropped"
    );
    assert!(!outcome.to_string().contains(secret));
    assert_eq!(
        outcome["report"]["candidates"][0]["evidence"][0]["binding"]["binding"]["value"]["decision"],
        "[redacted credential]"
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn outside_transcript_analysis_and_reasoning_follow_the_rules_grants() {
    for granted in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = fixture(root.path(), &server.endpoint, false);
        let fact = [
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","channel":"analysis","content":[{"type":"output_text","text":"OUTSIDE_ANALYSIS_SENTINEL"}]}}),
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","channel":"final","content":[{"type":"output_text","text":"OUTSIDE_PUBLIC_FACT"}]}}),
            json!({"type":"response_item","payload":{"type":"reasoning","encrypted_content":"OUTSIDE_OPAQUE_ENCRYPTED_SENTINEL","summary":[{"type":"summary_text","text":"OUTSIDE_REASONING_SENTINEL"}]}}),
        ].map(|line| line.to_string()+"\n").concat();
        let fact_path = root.path().join("fact.jsonl");
        std::fs::write(&fact_path, &fact).unwrap();
        let set_path = root.path().join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&set_path).unwrap()).unwrap();
        set["inputs"]["fact"] = json!({"kind":"conversation"});
        std::fs::write(set_path, set.to_string()).unwrap();
        let path = root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["evidence"][1]["select"] = rule["evidence"][0]["select"].clone();
        if granted {
            rule["private_access"] = json!(["analysis_channel", "encrypted_reasoning"]);
        }
        std::fs::write(path, rule.to_string()).unwrap();
        input["payload_schemas"] = json!([]);
        input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"conversation","transcript_path":fact_path,"format":"codex_rollout_jsonl"}}});
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let request = tokio::select! {
            output = &mut run => panic!("outside transcript did not reach judge: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request = server.next() => request,
        };
        let body = request.body.to_string();
        assert!(body.contains("OUTSIDE_PUBLIC_FACT"));
        assert_eq!(body.contains("OUTSIDE_ANALYSIS_SENTINEL"), granted);
        assert_eq!(body.contains("OUTSIDE_REASONING_SENTINEL"), granted);
        assert!(!body.contains("OUTSIDE_OPAQUE_ENCRYPTED_SENTINEL"));
        request.noul(0.9);
        let outcome = report(&run.await.unwrap(), 0);
        assert_eq!(
            outcome["report"]["candidates"][0]["outcome"]["kind"],
            "finding"
        );
        assert!(
            !outcome
                .to_string()
                .contains("OUTSIDE_OPAQUE_ENCRYPTED_SENTINEL")
        );
        let entry = &outcome["report"]["candidates"][0];
        if granted {
            for kind in ["analysis_channel", "encrypted_reasoning"] {
                assert!(
                    outcome["report"]["privacy"]["granted"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(
                            |grant| grant["rule"] == entry["rule"] && grant["private_kind"] == kind
                        )
                );
            }
            let mut other_scopes = outcome.clone();
            other_scopes["report"]["candidates"] = json!([]);
            assert!(
                !other_scopes
                    .to_string()
                    .contains("OUTSIDE_ANALYSIS_SENTINEL")
            );
            assert!(
                !other_scopes
                    .to_string()
                    .contains("OUTSIDE_REASONING_SENTINEL")
            );
        } else {
            assert!(!outcome.to_string().contains("OUTSIDE_ANALYSIS_SENTINEL"));
            assert!(!outcome.to_string().contains("OUTSIDE_REASONING_SENTINEL"));
            assert_eq!(outcome["report"]["privacy"]["granted"], json!([]));
        }
        assert_eq!(
            outcome["report"]["candidates"][0]["evidence"][0]["content_identity"],
            sunstack_evidence::content_digest(fact.as_bytes())
        );
        if !granted {
            for kind in ["analysis_channel", "encrypted_reasoning"] {
                assert!(
                    outcome["report"]["privacy"]["left_out"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|count| count
                            == &json!({"route":"outside_evidence","private_kind":kind,"count":1}))
                );
            }
        }
        assert_eq!(server.finish().await, 1);
    }
}
