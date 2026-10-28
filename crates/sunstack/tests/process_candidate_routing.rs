#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn narrowing_keeps_all_five_entries_and_identifies_the_two_negative_answers() {
    for narrowed in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
        input["sources"].as_array_mut().unwrap().truncate(1);
        let path = root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if narrowed {
            rule["narrowing"] = json!({"question":{"type":"noul","instructions":"NARROW this candidate"},"evidence":rule["evidence"],"threshold":0.8});
        }
        std::fs::write(path, rule.to_string()).unwrap();
        let lines = (0..5).map(|index| json!({"type":"assistant","message":{"id":format!("case{index}"),"content":format!("MATCH CASE_{index}")}}).to_string()).collect::<Vec<_>>().join("\n") + "\n";
        std::fs::write(root.path().join("first.jsonl"), lines).unwrap();
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..5 {
            let request = tokio::select! {
                output=&mut run=>panic!("candidate run stopped before judge: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
                request=server.next()=>request,
            };
            let body = request.body.to_string();
            let cases = (0..5)
                .filter(|index| body.contains(&format!("CASE_{index}")))
                .collect::<Vec<_>>();
            assert_eq!(cases.len(), 1, "one candidate per request: {body}");
            assert!(seen.insert(cases[0]));
            let questions = request.body["questions"].as_object().unwrap();
            let answers = questions
                .iter()
                .map(|(name, question)| {
                    let narrowing = question.to_string().contains("NARROW");
                    assert!(!narrowing || narrowed);
                    (
                        name.clone(),
                        json!({"type":"noul","noul":if narrowing && cases[0]<2 {0.1} else {0.9}}),
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            request.respond(200, json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
        }
        let output = run.await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 5);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry["outcome"]["kind"] == "finding")
                .count(),
            if narrowed { 3 } else { 5 }
        );
        let source = std::fs::read_to_string(root.path().join("first.jsonl")).unwrap();
        for entry in entries {
            let line = entry["occurrence"]["matched_at"]["line"].as_u64().unwrap() as usize;
            let record: Value =
                serde_json::from_str(source.lines().nth(line - 1).unwrap()).unwrap();
            let index = line - 1;
            assert_eq!(record["message"]["content"], format!("MATCH CASE_{index}"));
            assert_eq!(
                entry["outcome"]["kind"],
                if narrowed && index < 2 {
                    "dropped"
                } else {
                    "finding"
                }
            );
            let verdicts = entry["answers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|answer| answer["question"]["kind"] == "verdict")
                .collect::<Vec<_>>();
            assert_eq!(verdicts.len(), usize::from(!narrowed || index >= 2));
            for verdict in verdicts {
                assert_eq!(verdict["answer"]["yes_probability"], 0.9);
            }
            let narrowing = entry["answers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|answer| answer["question"]["kind"] == "narrowing")
                .collect::<Vec<_>>();
            assert_eq!(narrowing.len(), usize::from(narrowed));
            if narrowed {
                assert_eq!(
                    narrowing[0]["answer"]["yes_probability"],
                    if index < 2 { 0.1 } else { 0.9 }
                );
            }
            if entry["outcome"]["kind"] == "dropped" {
                assert_eq!(entry["outcome"]["at"]["kind"], "narrowing");
                assert_eq!(entry["outcome"]["answer"], narrowing[0]["answer_id"]);
                assert_eq!(narrowing[0]["answer"]["yes_probability"], 0.1);
                assert!(
                    !entry["answers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|answer| answer["question"]["kind"] == "verdict")
                );
            }
        }
        assert_eq!(server.finish().await, 5);
    }
}

fn router_fixture(root: &std::path::Path, endpoint: &str) -> Value {
    let mut input = candidate_fixture::fixture(root, endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rule: Value =
        serde_json::from_slice(&std::fs::read(root.join("rules.jsonl")).unwrap()).unwrap();
    let set: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    for side in ["left", "right"] {
        let mut rule = rule.clone();
        rule["rule_id"] = json!(format!("candidate.{side}"));
        std::fs::write(root.join(format!("{side}.jsonl")), rule.to_string()).unwrap();
        let mut set = set.clone();
        set["rule_paths"] = json!([format!("{side}.jsonl")]);
        std::fs::write(root.join(format!("{side}.json")), set.to_string()).unwrap();
    }
    std::fs::write(root.join("router.json"),json!({
        "schema_version":1,"name":"per candidate","description":"route one occurrence",
        "inputs":{"router_subject":{"kind":"conversation"}},
        "evidence":[{"id":"route_context","input":"router_subject","select":rule["evidence"][0]["select"]}],
        "question":{"type":"choice","instructions":"ROUTE the one candidate","criteria":{"left":"left","right":"right","none":"neither"}},
        "routes":{"left":[{"rule_set":"left.json","inputs":{"subject":"router_subject"}}],"right":[{"rule_set":"right.json","inputs":{"subject":"router_subject"}}]},
        "confidence_floor":0.7
    }).to_string()).unwrap();
    input["definitions"] = json!({"kind":"router","path":"router.json"});
    let lines = ["LEFT", "RIGHT", "UNCERTAIN"].iter().enumerate().map(|(index,label)| json!({"type":"assistant","message":{"id":format!("case{index}"),"content":format!("MATCH ROUTE_{label}")}}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
    std::fs::write(root.join("first.jsonl"), lines).unwrap();
    input
}

#[tokio::test]
async fn each_rule_on_each_occurrence_gets_its_own_router_and_native_choice() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let input = router_fixture(root.path(), &server.endpoint);
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let mut router_questions = 0;
    let output = loop {
        tokio::select! {
            output=&mut run=>break output.unwrap(),
            request=server.next()=>{
                let questions=request.body["questions"].as_object().unwrap();
                assert!(questions.values().filter(|question|question["type"]=="choice").count()<=1,"rules must have separate router requests");
                assert!(questions.values().filter(|question|question["type"]=="noul").count()<=1,"rules must have separate verdict requests");
                let body=request.body.to_string();
                let labels=["LEFT","RIGHT","UNCERTAIN"].into_iter().filter(|label|body.contains(&format!("ROUTE_{label}"))).collect::<Vec<_>>();
                assert_eq!(labels.len(),1,"one occurrence per request: {body}");
                let label=if labels[0]=="RIGHT" {"right"} else {"left"};
                let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)|{
                    let answer=if question["type"]=="choice" {
                        router_questions+=1;
                        json!({"type":"choice","choice":label,"confidence":if labels[0]=="UNCERTAIN" {0.55} else {0.93},"probabilities":{"left":0.57,"right":0.33,"none":0.1}})
                    } else {json!({"type":"noul","noul":0.9})};
                    (name.clone(),answer)
                }).collect::<serde_json::Map<_,_>>();
                request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
            }
        }
    };
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = wire["result"]["outcome"]["report"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 6, "{wire}");
    assert_eq!(router_questions, 6);
    for kind in ["finding", "dropped", "uncertain"] {
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry["outcome"]["kind"] == kind)
                .count(),
            2,
            "{wire}"
        );
    }
    let source = std::fs::read_to_string(root.path().join("first.jsonl")).unwrap();
    for entry in entries {
        let line = entry["occurrence"]["matched_at"]["line"].as_u64().unwrap() as usize;
        let record: Value = serde_json::from_str(source.lines().nth(line - 1).unwrap()).unwrap();
        let text = record["message"]["content"].as_str().unwrap();
        let label = if text.ends_with("RIGHT") {
            "right"
        } else {
            "left"
        };
        let uncertain = text.ends_with("UNCERTAIN");
        let expected = if uncertain {
            "uncertain"
        } else if entry["rule"]["rule_id"] == format!("candidate.{label}") {
            "finding"
        } else {
            "dropped"
        };
        assert_eq!(entry["outcome"]["kind"], expected, "{entry}");
        let answers = entry["answers"].as_array().unwrap();
        let router = answers
            .iter()
            .filter(|answer| answer["question"]["kind"] == "router")
            .collect::<Vec<_>>();
        assert_eq!(router.len(), 1, "{entry}");
        let native = &router[0]["answer"];
        assert_eq!(native["selected_label"], label);
        if expected == "finding" {
            assert!(
                entry["outcome"]["path"]
                    .as_array()
                    .unwrap()
                    .contains(&router[0]["answer_id"])
            );
        }
        assert_eq!(
            native["probabilities"],
            json!({"left":0.57,"right":0.33,"none":0.1})
        );
        if entry["outcome"]["kind"] == "uncertain" {
            assert_eq!(native["confidence"], 0.55);
            assert_eq!(entry["outcome"]["at"]["kind"], "router");
            assert_eq!(entry["outcome"]["answers"], json!([router[0]["answer_id"]]));
        } else {
            assert_eq!(native["confidence"], 0.93);
        }
        if entry["outcome"]["kind"] == "dropped" {
            assert_eq!(entry["outcome"]["at"]["kind"], "router");
            assert_eq!(entry["outcome"]["answer"], router[0]["answer_id"]);
        }
        if entry["outcome"]["kind"] != "finding" {
            assert!(
                !answers
                    .iter()
                    .any(|answer| answer["question"]["kind"] == "verdict"),
                "{entry}"
            );
        }
    }
    assert!(server.finish().await >= 6);
}

#[tokio::test]
async fn conflicting_target_mappings_refuse_before_http() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let input = router_fixture(root.path(), &server.endpoint);
    let path = root.path().join("router.json");
    let mut router: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    router["inputs"]["second_subject"] = json!({"kind":"conversation"});
    router["routes"]["right"] =
        json!([{"rule_set":"left.json","inputs":{"subject":"second_subject"}}]);
    std::fs::write(path, router.to_string()).unwrap();
    let output = support::run_without_credential(root.path(), "evaluate_candidates", input).await;
    assert_eq!(output.status.code(), Some(2));
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "invalid_wiring"),
        "{wire}"
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn router_uses_each_entries_alias_mapped_fact_and_preserves_unavailable_reason() {
    for available in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = router_fixture(root.path(), &server.endpoint);
        let schema = json!({"type":"object","properties":{"decision":{"type":"string"}},"required":["decision"]});
        std::fs::write(root.path().join("fact-schema.json"), schema.to_string()).unwrap();
        input["payload_schemas"] = json!([{"name":"urn:fact","path":"fact-schema.json"}]);
        for side in ["left", "right"] {
            let path = root.path().join(format!("{side}.json"));
            let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            set["inputs"]["rule_fact"] = json!({"kind":"payload","schema":"urn:fact"});
            std::fs::write(path, set.to_string()).unwrap();
            let path = root.path().join(format!("{side}.jsonl"));
            let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            rule["evidence"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":"fact","input":"rule_fact","select":{"kind":"whole_payload"}}));
            std::fs::write(path, rule.to_string()).unwrap();
        }
        let path = root.path().join("router.json");
        let mut router: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        router["inputs"]["router_fact"] = json!({"kind":"payload","schema":"urn:fact"});
        router["evidence"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"fact","input":"router_fact","select":{"kind":"whole_payload"}}));
        for side in ["left", "right"] {
            router["routes"][side][0]["inputs"]["rule_fact"] = json!("router_fact");
        }
        std::fs::write(path, router.to_string()).unwrap();
        let line = json!({"type":"assistant","message":{"id":"same","content":"MATCH paired"}})
            .to_string()
            + "\n";
        for session in ["first", "second"] {
            std::fs::write(root.path().join(format!("{session}.jsonl")), &line).unwrap();
        }
        input["sources"] = json!([
            {"source":{"kind":"session_pair","provider":"claude_code","session":"first"}},
            {"source":{"kind":"session_pair","provider":"claude_code","session":"second"},"evidence":{"router_fact":if available {json!({"kind":"bound","binding":{"kind":"payload","value":{"decision":"ENTRY_ALLOW"}}})} else {json!({"kind":"unavailable","reason":"withheld"})}}}
        ]);
        input["evidence"] = json!({"router_fact":if available {json!({"kind":"bound","binding":{"kind":"payload","value":{"decision":"RUN_DENY"}}})} else {json!({"kind":"unavailable","reason":"unknown"})}});
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let output = loop {
            tokio::select! {
                output=&mut run=>break output.unwrap(),
                request=server.next()=>{
                    assert!(available,"unavailable router fact must not call model");
                    let body=request.body.to_string();
                    assert_ne!(body.contains("ENTRY_ALLOW"),body.contains("RUN_DENY"),"{body}");
                    let label=if body.contains("ENTRY_ALLOW") {"left"} else {"right"};
                    let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)|(name.clone(),if question["type"]=="choice" {json!({"type":"choice","choice":label,"confidence":0.9,"probabilities":{"left":0.5,"right":0.4,"none":0.1}})} else {json!({"type":"noul","noul":0.9})})).collect::<serde_json::Map<_,_>>();
                    request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
                }
            }
        };
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 4);
        for entry in entries {
            assert_eq!(entry["evidence"][0]["input"], "router_fact");
            if available {
                let allowed = entry["evidence"][0]["binding"]["binding"]["value"]["decision"]
                    == "ENTRY_ALLOW";
                let left = entry["rule"]["rule_id"] == "candidate.left";
                assert_eq!(
                    entry["outcome"]["kind"],
                    if allowed == left {
                        "finding"
                    } else {
                        "dropped"
                    },
                    "{entry}"
                );
                assert!(entry["evidence"][0]["content_identity"].is_string());
            } else {
                assert_eq!(entry["outcome"]["kind"], "unresolved", "{entry}");
                assert_eq!(entry["outcome"]["at"]["kind"], "router");
                assert_eq!(entry["outcome"]["missing"]["evidence"], "router_fact");
                assert_eq!(
                    entry["outcome"]["missing"]["reason"],
                    entry["evidence"][0]["binding"]["reason"]
                );
            }
        }
        let count = server.finish().await;
        if available {
            assert!(count >= 4)
        } else {
            assert_eq!(count, 0)
        }
    }
}

#[tokio::test]
async fn private_grant_reaches_only_its_own_rules_router_and_verdict() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let input = router_fixture(root.path(), &server.endpoint);
    std::fs::write(root.path().join("first.jsonl"),json!({"type":"assistant","message":{"id":"private","content":[{"type":"text","text":"MATCH public"},{"type":"thinking","thinking":"PRIVATE_GRANTED_REASON"}]}}).to_string()+"\n").unwrap();
    let path = root.path().join("left.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["private_access"] = json!(["thinking"]);
    std::fs::write(path, rule.to_string()).unwrap();
    let path = root.path().join("router.json");
    let mut router: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let targets = json!([router["routes"]["left"][0], router["routes"]["right"][0]]);
    router["routes"] = json!({"both":targets});
    router["question"]["criteria"] = json!({"both":"both sets","none":"neither"});
    std::fs::write(path, router.to_string()).unwrap();
    let directory = root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let mut observed = std::collections::BTreeMap::new();
    let mut serial = 0;
    let output = loop {
        tokio::select! {
            output=&mut run=>break output.unwrap(),
            request=server.next()=>{
                let private=request.body.to_string().contains("PRIVATE_GRANTED_REASON");
                let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)| {
                    serial+=1;
                    let value=0.8+f64::from(serial)/1000.0;
                    let kind=if question["type"]=="choice" {"router"} else {"verdict"};
                    observed.insert(format!("{value:.3}"),(kind,private));
                    (name.clone(),if kind=="router" {json!({"type":"choice","choice":"both","confidence":value,"probabilities":{"both":0.9,"none":0.1}})} else {json!({"type":"noul","noul":value})})
                }).collect::<serde_json::Map<_,_>>();
                request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
            }
        }
    };
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = wire["result"]["outcome"]["report"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        let granted = entry["rule"]["rule_id"] == "candidate.left";
        assert_eq!(entry["outcome"]["kind"], "finding", "{entry}");
        assert_eq!(
            entry["privacy_view"]["grants"],
            if granted {
                json!(["thinking"])
            } else {
                json!([])
            }
        );
        for kind in ["router", "verdict"] {
            let answers = entry["answers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|answer| answer["question"]["kind"] == kind)
                .collect::<Vec<_>>();
            assert_eq!(answers.len(), 1);
            let value = answers[0]["answer"][if kind == "router" {
                "confidence"
            } else {
                "yes_probability"
            }]
            .as_f64()
            .unwrap();
            assert_eq!(
                observed.get(&format!("{value:.3}")),
                Some(&(kind, granted)),
                "each boundary must use its rule's grant: {entry}"
            );
        }
        if !granted {
            assert!(!entry.to_string().contains("PRIVATE_GRANTED_REASON"));
        }
    }
    assert_eq!(
        wire["result"]["outcome"]["report"]["privacy"]["granted"],
        json!([{"rule":{"rule_set":root.path().join("left.json").canonicalize().unwrap(),"rule_id":"candidate.left"},"private_kind":"thinking"}])
    );
    assert!(server.finish().await >= 2);
}
