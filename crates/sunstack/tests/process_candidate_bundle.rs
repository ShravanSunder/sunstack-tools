#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};
use std::path::Path;

fn write_bundle(root: &Path, output: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let path = root.join("bundle.json");
    std::fs::write(&path, &output.stdout).unwrap();
    serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["result"]["outcome"]["report"].clone()
}

// This walk has only the persisted bundle and source files; the responder ledger is gone.
fn walk_bundle(root: &Path) -> Value {
    let wire: Value =
        serde_json::from_slice(&std::fs::read(root.join("bundle.json")).unwrap()).unwrap();
    let report = &wire["result"]["outcome"]["report"];
    let mut seen = std::collections::BTreeSet::new();
    assert_eq!(report["candidates"].as_array().unwrap().len(), 5);
    for entry in report["candidates"].as_array().unwrap() {
        let occurrence = &entry["occurrence"];
        let record = occurrence["record"].as_str().unwrap();
        let input = report["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|input| record.starts_with(input["input_id"].as_str().unwrap()))
            .unwrap();
        let source = std::fs::read_to_string(input["location"]["path"].as_str().unwrap()).unwrap();
        let line = occurrence["matched_at"]["line"].as_u64().unwrap() as usize;
        let raw: Value = serde_json::from_str(source.lines().nth(line - 1).unwrap()).unwrap();
        let text = raw["message"]["content"].as_str().unwrap();
        assert!(text.starts_with("MATCH BUNDLE_"));
        let index = text
            .strip_prefix("MATCH BUNDLE_")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert!(seen.insert(index), "duplicate source entry {index}");
        assert_eq!(entry["candidate_id"]["occurrence"], occurrence["record"]);
        assert_eq!(entry["candidate_id"]["rule"], entry["rule"]);
        let answers = entry["answers"].as_array().unwrap();
        let resolve = |id: &Value| {
            answers
                .iter()
                .find(|answer| answer["answer_id"] == *id)
                .unwrap()
        };
        match entry["outcome"]["kind"].as_str().unwrap() {
            "finding" => {
                assert!(text.ends_with("0"));
                assert_eq!(
                    answers
                        .iter()
                        .map(|answer| answer["question"]["kind"].as_str().unwrap())
                        .collect::<std::collections::BTreeSet<_>>(),
                    std::collections::BTreeSet::from(["router", "narrowing", "verdict"])
                );
                let expected = ["router", "narrowing", "verdict"].map(|kind| {
                    answers
                        .iter()
                        .find(|answer| answer["question"]["kind"] == kind)
                        .unwrap()["answer_id"]
                        .clone()
                });
                assert_eq!(entry["outcome"]["path"], json!(expected));
                for id in entry["outcome"]["path"].as_array().unwrap() {
                    assert_eq!(resolve(id)["settlement"], "passed");
                }
            }
            "dropped" => {
                assert!(text.ends_with("1"));
                assert_eq!(entry["outcome"]["at"]["kind"], "narrowing");
                let answer = resolve(&entry["outcome"]["answer"]);
                assert_eq!(answer["question"], entry["outcome"]["at"]);
                assert_eq!(answer["answer"]["yes_probability"], 0.1);
            }
            "uncertain" => {
                assert!(text.ends_with("2"));
                let verdict = answers
                    .iter()
                    .find(|answer| answer["question"]["kind"] == "verdict")
                    .unwrap();
                assert_eq!(entry["outcome"]["at"], json!({"kind":"verdict"}));
                assert_eq!(entry["outcome"]["answers"], json!([verdict["answer_id"]]));
                assert_eq!(verdict["settlement"], "uncertain");
                for id in entry["outcome"]["answers"].as_array().unwrap() {
                    assert_eq!(resolve(id)["answer"]["yes_probability"], 0.55);
                }
            }
            "unresolved" => {
                assert!(text.ends_with("3"));
                assert_eq!(
                    entry["outcome"]["missing"],
                    json!({"kind":"evidence_unavailable","evidence":"fact","reason":"withheld"})
                );
            }
            "not_processed" => {
                assert!(text.ends_with("4"));
                assert_eq!(entry["outcome"]["reason"], "evaluation_failed");
            }
            other => panic!("unexpected outcome {other}"),
        }
        let evidence = &entry["evidence"][0];
        assert_eq!(evidence["input"], "fact");
        if text.ends_with("3") {
            assert_eq!(
                evidence["binding"],
                json!({"kind":"unavailable","reason":"withheld"})
            );
            assert!(evidence["content_identity"].is_null());
        } else {
            let value = &evidence["binding"]["binding"]["value"];
            assert_eq!(value, &json!({"decision":"PUBLIC_OUTSIDE"}));
            assert_eq!(
                evidence["content_identity"],
                sunstack_evidence::content_digest(&serde_json::to_vec(value).unwrap())
            );
        }
    }
    assert_eq!(seen, std::collections::BTreeSet::from([0, 1, 2, 3, 4]));
    report.clone()
}

#[tokio::test]
async fn saved_bundle_resolves_all_five_outcomes_and_native_usage() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = evidence_fixture::fixture(root.path(), &server.endpoint, false);
    input["sources"] = json!([]);
    for index in 0..5 {
        let session = format!("bundle{index}");
        std::fs::write(root.path().join(format!("{session}.jsonl")),json!({"type":"assistant","message":{"id":session,"content":format!("MATCH BUNDLE_{index}")}}).to_string()+"\n").unwrap();
        let mut entry =
            json!({"source":{"kind":"session_pair","provider":"claude_code","session":session}});
        if index == 3 {
            entry["evidence"] = json!({"fact":{"kind":"unavailable","reason":"withheld"}});
        }
        input["sources"].as_array_mut().unwrap().push(entry);
    }
    input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"PUBLIC_OUTSIDE"}}}});
    let path = root.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["verdict"]["threshold"] = json!(0.5);
    rule["verdict"]["uncertain_below"] = json!(0.7);
    rule["narrowing"] = json!({"question":{"type":"noul","instructions":"NARROW this occurrence"},"evidence":rule["evidence"],"threshold":0.8});
    std::fs::write(&path, rule.to_string()).unwrap();
    std::fs::write(root.path().join("router.json"),json!({"schema_version":1,"name":"bundle router","description":"route each occurrence","inputs":{"subject":{"kind":"conversation"},"fact":{"kind":"payload","schema":"urn:owner-fact"}},"evidence":rule["evidence"],"question":{"type":"choice","instructions":"route","criteria":{"all":"target"}},"routes":{"all":[{"rule_set":"set.json","inputs":{"subject":"subject","fact":"fact"}}]},"confidence_floor":0.7}).to_string()).unwrap();
    input["definitions"] = json!({"kind":"router","path":"router.json"});
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let mut seen = std::collections::BTreeSet::new();
    let output = loop {
        tokio::select! {
            output=&mut run=>break output.unwrap(),
            request=server.next()=>{
                let body=request.body.to_string();
                let cases=(0..5).filter(|index|body.contains(&format!("BUNDLE_{index}"))).collect::<Vec<_>>();
                assert_eq!(cases.len(),1);
                assert!(seen.insert(cases[0]));
                if cases[0]==4 {request.respond(401,json!({"error":{"message":"synthetic unavailable provider"}}));continue;}
                assert_ne!(cases[0],3,"withheld evidence reached model");
                let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)| {
                    (name.clone(),if question["type"]=="choice" {json!({"type":"choice","choice":"all","confidence":0.91,"probabilities":{"all":1.0}})} else {json!({"type":"noul","noul":if cases[0]==1 && question.to_string().contains("NARROW") {0.1} else if cases[0]==2 && !question.to_string().contains("NARROW") {0.55} else {0.9}})})
                }).collect::<serde_json::Map<_,_>>();
                request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
            }
        }
    };
    let report = write_bundle(root.path(), &output, 0);
    assert_eq!(server.finish().await, 4);
    assert_eq!(seen, std::collections::BTreeSet::from([0, 1, 2, 4]));
    assert_eq!(report["candidates"].as_array().unwrap().len(), 5);
    assert_eq!(
        report["completeness"],
        json!({"kind":"findings","count":1,"inconclusive":3})
    );
    assert_eq!(report["usage"]["requests"], 4);
    assert_eq!(report["usage"]["attempts"], 4);
    assert_eq!(report["usage"]["input_tokens"], 60);
    assert_eq!(report["usage"]["output_tokens"], 15);
    assert!(report["usage"]["cost_usd"].is_null());
    assert_eq!(walk_bundle(root.path()), report);
}

#[tokio::test]
async fn headlines_preserve_empty_unknown_missing_input_and_two_of_five_request_limit() {
    for mode in ["empty", "dropped", "uncertain", "missing", "limited"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = evidence_fixture::record_fixture(root.path(), &server.endpoint);
        input["sources"].as_array_mut().unwrap().truncate(1);
        let lines=(0..5).map(|index|json!({"type":"assistant","message":{"id":format!("case{index}"),"content":format!("{} HEADLINE_{index}",if mode=="empty" {"ordinary"} else {"MATCH"})}}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
        std::fs::write(root.path().join("first.jsonl"), lines).unwrap();
        let path = root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["verdict"]["threshold"] = json!(0.5);
        rule["verdict"]["uncertain_below"] = json!(0.7);
        std::fs::write(path, rule.to_string()).unwrap();
        if mode == "missing" {
            input["sources"].as_array_mut().unwrap().push(json!({"source":{"kind":"session_pair","provider":"claude_code","session":"does-not-exist"}}));
        }
        if mode == "limited" {
            input["request_budget"]["max_requests"] = json!(2);
        }
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let output = loop {
            tokio::select! {
                output=&mut run=>break output.unwrap(),
                request=server.next()=>request.noul(if mode=="uncertain" {0.55} else if mode=="dropped" {0.1} else {0.9}),
            }
        };
        let report = write_bundle(
            root.path(),
            &output,
            if matches!(mode, "missing" | "limited") {
                3
            } else {
                0
            },
        );
        let expected = match mode {
            "empty" | "dropped" => json!({"kind":"nothing_found"}),
            "uncertain" => {
                json!({"kind":"inconclusive","uncertain":5,"unresolved":0,"not_processed":0})
            }
            "missing" => {
                json!({"kind":"partial","stopped_by":"unlocatable_input","findings":5,"inconclusive":0})
            }
            "limited" => {
                json!({"kind":"partial","stopped_by":"request_limit","findings":2,"inconclusive":3})
            }
            _ => unreachable!(),
        };
        assert_eq!(report["completeness"], expected, "{report}");
        let count = if mode == "empty" {
            0
        } else if mode == "limited" {
            2
        } else {
            5
        };
        assert_eq!(
            report["candidates"].as_array().unwrap().len(),
            if mode == "empty" { 0 } else { 5 }
        );
        assert_eq!(report["usage"]["requests"], count);
        assert_eq!(report["usage"]["attempts"], count);
        assert_eq!(report["usage"]["input_tokens"], count * 20);
        assert_eq!(report["usage"]["output_tokens"], count * 5);
        if count > 0 {
            assert!(
                (report["usage"]["cost_usd"].as_f64().unwrap() - count as f64 * 0.001).abs()
                    < 1e-10
            );
        }
        if mode == "limited" {
            assert_eq!(
                report["candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| entry["outcome"]
                        == json!({"kind":"not_processed","reason":"run_stopped"}))
                    .count(),
                3
            );
        }
        assert_eq!(server.finish().await, count);
    }
}
