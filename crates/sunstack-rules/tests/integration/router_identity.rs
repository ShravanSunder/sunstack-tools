#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
fn conversation() -> Value {
    json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"excluded"},"hook_injected":"excluded"})
}
fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn answer(request: controlled::PendingHttpRequest) {
    let answers = request.body["questions"].as_object().unwrap().iter().map(|(name, question)| {
        let value = if question["type"] == "choice" {
            json!({"type":"choice","choice":"alpha","confidence":0.9,"probabilities":{"alpha":0.9,"none":0.1}})
        } else { json!({"type":"noul","noul":0.9}) };
        (name.clone(), value)
    }).collect::<serde_json::Map<_,_>>();
    request.respond(200, json!({"model":"typesafe/identity","answers":answers,"usage":{"input_tokens":30,"output_tokens":5,"cost":0.002}}));
}
#[tokio::test]
async fn carrying_compares_canonical_payload_values_and_conversation_versions_instead_of_input_names()
 {
    for kind in ["payload", "conversation"] {
        for same in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let mut rule = fixture::rule("one");
            let select = if kind == "payload" {
                json!({"kind":"whole_payload"})
            } else {
                conversation()
            };
            rule["evidence"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":"context","input":"context","select":select}));
            fixture::write_set(root.path(), &[rule], vec![]);
            let declaration = if kind == "payload" {
                json!({"kind":"payload","schema":"flags"})
            } else {
                json!({"kind":"conversation"})
            };
            let mut set: Value =
                serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap())
                    .unwrap();
            set["inputs"] = json!({"subject":{"kind":"document"},"context":declaration});
            write_json(&root.path().join("set.json"), &set);
            write_json(
                &root.path().join("schema.json"),
                &json!({"type":"object","additionalProperties":true}),
            );
            std::fs::write(root.path().join("session.jsonl"), b"{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"TRANSCRIPT_BODY\"}}\n").unwrap();
            let left = if kind == "payload" {
                json!({"kind":"payload","value":{"first":1,"second":2}})
            } else {
                json!({"kind":"conversation","transcript_path":root.path().join("session.jsonl"),"format":"claude_code_jsonl","final_assistant_text":"SUPPLEMENT_A"})
            };
            let right = if kind == "payload" {
                // Parse opposite authored key order, so equality is value based.
                serde_json::from_str::<Value>(if same {
                    r#"{"kind":"payload","value":{"second":2,"first":1}}"#
                } else {
                    r#"{"kind":"payload","value":{"first":9,"second":2}}"#
                })
                .unwrap()
            } else {
                json!({"kind":"conversation","transcript_path":root.path().join("./session.jsonl"),"format":"claude_code_jsonl","final_assistant_text":if same {"SUPPLEMENT_A"} else {"SUPPLEMENT_B"}})
            };
            write_json(
                &root.path().join("router.json"),
                &json!({"schema_version":1,"name":"identity","description":"canonical source identity","inputs":{"subject":{"kind":"document"},"left":declaration,"right":declaration},"evidence":[{"id":"context","input":"left","select":select}],"question":{"type":"choice","instructions":"route","criteria":{"alpha":"selected","none":"no route"}},"routes":{"alpha":[{"rule_set":"set.json","inputs":{"subject":"subject","context":"right"}}]},"confidence_floor":0.6}),
            );
            let mut server = controlled::ControlledServer::start().await;
            let mut input =
                serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
            input["definitions"] = json!({"kind":"router","path":"router.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"},"left":left,"right":right}}]});
            input["payload_schemas"] = if kind == "payload" {
                json!([{"name":"flags","path":"schema.json"}])
            } else {
                json!([])
            };
            let mut execution = tokio::spawn(evaluate_rules(
                serde_json::from_value(input).unwrap(),
                RuleExecutionContext {
                    backend: Arc::new(transport::backend(&server.endpoint)),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: CancellationToken::new(),
                },
            ));
            let first = tokio::select! {
                result = &mut execution => panic!("{kind} same={same}: invocation ended before HTTP: {result:?}"),
                request = server.next() => request,
            };
            assert_eq!(
                first.body["questions"].as_object().unwrap().len(),
                if same { 2 } else { 1 },
                "{kind} same={same}"
            );
            answer(first);
            if !same {
                let next = tokio::select! {
                    result = &mut execution => panic!("{kind} same={same}: invocation ended before rule HTTP: {result:?}"),
                    request = server.next() => request,
                };
                assert_eq!(next.body["questions"].as_object().unwrap().len(), 1);
                if kind == "conversation" {
                    let text = serde_json::to_string(&next.body["state"]).unwrap();
                    assert!(text.contains("SUPPLEMENT_B"));
                    assert!(!text.contains("SUPPLEMENT_A"));
                }
                answer(next);
            }
            let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
                .await
                .unwrap()
                .unwrap();
            let RuleInvocationOutcome::CompleteReport { report } = outcome else {
                panic!("{kind} same={same}: {outcome:?}")
            };
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::Judged { .. }
            ));
            assert_eq!(report.measurements.model_requests, if same { 1 } else { 2 });
            let RoutingReport::Router { groups } = &report.routing else {
                panic!("{report:?}")
            };
            let RouterOutcome::Routed { answer, .. } = &groups[0].outcome else {
                panic!("{groups:?}")
            };
            assert!(matches!(
                report.answer(answer),
                Some(sunstack_decisions::JudgmentAnswer::Choice { .. })
            ));
            assert_eq!(server.finish().await, report.measurements.model_requests);
        }
    }
}

#[tokio::test]
async fn two_rulesets_named_subject_wire_document_and_conversation_separately_and_refuse_cross_kind()
 {
    for invalid in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(root.path(), &[fixture::rule("document")], vec![]);
        std::fs::write(root.path().join("source.md"), "U001: DOC_ONLY\n").unwrap();
        std::fs::write(root.path().join("session.jsonl"), b"{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"CHAT_ONLY\"}}\n").unwrap();
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
        document["inputs"] = json!({"subject":{"kind":"document"}});
        write_json(&root.path().join("set.json"), &document);
        let mut chat = document.clone();
        chat["inputs"]["subject"] = json!({"kind":"conversation"});
        chat["rule_paths"] = json!(["chat.jsonl"]);
        write_json(&root.path().join("chat.json"), &chat);
        let mut rule = fixture::rule("conversation");
        rule["evidence"][0]["select"] = conversation();
        write_json(&root.path().join("chat.jsonl"), &rule);
        write_json(
            &root.path().join("router.json"),
            &json!({"schema_version":1,"name":"mixed","description":"wiring kinds","inputs":{"document":{"kind":"document"},"chat":{"kind":"conversation"}},"evidence":[{"id":"context","input":"document","select":{"kind":"whole_document"}}],"question":{"type":"choice","instructions":"select","criteria":{"alpha":"route","none":"unrouted"}},"routes":{"alpha":[{"rule_set":"set.json","inputs":{"subject":"document"}},{"rule_set":"chat.json","inputs":{"subject":if invalid {"document"} else {"chat"}}}]},"confidence_floor":0.6}),
        );
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        input["definitions"] = json!({"kind":"router","path":"router.json","binding_groups":[{"bindings":{"document":{"kind":"document","repository":"repo","path":"source.md"},"chat":{"kind":"conversation","transcript_path":root.path().join("session.jsonl"),"format":"claude_code_jsonl"}}}]});
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        if invalid {
            let outcome = execution.await;
            let RuleInvocationOutcome::Invalid { diagnostics } = outcome else {
                panic!("{outcome:?}")
            };
            assert!(
                serde_json::to_string(&diagnostics)
                    .unwrap()
                    .contains("invalid_wiring")
            );
            assert_eq!(server.finish().await, 0);
            continue;
        }
        let execution = tokio::spawn(execution);
        let first = server.next().await;
        let content = serde_json::to_string(&first.body["state"]).unwrap();
        assert!(content.contains("DOC_ONLY"));
        assert!(!content.contains("CHAT_ONLY"));
        assert_eq!(first.body["questions"].as_object().unwrap().len(), 2);
        answer(first);
        let second = server.next().await;
        let content = serde_json::to_string(&second.body["state"]).unwrap();
        assert!(content.contains("CHAT_ONLY"));
        assert!(!content.contains("DOC_ONLY"));
        answer(second);
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.occurrences.len(), 2);
        assert!(
            report
                .occurrences
                .iter()
                .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
        );
        assert_eq!(report.measurements.model_requests, 2);
        assert_eq!(server.finish().await, 2);
    }
}
