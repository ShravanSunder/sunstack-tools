#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

async fn judge_one(
    rule_edit: impl FnOnce(&mut Value),
    response: impl FnOnce(&Value) -> Value,
) -> Value {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule_edit(&mut rule);
    std::fs::write(path, rule.to_string()).unwrap();
    std::fs::write(
        root.path().join("first.jsonl"),
        format!(
            "{}\n",
            json!({"type":"assistant","message":{"id":"one","content":"MATCH certainty evidence"}})
        ),
    )
    .unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = tokio::select! {
        output=&mut run=>panic!("operation stopped before judge: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
        request=server.next()=>request,
    };
    let body = response(&request.body);
    request.respond(200, body);
    let output = tokio::select! {
        output=&mut run=>output.unwrap(),
        unexpected=server.next()=>panic!("another request escaped the settled gate: {}",unexpected.body),
    };
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(server.finish().await, 1);
    wire["result"]["outcome"]["report"].clone()
}
fn noul_response(body: &Value, probability: f64) -> Value {
    let answers = body["questions"]
        .as_object()
        .unwrap()
        .keys()
        .map(|name| (name.clone(), json!({"type":"noul","noul":probability})))
        .collect::<serde_json::Map<_, _>>();
    json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
}

#[tokio::test]
async fn the_band_preserves_native_probabilities_and_leaves_binary_questions_binary() {
    let binary = judge_one(
        |rule| {
            rule["verdict"]["threshold"] = json!(0.5);
        },
        |body| noul_response(body, 0.55),
    )
    .await;
    assert_eq!(binary["candidates"][0]["outcome"]["kind"], "finding");
    for (probability, kind, settlement) in [
        (0.55, "uncertain", "uncertain"),
        (0.9, "finding", "passed"),
        (0.7, "finding", "passed"),
        (0.1, "dropped", "did_not_pass"),
    ] {
        let report = judge_one(
            |rule| {
                rule["verdict"]["threshold"] = json!(0.5);
                rule["verdict"]["uncertain_below"] = json!(0.7);
            },
            |body| noul_response(body, probability),
        )
        .await;
        let candidate = &report["candidates"][0];
        assert_eq!(candidate["outcome"]["kind"], kind);
        assert_eq!(
            candidate["answers"][0]["answer"]["yes_probability"],
            json!(probability)
        );
        assert_eq!(candidate["answers"][0]["settlement"], settlement);
        if kind == "uncertain" {
            assert_eq!(report["completeness"]["kind"], "inconclusive");
        }
    }
}

#[tokio::test]
async fn banded_gates_use_their_native_answer_and_discard_unusable_carried_verdicts() {
    for (probability, kind, settlement) in [
        (0.55, "uncertain", "uncertain"),
        (0.9, "finding", "passed"),
        (0.1, "dropped", "did_not_pass"),
    ] {
        let report = judge_one(|rule| {
            let evidence = rule["evidence"].clone();
            rule["applicability"] = json!([{"kind":"decision","question":{"type":"noul","instructions":"GATE_CERTAINTY"},"evidence":evidence,"threshold":0.5,"uncertain_below":0.7}]);
        }, |body| {
            let questions = body["questions"].as_object().unwrap();
            // Same-source questions share the existing carrier, even with speculation off.
            assert_eq!(questions.len(), 2);
            let gate_count = questions.values().filter(|question| question["instructions"].to_string().contains("GATE_CERTAINTY")).count();
            assert_eq!(gate_count, 1);
            assert_eq!(questions.values().filter(|question| question["instructions"].to_string().contains("Is this occurrence a finding?")).count(), 1);
            let answers = questions.iter().map(|(name, question)| {
                let is_gate = question["instructions"].to_string().contains("GATE_CERTAINTY");
                (name.clone(), json!({"type":"noul","noul":if is_gate {probability} else {0.99}}))
            }).collect::<serde_json::Map<_, _>>();
            json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
        }).await;
        let entry = &report["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], kind);
        let answers = entry["answers"].as_array().unwrap();
        let gate = answers
            .iter()
            .find(|answer| answer["question"] == json!({"kind":"rule_gate","position":0}))
            .unwrap();
        assert_eq!(gate["answer"]["yes_probability"], probability);
        assert_eq!(gate["settlement"], settlement);
        if kind == "finding" {
            assert_eq!(answers.len(), 2);
            let verdict = answers
                .iter()
                .find(|answer| answer["question"] == json!({"kind":"verdict"}))
                .unwrap();
            assert_eq!(verdict["answer"]["yes_probability"], 0.99);
            assert!(
                entry["outcome"]["path"]
                    .as_array()
                    .unwrap()
                    .contains(&verdict["answer_id"])
            );
        } else {
            assert_eq!(
                answers.len(),
                1,
                "the unusable carried verdict must be discarded"
            );
            assert_eq!(
                entry["outcome"]["at"],
                json!({"kind":"rule_gate","position":0})
            );
            if kind == "dropped" {
                assert_eq!(entry["outcome"]["answer"], gate["answer_id"]);
            } else {
                assert_eq!(entry["outcome"]["answers"], json!([gate["answer_id"]]));
            }
        }
        // judge_one also rejects any subsequent HTTP request and requires exactly one dispatch.
    }
}

#[tokio::test]
async fn composite_uncertainty_uses_each_reported_certainty_never_the_weighted_score() {
    for (confidence, score, probability, kind) in [
        (0.69, 1.0, 0.95, "uncertain"),
        (0.9, 1.0, 0.95, "finding"),
        (0.9, 0.0, 0.1, "dropped"),
    ] {
        let report=judge_one(|rule|{
            rule["questions"]=json!({"yes":{"type":"noul","instructions":"binary dimension"},"quality":{"type":"score","instructions":"score dimension","criteria":["low","high"]}});
            rule["verdict"]=json!({"kind":"composite","dimensions":[{"question":"yes","weight":0.5},{"question":"quality","weight":0.5}],"threshold":0.8,"uncertain_below":0.7});
        },|body|{
            let answers=body["questions"].as_object().unwrap().iter().map(|(name,question)|{
                let answer=if question["type"]=="score" {
                    json!({"type":"score","score":score,"confidence":confidence,"legend":{"0":"low","1":"high"},"probabilities":{"0":1.0-score,"1":score}})
                } else {json!({"type":"noul","noul":probability})};
                (name.clone(),answer)
            }).collect::<serde_json::Map<_,_>>();
            json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
        }).await;
        let entry = &report["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], kind);
        let answers = entry["answers"].as_array().unwrap();
        assert_eq!(answers.len(), 3);
        let logical = answers
            .iter()
            .find(|answer| answer["question"] == json!({"kind":"verdict"}))
            .unwrap();
        assert!(logical["answer"].is_null());
        if kind == "dropped" {
            assert_eq!(entry["outcome"]["answer"], logical["answer_id"]);
        }
        if kind == "finding" {
            assert!(
                entry["outcome"]["path"]
                    .as_array()
                    .unwrap()
                    .contains(&logical["answer_id"])
            );
        }
        let quality = answers
            .iter()
            .find(|answer| answer["question"] == json!({"kind":"informational","name":"quality"}))
            .unwrap();
        assert_eq!(quality["answer"]["confidence"], confidence);
        assert_eq!(
            quality["settlement"],
            if confidence < 0.7 {
                "uncertain"
            } else {
                "passed"
            }
        );
        if kind == "uncertain" {
            assert_eq!(entry["outcome"]["answers"].as_array().unwrap().len(), 2);
        }
    }
}

#[path = "../../sunstack-rules/tests/support/rule_fixture.rs"]
mod rule_fixture;

#[tokio::test]
async fn old_rule_operations_keep_binary_settlement_with_the_new_optional_floor() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut rule = rule_fixture::rule("binary");
    rule["verdict"]["threshold"] = json!(0.5);
    rule["verdict"]["uncertain_below"] = json!(0.7);
    let mut gate = rule_fixture::decision("subject");
    gate["threshold"] = json!(0.5);
    gate["uncertain_below"] = json!(0.7);
    rule["applicability"] = json!([gate]);
    rule_fixture::write_set(root.path(), &[rule], vec![]);
    let input = serde_json::to_value(rule_fixture::input(root.path(), &server.endpoint)).unwrap();
    let preparation = support::run(root.path(), "prepare_rules", input.clone()).await;
    assert_eq!(
        preparation.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&preparation.stdout)
    );
    let directory = root.path().to_owned();
    let run = tokio::spawn(async move { support::run(&directory, "evaluate_rules", input).await });
    let request = server.next().await;
    let body = noul_response(&request.body, 0.55);
    request.respond(200, body);
    let output = run.await.unwrap();
    assert_eq!(output.status.code(), Some(0));
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(wire["result"]["outcome"]["kind"], "complete_report");
    assert_eq!(
        report["occurrences"][0]["judgment"]["verdict"]["result"]["kind"],
        "pass"
    );
    assert_eq!(
        report["groups"][0]["entries"][0]["result"]["outcome"]["kind"],
        "passed"
    );
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn invalid_confidence_floors_refuse_before_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = candidate_fixture::fixture(root.path(), &server.endpoint);
    let path = root.path().join("rules.jsonl");
    let original: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for floor in [-0.01, 1.01] {
        let mut rule = original.clone();
        rule["verdict"]["uncertain_below"] = json!(floor);
        std::fs::write(&path, rule.to_string()).unwrap();
        let output =
            support::run_without_credential(root.path(), "evaluate_candidates", input.clone())
                .await;
        assert_eq!(output.status.code(), Some(2));
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    }
    assert_eq!(server.finish().await, 0);
}
