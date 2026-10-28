#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/eval_process_fixture.rs"]
mod eval_fixture;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use eval_fixture::{EvalFixture, LABEL_SENTINEL, read_json, response, write_json};
use serde_json::json;
use sunstack_evals::{CaseId, Label};
use sunstack_rules::{CandidateDefinitions, QuestionRef};
use transport::{AnswerServer, ReplyMode, RouterReply};

#[tokio::test]
async fn frozen_router_entry_preserves_its_native_drop_answer_and_changes_the_binary_score() {
    for routed in [true, false] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::NoRoute,
        })
        .await;
        let mut fixture = EvalFixture::new(&server.endpoint);
        let root = fixture.root.path();
        let set = read_json(&root.join("set.json"));
        let mut other_set = set.clone();
        other_set["name"] = json!("other");
        other_set["rule_paths"] = json!(["b.rules.jsonl"]);
        write_json(&root.join("b.json"), &other_set);
        let mut other_rule = read_json(&root.join("rules.jsonl"));
        other_rule["rule_id"] = json!("rule.b");
        write_json(&root.join("b.rules.jsonl"), &other_rule);
        let mut declaration = other_rule["evidence"][0].clone();
        declaration["id"] = json!("route_context");
        let router = json!({"schema_version":1,"name":"frozen-route","description":"Route A or B","inputs":set["inputs"],"evidence":[declaration],"question":{"type":"choice","instructions":"FROZEN_ROUTER_CHOOSE_B","criteria":{"review_handback":"A","other":"B"}},"routes":{"review_handback":[{"rule_set":"set.json","inputs":{"subject":"subject","fact":"fact"}}],"other":[{"rule_set":"b.json","inputs":{"subject":"subject","fact":"fact"}}]},"confidence_floor":0.6});
        write_json(&root.join("router.json"), &router);
        fixture.specification.definition_files.extend([
            root.join("b.json"),
            root.join("b.rules.jsonl"),
            root.join("router.json"),
        ]);
        fixture.specification.entry = if routed {
            CandidateDefinitions::Router {
                path: "router.json".into(),
            }
        } else {
            CandidateDefinitions::RuleSets {
                rule_sets: vec!["set.json".into()],
            }
        };
        fixture.specification.labels[0].label = Label::No;
        let mut router_label = fixture.specification.labels[0].clone();
        router_label.case_id = CaseId::new("PRIVATE_ROUTER_LABEL_ONLY").unwrap();
        router_label.question = QuestionRef::Router {};
        fixture.specification.labels.push(router_label);
        fixture.freeze_as(if routed { "routed-case" } else { "direct-case" });
        let frozen_manifest = read_json(&fixture.manifest);
        assert_eq!(
            frozen_manifest["entry"],
            if routed {
                json!({"kind":"router","path":"router.json"})
            } else {
                json!({"kind":"rule_sets","rule_sets":["set.json"]})
            }
        );
        // A separate existing operation exposes the native bundle join, without changing score_eval's output.
        let input = fixture.candidate_input();
        let output = support::run(fixture.root.path(), "evaluate_candidates", input).await;
        assert_eq!(output.status.code(), Some(0));
        let wire: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        let frozen = sunstack_evals::load_case_set(&fixture.manifest).unwrap();
        let expected_rule = serde_json::to_value(frozen.replay_rule()).unwrap();
        let selected = entries
            .iter()
            .filter(|candidate| candidate["rule"] == expected_rule)
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1);
        let candidate = selected[0];
        assert_eq!(candidate["candidate_id"]["rule"], expected_rule);
        let mut expected_occurrence =
            serde_json::to_value(&fixture.specification.labels[0].source_anchor).unwrap();
        // The label identifies the record; the candidate retains the fixture's MATCH attribution.
        expected_occurrence["field"] = json!("/body/blocks/0/block/text");
        expected_occurrence["span"] = json!({"byte_start":0,"byte_end":5});
        assert_eq!(candidate["occurrence"], expected_occurrence);
        assert_eq!(
            candidate["candidate_id"]["occurrence"],
            fixture.specification.labels[0]
                .source_anchor
                .occurrence_identity()
                .unwrap()
                .as_str()
        );
        if routed {
            assert_eq!(candidate["outcome"]["kind"], "dropped");
            assert_eq!(candidate["outcome"]["at"], json!({"kind":"router"}));
            assert_eq!(candidate["answers"].as_array().unwrap().len(), 1);
            let answer = &candidate["answers"][0];
            assert_eq!(answer["question"], json!({"kind":"router"}));
            assert_eq!(answer["settlement"], "did_not_pass");
            assert_eq!(answer["answer"]["selected_label"], "other");
            assert_eq!(candidate["outcome"]["answer"], answer["answer_id"]);
        } else {
            assert_eq!(candidate["outcome"]["kind"], "finding");
            assert!(
                candidate["answers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|answer| answer["question"]["kind"] != "router")
            );
        }
        let mut live_router = router;
        live_router["question"]["instructions"] = json!("LIVE_ROUTER_NEVER_USED");
        live_router["routes"]["other"][0]["rule_set"] = json!("set.json");
        write_json(&fixture.root.path().join("router.json"), &live_router);
        let before = server.count();
        let output = support::run(fixture.root.path(), "score_eval", fixture.input).await;
        let wire = response(&output, 0);
        let report = &wire["result"]["outcome"]["report"];
        let expected = json!([LABEL_SENTINEL]);
        if routed {
            assert_eq!(report["end_result"]["correct_negatives"], expected);
            assert_eq!(report["end_result"]["false_findings"], json!([]));
            let steps = report["steps"].as_array().unwrap();
            let router_step = steps
                .iter()
                .find(|step| step["question"]["kind"] == "router")
                .unwrap();
            assert_eq!(router_step["correct"], json!(["PRIVATE_ROUTER_LABEL_ONLY"]));
            assert_eq!(router_step["wrong"], json!([]));
            assert_eq!(router_step["unknown"], json!([]));
        } else {
            assert_eq!(report["end_result"]["false_findings"], expected);
            assert_eq!(report["end_result"]["correct_negatives"], json!([]));
        }
        let (_, bodies) = server.finish().await;
        let scored = &bodies[before..];
        assert!(!scored.is_empty());
        let choices = scored
            .iter()
            .flat_map(|body| body["questions"].as_object().unwrap().values())
            .filter(|question| question["type"] == "choice")
            .count();
        assert_eq!(choices > 0, routed);
        for body in scored {
            let text = body.to_string();
            assert!(!text.contains(LABEL_SENTINEL));
            assert!(!text.contains("PRIVATE_ROUTER_LABEL_ONLY"));
            assert!(!text.contains("LIVE_ROUTER_NEVER_USED"));
        }
    }
}
