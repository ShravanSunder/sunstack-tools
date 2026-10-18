#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/markdown_rule_fixture.rs"]
mod markdown_fixture;
use markdown_fixture::input;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

#[tokio::test]
async fn held_different_input_gate_defers_capture_then_installs_three_owners_and_keeps_completed_peer()
 {
    for stop in ["pass", "nonpass", "failure", "cancel"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let request = input(root.path(), &server.endpoint);
        let cancellation = CancellationToken::new();
        let execution = tokio::spawn(evaluate_rules(
            request,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: cancellation.clone(),
            },
        ));
        // With one request slot the held gate can reach HTTP only after this
        // peer's real response is consumed and settled by the scheduler.
        let peer = server.next().await;
        assert_eq!(
            peer.body["questions"]["q000001"]["instructions"]["owner"],
            "rule:peer"
        );
        peer.noul(0.95);
        let gate = server.next().await;
        assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
        assert_eq!(
            gate.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        let held_state = serde_json::to_string(&gate.body["state"]).unwrap();
        assert!(!held_state.contains("INITIAL"));
        // A wrong eager read freezes INITIAL in EvidenceSession. Later reads
        // cannot repair that witness: UPDATED plus one counted acquisition is
        // required after release, and no target acquisition on other outcomes.
        let updated = "- Use alpha; Use beta; Use gamma. UPDATED\n";
        std::fs::write(root.path().join("gated/source.md"), updated).unwrap();
        match stop {
            "pass" => {
                gate.noul(0.95);
                let children = server.next().await;
                let state = serde_json::to_string(&children.body["state"]).unwrap();
                assert!(state.contains("UPDATED"));
                assert!(!state.contains("INITIAL"));
                let questions = children.body["questions"].as_object().unwrap();
                assert_eq!(questions.len(), 3);
                assert!(
                    !serde_json::to_string(questions)
                        .unwrap()
                        .contains(root.path().canonicalize().unwrap().to_str().unwrap())
                );
                let owners = questions
                    .values()
                    .map(|question| {
                        question["instructions"]["owner"]
                            .as_str()
                            .unwrap()
                            .to_owned()
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(owners.len(), 3);
                assert!(owners.iter().all(|owner| owner.starts_with("rule:many:")));
                children.noul(0.95);
            }
            "nonpass" => gate.noul(0.1),
            "failure" => gate.respond(503, json!({"error":"controlled failure"})),
            "cancel" => {
                cancellation.cancel();
                drop(gate);
            }
            _ => unreachable!(),
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let sent = server.finish().await;
        assert_eq!(
            outcome.exit_code(),
            match stop {
                "pass" | "nonpass" => 0,
                "cancel" => 4,
                _ => 3,
            }
        );
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } => *partial_report,
            other => panic!("{stop}: {other:?}"),
        };
        assert_eq!(sent, if stop == "pass" { 3 } else { 2 });
        assert_eq!(
            report.measurements.files_read,
            if stop == "pass" { 3 } else { 2 }
        );
        let peer = report
            .occurrences
            .iter()
            .find(|occurrence| occurrence.key().rule_id.as_str() == "peer")
            .unwrap();
        assert!(
            matches!(peer, RuleOccurrenceResult::Judged { .. }),
            "{stop}: {peer:?}"
        );
        let wire = serde_json::to_value(&report).unwrap();
        let evaluation = wire["rule_evaluations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|evaluation| evaluation["key"]["rule_id"] == "many")
            .unwrap();
        assert_eq!(
            evaluation["kind"],
            match stop {
                "pass" => "expanded",
                "nonpass" => "not_applicable",
                "cancel" => "cancelled",
                _ => "evaluation_failed",
            }
        );
        if stop == "pass" {
            let children = report
                .occurrences
                .iter()
                .filter(|occurrence| occurrence.key().rule_id.as_str() == "many")
                .collect::<Vec<_>>();
            assert_eq!(children.len(), 3);
            let keys = children
                .iter()
                .map(|occurrence| occurrence.key())
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(keys.len(), 3);
            for child in children {
                assert!(matches!(child, RuleOccurrenceResult::Judged { .. }));
            }
            assert_eq!(evaluation["occurrences"].as_array().unwrap().len(), 3);
            let mut missing = report.clone();
            missing
                .occurrences
                .retain(|occurrence| occurrence.key().rule_id.as_str() == "peer");
            assert!(
                !missing.is_complete(),
                "expanded children cannot disappear into a complete report"
            );
        } else {
            assert_eq!(report.occurrences.len(), 1);
        }
    }
}

#[tokio::test]
async fn zero_target_discovery_keeps_real_gate_work_and_reports_named_complete_result() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let request = input(root.path(), &server.endpoint);
    let path = root.path().join("gated/rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["select"]["pattern"] = json!("NotPresent");
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    let execution = tokio::spawn(evaluate_rules(
        request,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    server.next().await.noul(0.95);
    let gate = server.next().await;
    assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
    gate.noul(0.95);
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 2);
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(report.is_complete());
    assert_eq!(report.occurrences.len(), 1);
    assert_eq!(report.measurements.files_read, 3);
    let wire = serde_json::to_value(report).unwrap();
    let evaluation = wire["rule_evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|evaluation| evaluation["key"]["rule_id"] == "many")
        .unwrap();
    assert_eq!(evaluation["kind"], "zero_matches");
    assert_eq!(evaluation["input"], "subject");
    assert_eq!(evaluation["source"]["kind"], "document");
    assert!(
        wire["requests"]
            .as_array()
            .unwrap()
            .iter()
            .all(|request| request["status"]["kind"] != "sent"
                || !request["questions"].as_array().unwrap().is_empty())
    );
}

fn respond_with_target_readback(request: controlled::PendingHttpRequest) {
    let answers = request.body["questions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, question)| {
            let owner = question["instructions"]["owner"].as_str().unwrap();
            let probability = if owner == "rule:peer" || owner.starts_with("rule_set_decision:") {
                0.95
            } else {
                let target_label = question["instructions"]["reads"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap())
                    .find(|label| label.ends_with("/target"))
                    .unwrap();
                let content_id = request.body["state"]["evidence"][target_label]["content"]
                    .as_str()
                    .unwrap();
                let content =
                    serde_json::to_string(&request.body["state"]["contents"][content_id]).unwrap();
                if content.contains("PASS_SENTINEL") {
                    assert!(!content.contains("FAIL_SENTINEL"));
                    0.95
                } else {
                    assert!(content.contains("FAIL_SENTINEL"));
                    0.05
                }
            };
            (name.clone(), json!({"type":"noul","noul":probability}))
        })
        .collect::<serde_json::Map<_, _>>();
    request.respond(200,json!({"model":"typesafe/readback","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
}

#[tokio::test]
async fn opposite_target_answers_keep_evidence_owner_reference_and_verdict_across_budget_split() {
    let root = tempfile::tempdir().unwrap();
    let base = input(root.path(), "http://127.0.0.1:9/v1/systemone");
    let text = "Use PASS_SENTINEL.\n\nUse FAIL_SENTINEL.\n";
    std::fs::write(root.path().join("gated/source.md"), text).unwrap();
    let path = root.path().join("gated/rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["select"]["units"] = json!(["paragraph"]);
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    let mut baseline = None;
    for split in [false, true] {
        let mut server = controlled::ControlledServer::start().await;
        let mut request = base.clone();
        request.backend = transport::selection(&server.endpoint);
        if split {
            request.request_budget.max_questions = 1;
        }
        let mut execution = tokio::spawn(evaluate_rules(
            request,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let outcome=tokio::time::timeout(Duration::from_secs(3),async {
            loop { tokio::select! { result=&mut execution=>break result.unwrap(), request=server.receive()=>respond_with_target_readback(request) } }
        }).await.unwrap();
        let sent = server.finish().await;
        assert_eq!(sent, if split { 4 } else { 3 });
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        let mut semantic = Vec::new();
        for result in report
            .occurrences
            .iter()
            .filter(|result| result.key().rule_id.as_str() == "many")
        {
            let RuleOccurrenceResult::Judged { key, judgment, .. } = result else {
                panic!("{result:?}")
            };
            let sunstack_evidence::TargetLocation::MarkdownUnit { occurrence } = &judgment.target
            else {
                panic!("expected unit target")
            };
            let sunstack_evidence::MarkdownOccurrence::CallSite { unit_span, .. } =
                occurrence.as_ref()
            else {
                panic!("expected call site")
            };
            let expected =
                if text[unit_span.byte_start..unit_span.byte_end].contains("PASS_SENTINEL") {
                    0.95
                } else {
                    0.05
                };
            assert_eq!(judgment.verdict.value, expected);
            assert_eq!(
                matches!(judgment.verdict.result, VerdictResult::Pass),
                expected > 0.8
            );
            let VerdictBasis::Noul { answer } = &judgment.verdict.basis else {
                panic!("expected noul basis")
            };
            assert!(
                matches!(report.answer(answer),Some(sunstack_decisions::JudgmentAnswer::Noul {yes_probability}) if *yes_probability==expected)
            );
            let owner = &report.requests[answer.request_index]
                .questions
                .iter()
                .find(|question| question.request_name == answer.question_name)
                .unwrap()
                .owner;
            assert!(matches!(owner,QuestionOwner::Verdict {key:owner} if owner==key));
            semantic.push((key.clone(), judgment.target.clone(), judgment.verdict.value));
        }
        assert_eq!(semantic.len(), 2);
        if let Some(baseline) = &baseline {
            assert_eq!(&semantic, baseline);
        } else {
            baseline = Some(semantic);
        }
    }
}

#[tokio::test]
async fn same_input_speculation_and_split_keep_one_gate_and_deduplicate_content_only() {
    for (split, gate_probability) in [(false, 0.95), (false, 0.1), (true, 0.95), (true, 0.1)] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let mut request = input(root.path(), &server.endpoint);
        let path = root.path().join("gated/set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["applicability"][0]["evidence"][0] =
            json!({"id":"context","input":"subject","select":{"kind":"whole_document"}});
        std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
        if split {
            request.request_budget.max_questions = 2;
        }
        let mut execution = tokio::spawn(evaluate_rules(
            request,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        ));
        let mut gates = 0;
        let mut saw_three_attributions = false;
        let outcome=tokio::time::timeout(Duration::from_secs(3),async {
            loop {tokio::select! {
                result=&mut execution=>break result.unwrap(),
                pending=server.receive()=>{
                    let targets=pending.body["state"]["evidence"].as_object().unwrap().iter().filter(|(label,_)|label.starts_with("rule:many:")&&label.ends_with("/target")).collect::<Vec<_>>();
                    if targets.len()>1 {let ids=targets.iter().map(|(_,value)|value["content"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();assert_eq!(ids.len(),1);}
                    saw_three_attributions|=targets.len()==3;
                    let answers=pending.body["questions"].as_object().unwrap().iter().map(|(name,question)|{
                        let probability=if question["instructions"]["owner"].as_str().unwrap().starts_with("rule_set_decision:"){gates+=1;gate_probability}else{0.95};
                        (name.clone(),json!({"type":"noul","noul":probability}))
                    }).collect::<serde_json::Map<_,_>>();
                    pending.respond(200,json!({"model":"typesafe/speculative","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
                }
            }}
        }).await.unwrap();
        let sent = server.finish().await;
        assert_eq!(gates, 1);
        assert_eq!(
            sent,
            if split && gate_probability > 0.8 {
                3
            } else {
                2
            }
        );
        if !split {
            assert!(saw_three_attributions);
        }
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.files_read, 2);
        let children = report
            .occurrences
            .iter()
            .filter(|result| result.key().rule_id.as_str() == "many")
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 3);
        if gate_probability > 0.8 {
            assert!(
                children
                    .iter()
                    .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
            );
        } else {
            assert!(
                children
                    .iter()
                    .all(|result| matches!(result, RuleOccurrenceResult::NotApplicable { .. }))
            );
            assert_eq!(
                report
                    .requests
                    .iter()
                    .map(|request| request.discarded.len())
                    .sum::<usize>(),
                if split { 1 } else { 3 }
            );
        }
    }
}

#[tokio::test]
async fn one_multi_scalar_entity_keeps_two_keys_with_shared_raw_span_through_settlement() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let request = input(root.path(), &server.endpoint);
    std::fs::write(root.path().join("gated/source.md"), "&NotEqualTilde;\n").unwrap();
    let path = root.path().join("gated/rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["select"]["units"] = json!(["paragraph"]);
    rule["evidence"][0]["select"]["pattern"] = json!(".");
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    let execution = tokio::spawn(evaluate_rules(
        request,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    ));
    server.next().await.noul(0.95);
    server.next().await.noul(0.95);
    let children = server.next().await;
    assert_eq!(children.body["questions"].as_object().unwrap().len(), 2);
    let answers = children.body["questions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, question)| {
            let label = question["instructions"]["reads"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .find(|label| label.ends_with("/target"))
                .unwrap();
            let target = &children.body["state"]["evidence"][label]["units"][0]["occurrence"];
            assert_eq!(target["match_span"]["byte_start"], 0);
            assert_eq!(target["match_span"]["byte_end"], 15);
            let probability = if target["visible_range"]["start"] == 0 {
                0.95
            } else {
                0.05
            };
            (name.clone(), json!({"type":"noul","noul":probability}))
        })
        .collect::<serde_json::Map<_, _>>();
    children.respond(200,json!({"model":"typesafe/entity","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 3);
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let targets = report
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.key().rule_id.as_str() == "many")
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2);
    assert_eq!(
        targets
            .iter()
            .map(|occurrence| occurrence.key())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    let mut ranges = Vec::new();
    for occurrence in targets {
        let RuleOccurrenceResult::Judged { key, judgment, .. } = occurrence else {
            panic!("{occurrence:?}")
        };
        let RuleOccurrenceTarget::CallSite {
            match_range,
            visible_range,
            ..
        } = key.target.as_ref()
        else {
            panic!("expected call-site key")
        };
        assert_eq!((match_range.start, match_range.end), (0, 15));
        assert_eq!(
            judgment.verdict.value,
            if visible_range.start == 0 { 0.95 } else { 0.05 }
        );
        ranges.push((visible_range.start, visible_range.end));
    }
    assert_eq!(ranges, vec![(0, 3), (3, 5)]);
}

#[tokio::test]
async fn staged_prepare_reports_one_waiting_evaluation_without_target_reads_or_child_keys() {
    let root = tempfile::tempdir().unwrap();
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let outcome = prepare_rules(
        input(root.path(), endpoint),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let RulePreparationOutcome::Prepared { preparation } = outcome else {
        panic!("{outcome:?}")
    };
    let wire = serde_json::to_value(preparation).unwrap();
    assert_eq!(wire["measurements"]["files_read"], 2);
    let evaluations = wire["rule_evaluations"].as_array().unwrap();
    let waiting = evaluations
        .iter()
        .find(|evaluation| evaluation["key"]["rule_id"] == "many")
        .unwrap();
    assert_eq!(waiting["kind"], "waiting_for_expansion");
    assert_eq!(waiting["on"].as_array().unwrap().len(), 1);
    assert_eq!(waiting["on"][0]["position"], 0);
    assert_eq!(wire["occurrences"].as_array().unwrap().len(), 1);
    assert_eq!(wire["occurrences"][0]["key"]["rule_id"], "peer");
    let questions = wire["planned_requests"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|request| request["questions"].as_array().unwrap())
        .collect::<Vec<&Value>>();
    assert_eq!(questions.len(), 2);
    assert_eq!(
        questions
            .iter()
            .filter(|question| question["owner"]["kind"] == "rule_set_decision")
            .count(),
        1
    );
}
