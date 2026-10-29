#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::{controlled_http::ControlledServer, support};
#[path = "signal_process.rs"]
mod signal_process;
use serde_json::{Value, json};
use std::path::Path;
use tokio::{io::AsyncWriteExt, task::JoinHandle};

pub(super) fn candidate_line(entry: &Value) -> usize {
    entry["occurrence"]["positions"][0]["line"]
        .as_u64()
        .unwrap() as usize
}

pub(super) fn narrowing_answer(entry: &Value) -> &Value {
    entry["answers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|answer| answer["question"]["kind"] == "narrowing")
        .unwrap()
}

pub(super) fn assert_clear_narrowing_drop_answers_join(report: &Value) {
    for candidate in report["candidates"].as_array().unwrap() {
        if candidate["outcome"]["kind"] != "dropped"
            || candidate["outcome"]["at"]["kind"] != "narrowing"
        {
            continue;
        }
        let answer_id = &candidate["outcome"]["answer"];
        let answer = candidate["answers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|answer| {
                answer["question"]["kind"] == "narrowing" && &answer["answer_id"] == answer_id
            })
            .unwrap_or_else(|| panic!("narrowing outcome answer is not retained: {candidate}"));
        assert_eq!(answer["answer"]["type"], "noul");
        assert_eq!(answer["answer"]["yes_probability"], 0.1);
        assert_eq!(answer["settlement"], "did_not_pass");
    }
}

pub(super) fn sample_line(sample: &Value) -> usize {
    candidate_line(sample)
}

pub(super) async fn start_candidate_process(
    root: &Path,
    input: Value,
) -> JoinHandle<std::process::Output> {
    let (mut child, stderr, transcript) = signal_process::ready_process(root).await;
    let request_bytes = serde_json::to_vec(&json!({
        "protocol_version":1,
        "operation":"evaluate_candidates",
        "input":input
    }))
    .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(&request_bytes).await.unwrap();
    drop(stdin);
    let evidence_root = root.to_owned();
    tokio::spawn(async move {
        let output = signal_process::collect_output(child, stderr, transcript).await;
        support::record_output(&evidence_root, &request_bytes, &output);
        output
    })
}

pub(super) async fn answer_until_process_finishes(
    server: &mut ControlledServer,
    run: &mut JoinHandle<std::process::Output>,
    mut response_for: impl FnMut(&Value) -> Value,
) -> std::process::Output {
    loop {
        tokio::select! {
            output = &mut *run => return output.unwrap(),
            request = server.next() => {
                let body = request.body.clone();
                let response = response_for(&body);
                request.respond(200, response);
            }
        }
    }
}

pub(super) fn noul_response(
    body: &Value,
    probability_for_question: impl Fn(&Value) -> f64,
) -> Value {
    let answers = body["questions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, question)| {
            assert_eq!(question["type"], "noul", "unexpected question: {question}");
            (
                name.clone(),
                json!({"type":"noul","noul":probability_for_question(question)}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
}

pub(super) fn report_from_process_output(output: &std::process::Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["result"]["operation"], "evaluate_candidates");
    let report = wire["result"]["outcome"]["report"].clone();
    assert_clear_narrowing_drop_answers_join(&report);
    report
}

pub(super) fn add_narrowing_question(root: &Path, instructions: &str) {
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let evidence = rule["evidence"].clone();
    rule["narrowing"] = json!({
        "question":{"type":"noul","instructions":instructions},
        "evidence":evidence,
        "threshold":0.8
    });
    std::fs::write(path, rule.to_string()).unwrap();
}

pub(super) fn response_for_clear_code_or_document_drop(body: &Value) -> Value {
    noul_response(body, |question| {
        if question.to_string().contains("NARROW_") {
            0.1
        } else {
            0.9
        }
    })
}
