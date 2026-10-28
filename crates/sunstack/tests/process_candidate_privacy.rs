#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_code_fixture.rs"]
mod code_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use code_fixture::CodeFixture;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn code_and_outside_document_grants_are_isolated_through_router_and_bundle() {
    let mut server = ControlledServer::start().await;
    let fixture = CodeFixture::new(&server.endpoint, false);
    let root = fixture.repository.root.path();
    let mut input = fixture.input.clone();
    std::fs::create_dir(root.join("facts")).unwrap();
    std::fs::write(
        root.join("facts/outside.md"),
        "OUTSIDE_PUBLIC\napi_key = publicsyntheticoutside123456789\n",
    )
    .unwrap();
    input["repositories"].as_array_mut().unwrap().push(json!({"key":"facts","root":root.join("facts"),"input_paths":[{"kind":"exact","path":"outside.md"}],"change_scope":{"kind":"full"}}));
    input["evidence"] = json!({"outside_fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"outside.md"}}});
    let path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["fact"] = json!({"kind":"document"});
    std::fs::write(path, set.to_string()).unwrap();
    let path = root.join("rules.jsonl");
    let mut denied: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    denied["rule_id"] = json!("denied.code");
    denied["evidence"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"outside","input":"fact","select":{"kind":"whole_document"}}));
    let mut granted = denied.clone();
    granted["rule_id"] = json!("granted.code");
    granted["private_access"] = json!(["credential_like"]);
    std::fs::write(path, format!("{denied}\n{granted}\n")).unwrap();
    let mut router_evidence = denied["evidence"].clone();
    router_evidence[0]["input"] = json!("code_subject");
    router_evidence[1]["input"] = json!("outside_fact");
    std::fs::write(root.join("router.json"),json!({"schema_version":1,"name":"code privacy","description":"per rule code context","inputs":{"code_subject":{"kind":"change_set"},"outside_fact":{"kind":"document"}},"evidence":router_evidence,"question":{"type":"choice","instructions":"route this occurrence","criteria":{"selected":"rule set"}},"routes":{"selected":[{"rule_set":"set.json","inputs":{"subject":"code_subject","fact":"outside_fact"}}]},"confidence_floor":0.7}).to_string()).unwrap();
    input["definitions"] = json!({"kind":"router","path":"router.json"});
    let directory = root.to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let mut ledger = std::collections::BTreeMap::new();
    let mut serial = 0;
    let output = loop {
        tokio::select! {
            output=&mut run=>break output.unwrap(),
            request=server.next()=>{
                let body=request.body.to_string();let code=body.contains("sk-proj-publicfixturecredential123456789");let outside=body.contains("publicsyntheticoutside123456789");
                assert_eq!(code,outside,"the rule grant must apply to both declared routes");assert!(body.contains("OUTSIDE_PUBLIC"));
                let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)|{
                    serial+=1;let value=0.8+f64::from(serial)/10000.0;let router=question["type"]=="choice";
                    ledger.insert(format!("{value:.4}"),(router,code,outside));
                    (name.clone(),if router {json!({"type":"choice","choice":"selected","confidence":value,"probabilities":{"selected":1.0}})} else {json!({"type":"noul","noul":value})})
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
    let report = &wire["result"]["outcome"]["report"];
    let entries = report["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    for entry in entries {
        let granted = entry["rule"]["rule_id"] == "granted.code";
        assert_eq!(entry["outcome"]["kind"], "finding");
        assert_eq!(
            entry["privacy_view"]["grants"],
            if granted {
                json!(["credential_like"])
            } else {
                json!([])
            }
        );
        for router in [true, false] {
            let answers = entry["answers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|answer| {
                    answer["question"]["kind"] == if router { "router" } else { "verdict" }
                })
                .collect::<Vec<_>>();
            assert_eq!(answers.len(), 1);
            let value = answers[0]["answer"][if router {
                "confidence"
            } else {
                "yes_probability"
            }]
            .as_f64()
            .unwrap();
            assert_eq!(
                ledger.get(&format!("{value:.4}")),
                Some(&(router, granted, granted))
            );
        }
        assert_eq!(
            entry["context"]["context"]["redactions"]
                .as_array()
                .unwrap()
                .is_empty(),
            granted
        );
        if !granted {
            assert!(
                !entry
                    .to_string()
                    .contains("sk-proj-publicfixturecredential123456789")
            );
        }
    }
    let mut denied_scope = report.clone();
    denied_scope["candidates"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["rule"]["rule_id"] != "granted.code");
    denied_scope["dropped_samples"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["rule"]["rule_id"] != "granted.code");
    let denied_scope = denied_scope.to_string();
    assert!(!denied_scope.contains("sk-proj-publicfixturecredential123456789"));
    assert!(!denied_scope.contains("publicsyntheticoutside123456789"));
    for route in ["code", "outside_evidence"] {
        assert!(
            report["privacy"]["left_out"]
                .as_array()
                .unwrap()
                .iter()
                .any(|count| count["route"] == route
                    && count["private_kind"] == "credential_like"
                    && count["count"].as_u64().unwrap() > 0)
        );
    }
    assert_eq!(
        report["privacy"]["granted"],
        json!([{"rule":{"rule_set":root.join("set.json").canonicalize().unwrap(),"rule_id":"granted.code"},"private_kind":"credential_like"}])
    );
    assert!(server.finish().await >= 4);
}
