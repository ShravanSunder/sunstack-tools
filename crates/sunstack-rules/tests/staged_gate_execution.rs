#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn staged_gates_wait_in_authored_order_skip_later_acquisition_and_keep_first_captured_bytes()
{
    for stop in ["pass", "nonpass", "failure", "cancel"] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![
                fixture::decision("subject"),
                fixture::decision("other"),
                fixture::decision("later"),
            ],
        );
        let mut set: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
        set["inputs"]["later"] = json!({"kind":"document"});
        std::fs::write(
            root.path().join("set.json"),
            serde_json::to_vec(&set).unwrap(),
        )
        .unwrap();
        std::fs::write(root.path().join("later.md"), "U001: last gate\n").unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let base = fixture::input(root.path(), &server.endpoint);
        let mut input = serde_json::to_value(base).unwrap();
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["later"] =
            json!({"kind":"document","repository":"repo","path":"later.md"});
        input["repositories"][0]["input_paths"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"exact","path":"later.md"}));
        input["attempts"]["retry_count"] = json!(0);
        let cancellation = CancellationToken::new();
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: cancellation.clone(),
            },
        ));
        let first = server.next().await;
        assert_eq!(first.body["questions"].as_object().unwrap().len(), 1);
        assert_eq!(
            first.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        assert!(
            serde_json::to_string(&first.body["state"])
                .unwrap()
                .contains("original content")
        );
        // The first HTTP body proves capture and selection completed. Deleting the live source
        // must not change its later rule evidence or trigger another filesystem dependency.
        std::fs::remove_file(root.path().join("source.md")).unwrap();
        first.noul(0.9);
        let second = server.next().await;
        assert_eq!(
            second.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:1"
        );
        match stop {
            "nonpass" => second.noul(0.1),
            "failure" => second.respond(503, json!({"error":"controlled rejection"})),
            "cancel" => {
                cancellation.cancel();
                drop(second);
            }
            _ => {
                second.noul(0.9);
                let third = server.next().await;
                assert_eq!(
                    third.body["questions"]["q000001"]["instructions"]["owner"],
                    "rule_set_decision:2"
                );
                third.noul(0.9);
                let rule = server.next().await;
                assert_eq!(
                    rule.body["questions"]["q000001"]["instructions"]["owner"],
                    "rule:one"
                );
                assert!(
                    serde_json::to_string(&rule.body["state"])
                        .unwrap()
                        .contains("original content")
                );
                rule.noul(0.9);
            }
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report } if stop == "pass" => report,
            RuleInvocationOutcome::CompleteReport { report } if stop == "nonpass" => report,
            RuleInvocationOutcome::IncompleteReport { report } if stop == "failure" => report,
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } if stop == "cancel" => *partial_report,
            _ => panic!("{stop}: {outcome:?}"),
        };
        assert_eq!(
            report.measurements.files_read,
            if stop == "pass" { 3 } else { 2 }
        );
        assert_eq!(
            report.measurements.model_requests,
            if stop == "pass" { 4 } else { 2 }
        );
        if stop == "pass" {
            let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[0] else {
                panic!("{report:?}")
            };
            assert!(
                serde_json::to_string(&judgment.target)
                    .unwrap()
                    .contains("U001")
            );
        }
        if stop == "nonpass" {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable {
                    stopped_at: EntryKey { position: 1, .. },
                    ..
                }
            ));
        }
        if stop != "pass" {
            assert!(
                report
                    .requests
                    .iter()
                    .any(|request| matches!(request.status, RequestStatus::NotSent { .. }))
            );
        }
        let EntryResult::Decision {
            outcome: DecisionEntryOutcome::Passed { answer },
            ..
        } = &report.groups[0].entries[0].result
        else {
            panic!("{report:?}")
        };
        assert!(matches!(
            report.answer(answer),
            Some(sunstack_decisions::JudgmentAnswer::Noul { .. })
        ));
        assert!(
            matches!(&report.requests[answer.request_index].status, RequestStatus::Sent { outcome: sunstack_decisions::DecisionRunOutcome::Completed { receipt, attempts, .. } } if receipt.usage.cost_usd == Some(0.001) && attempts.len() == 1)
        );
        assert_eq!(server.finish().await, if stop == "pass" { 4 } else { 2 });
    }
}
