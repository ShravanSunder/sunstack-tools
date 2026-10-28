#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/eval_process_fixture.rs"]
mod eval_fixture;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use eval_fixture::{EvalFixture, LABEL_SENTINEL, response};
use transport::{AnswerServer, ReplyMode, RouterReply};

#[tokio::test]
async fn binary_eval_replays_frozen_definitions_keeps_source_facts_and_excludes_all_label_text() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture = EvalFixture::new(&server.endpoint);
    fixture.assert_label_confinement();
    let frozen = sunstack_evals::load_case_set(&fixture.manifest).unwrap();
    let expected_rule = serde_json::to_value(&frozen.manifest().rule).unwrap();
    let expected_case =
        sunstack_evidence::content_digest(&std::fs::read(&fixture.manifest).unwrap());
    let expected_version = frozen.rule_version().as_str().to_owned();
    fixture.mutate_live_rule();
    let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["outcome"]["kind"], "eval_report");
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["rule"], expected_rule);
    assert_eq!(report["case_set"], expected_case);
    assert_eq!(report["rule_version"], expected_version);
    assert_eq!(report["filter"]["positive_cases"], 1);
    assert_eq!(report["filter"]["kept"], 1);
    assert_eq!(
        report["end_result"]["true_findings"],
        serde_json::json!([LABEL_SENTINEL])
    );
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 1);
    for body in bodies {
        let serialized = body.to_string();
        assert!(!serialized.contains(LABEL_SENTINEL));
        assert!(!serialized.contains("LIVE_RULE_MUTATION_NEVER_USED"));
        assert!(serialized.contains("MATCH public occurrence"));
        assert!(serialized.contains("OWNER_PUBLIC_BOUND_FACT"));
    }
}

#[tokio::test]
async fn stale_label_hash_and_each_retired_definition_field_refuse_before_any_dispatch() {
    use eval_fixture::{read_json, write_json};
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture = EvalFixture::new(&server.endpoint);
    for field in ["definitions", "definition_root", "payload_schemas"] {
        let mut input = fixture.input.clone();
        input[field] = serde_json::json!({});
        let output =
            support::run_without_credential(fixture.root.path(), "score_eval", input).await;
        let wire = response(&output, 2);
        assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
        assert!(
            wire["result"]["outcome"]["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "unknown_field"
                    && diagnostic["location"]["field_path"] == serde_json::json!([field]))
        );
    }
    let path = fixture.labels_path();
    let mut labels = read_json(&path);
    labels["label"] = serde_json::json!("no");
    write_json(&path, &labels);
    let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(LABEL_SENTINEL));
    assert_eq!(server.finish().await.0, 0);
}

#[tokio::test]
async fn integrity_valid_invalid_labels_are_interpreted_only_after_a_real_request() {
    use eval_fixture::{read_json, write_json};
    let mut server = transport::controlled::ControlledServer::start().await;
    let fixture = EvalFixture::new(&server.endpoint);
    let path = fixture.labels_path();
    let mut labels = read_json(&path);
    labels["label"] = serde_json::json!("not_a_valid_label");
    write_json(&path, &labels);
    let mut manifest = read_json(&fixture.manifest);
    manifest["labels"]["content_identity"] = serde_json::json!(sunstack_evidence::content_digest(
        &std::fs::read(&path).unwrap()
    ));
    write_json(&fixture.manifest, &manifest);
    let mut running = start_controlled_eval(&fixture).await;
    let request = tokio::select! {output=&mut running=>panic!("labels were interpreted before dispatch: {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
    assert!(!request.body.to_string().contains(LABEL_SENTINEL));
    request.noul(0.9);
    let output = running.await.unwrap();
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn a_label_file_changed_after_dispatch_cannot_replace_the_original_verified_label_bytes() {
    use eval_fixture::{read_json, write_json};
    let mut server = transport::controlled::ControlledServer::start().await;
    let fixture = EvalFixture::new(&server.endpoint);
    let mut running = start_controlled_eval(&fixture).await;
    let request = tokio::select! {output=&mut running=>panic!("request was not reached: {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
    assert!(!request.body.to_string().contains(LABEL_SENTINEL));
    let path = fixture.labels_path();
    let mut labels = read_json(&path);
    labels["label"] = serde_json::json!("no");
    write_json(&path, &labels);
    request.noul(0.9);
    let output = running.await.unwrap();
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["outcome"]["kind"], "eval_report");
    assert_eq!(
        wire["result"]["outcome"]["report"]["end_result"]["true_findings"],
        serde_json::json!([LABEL_SENTINEL])
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn the_frozen_unavailable_fact_overrides_a_bound_run_wide_fallback() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture =
        EvalFixture::configured(&server.endpoint, 1, eval_fixture::FrozenFact::Unavailable);
    let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
    let wire = response(&output, 0);
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(
        report["end_result"]["inconclusive"],
        serde_json::json!([LABEL_SENTINEL])
    );
    assert_eq!(
        report["steps"][0]["unknown"],
        serde_json::json!([LABEL_SENTINEL])
    );
    assert_eq!(report["cost"]["requests"], 0);
    for step in report["steps"].as_array().unwrap() {
        assert_eq!(step["wrong"], serde_json::json!([]));
        assert_eq!(step["correct"], serde_json::json!([]));
    }
    for field in [
        "true_findings",
        "false_findings",
        "correct_negatives",
        "missed_findings",
    ] {
        assert_eq!(report["end_result"][field], serde_json::json!([]));
    }
    assert_eq!(server.finish().await.0, 0);
}

#[tokio::test]
async fn deserializable_labels_with_an_unbound_source_are_validated_after_the_bundle_returns() {
    use eval_fixture::{read_json, write_json};
    let mut server = transport::controlled::ControlledServer::start().await;
    let fixture = EvalFixture::new(&server.endpoint);
    let path = fixture.labels_path();
    let mut labels = read_json(&path);
    labels["source"] = serde_json::json!("unbound_source");
    serde_json::from_value::<sunstack_evals::LabeledCase>(labels.clone()).unwrap();
    write_json(&path, &labels);
    let mut manifest = read_json(&fixture.manifest);
    manifest["labels"]["content_identity"] = serde_json::json!(sunstack_evidence::content_digest(
        &std::fs::read(&path).unwrap()
    ));
    write_json(&fixture.manifest, &manifest);
    let mut running = start_controlled_eval(&fixture).await;
    let request = tokio::select! {output=&mut running=>panic!("labels were validated before dispatch: {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
    assert!(!request.body.to_string().contains(LABEL_SENTINEL));
    request.noul(0.9);
    let output = running.await.unwrap();
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["location"]["field_path"] == serde_json::json!(["labels", "source"]))
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn a_request_budget_stopping_the_replay_returns_an_incomplete_report_with_known_progress() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let mut fixture =
        EvalFixture::configured(&server.endpoint, 2, eval_fixture::FrozenFact::Document);
    fixture.input["request_budget"]["max_requests"] = serde_json::json!(1);
    let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
    let wire = response(&output, 3);
    assert_eq!(wire["result"]["outcome"]["kind"], "incomplete");
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["filter"]["positive_cases"], 2);
    assert_eq!(report["filter"]["kept"], 2);
    assert_eq!(
        report["end_result"]["true_findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["end_result"]["inconclusive"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(report["cost"]["requests"], 1);
    assert_eq!(server.finish().await.0, 1);
}

#[tokio::test]
async fn frozen_payload_schema_accepts_and_rejects_values_even_when_the_live_schema_says_the_opposite()
 {
    use eval_fixture::FrozenFact;
    for accepted in [true, false] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let fact = FrozenFact::Payload(if accepted {
            serde_json::json!({"decision":"approved"})
        } else {
            serde_json::json!({"decision":17})
        });
        let fixture = EvalFixture::configured(&server.endpoint, 1, fact);
        std::fs::write(
            fixture.root.path().join("fact-schema.json"),
            if accepted { "{\"not\":{}}" } else { "{}" },
        )
        .unwrap();
        let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
        let wire = response(&output, if accepted { 0 } else { 2 });
        if accepted {
            assert_eq!(
                wire["result"]["outcome"]["report"]["end_result"]["true_findings"],
                serde_json::json!([LABEL_SENTINEL])
            );
        } else {
            assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
        }
        let (count, bodies) = server.finish().await;
        assert_eq!(count, usize::from(accepted));
        if accepted {
            assert!(
                bodies[0]["state"]["contents"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|content| content["decision"] == "approved")
            );
            assert!(!bodies[0].to_string().contains(LABEL_SENTINEL));
        }
    }
}

#[tokio::test]
async fn tampered_frozen_schema_refuses_through_the_binary_before_dispatch() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture = EvalFixture::configured(
        &server.endpoint,
        1,
        eval_fixture::FrozenFact::Payload(serde_json::json!({"decision":"approved"})),
    );
    std::fs::write(
        fixture
            .manifest
            .parent()
            .unwrap()
            .join("definitions/fact-schema.json"),
        "{}",
    )
    .unwrap();
    let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert_eq!(server.finish().await.0, 0);
}

// Observe the existing application-ready event before starting the HTTP receipt deadline.
// Signing and process startup belong to readiness; the HTTP deadline still bounds dispatch.
async fn start_controlled_eval(
    fixture: &EvalFixture,
) -> tokio::task::JoinHandle<std::process::Output> {
    use tokio::io::AsyncWriteExt;
    let directory = fixture.root.path().to_owned();
    let bytes = serde_json::to_vec(&serde_json::json!({
        "protocol_version":1,"operation":"score_eval","input":fixture.input
    }))
    .unwrap();
    let (mut process, stderr, transcript) = signal_process::ready_process(&directory).await;
    tokio::spawn(async move {
        let mut stdin = process.stdin.take().unwrap();
        stdin.write_all(&bytes).await.unwrap();
        drop(stdin);
        let output = signal_process::collect_output(process, stderr, transcript).await;
        support::record_output(&directory, &bytes, &output);
        output
    })
}
