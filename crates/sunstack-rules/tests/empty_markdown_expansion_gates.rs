#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/markdown_rule_fixture.rs"]
mod markdown_fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

async fn exercise_empty_expansion(selection_failed: bool) {
    for stop in ["failure", "nonpass", "cancel", "pass"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let request = markdown_fixture::input(root.path(), &server.endpoint);
        let mut wire = serde_json::to_value(request).unwrap();
        wire["definitions"]["rule_sets"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let path = root.path().join("gated/set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["applicability"][0]["evidence"][0] =
            json!({"id":"context","input":"subject","select":{"kind":"whole_document"}});
        std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
        let path = root.path().join("gated/rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if selection_failed {
            rule["evidence"][0]["max_items"] = json!(2);
        } else {
            rule["evidence"][0]["select"]["pattern"] = json!("NotPresent");
        }
        std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
        let cancellation = CancellationToken::new();
        let execution = tokio::spawn(evaluate_rules(
            serde_json::from_value(wire).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: cancellation.clone(),
            },
        ));
        let peer = server.next().await;
        assert_eq!(
            peer.body["questions"]["q000001"]["instructions"]["owner"],
            "rule:peer"
        );
        peer.noul(0.95);
        let gate = server.next().await;
        assert_eq!(
            gate.body["questions"].as_object().unwrap().len(),
            1,
            "empty/failed discovery must keep a real gate-only carrier"
        );
        match stop {
            "failure" => gate.respond(503, json!({"error":"controlled"})),
            "nonpass" => gate.noul(0.1),
            "cancel" => {
                cancellation.cancel();
                drop(gate);
            }
            _ => gate.noul(0.95),
        }
        let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.finish().await, 2);
        let expected_exit = match stop {
            "cancel" => 4,
            "failure" => 3,
            "pass" if selection_failed => 3,
            _ => 0,
        };
        assert_eq!(
            outcome.exit_code(),
            expected_exit,
            "{selection_failed}/{stop}: {outcome:?}"
        );
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } => *partial_report,
            other => panic!("{other:?}"),
        };
        assert_eq!(report.occurrences.len(), 1);
        assert!(matches!(
            report.occurrences[0],
            RuleOccurrenceResult::Judged { .. }
        ));
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
                "failure" => "evaluation_failed",
                "nonpass" => "not_applicable",
                "cancel" => "cancelled",
                "pass" if selection_failed => "selection_failed",
                _ => "zero_matches",
            }
        );
        assert_eq!(report.is_complete(), expected_exit == 0);
    }
}

#[tokio::test]
async fn zero_matches_do_not_hide_a_failed_or_stopped_carried_gate() {
    exercise_empty_expansion(false).await;
}

#[tokio::test]
async fn speculative_selection_failure_does_not_override_the_carried_gate_outcome() {
    exercise_empty_expansion(true).await;
}

#[tokio::test]
async fn preparation_reports_expansion_failure_even_without_child_occurrences() {
    let root = tempfile::tempdir().unwrap();
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let input = markdown_fixture::input(root.path(), endpoint);
    let path = root.path().join("gated/set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["applicability"] = json!([]);
    std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
    let path = root.path().join("gated/rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][0]["max_items"] = json!(2);
    std::fs::write(path, serde_json::to_vec(&rule).unwrap()).unwrap();
    let outcome = prepare_rules(
        input,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let RulePreparationOutcome::PreparedWithFailures {
        preparation,
        diagnostics,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(preparation.occurrences.len(), 1);
    assert_eq!(preparation.measurements.rules_failed, 1);
    assert!(!diagnostics.is_empty());
    assert!(preparation.rule_evaluations.iter().any(|evaluation|matches!(evaluation,PreparedRuleEvaluation::SelectionFailed {key,..} if key.rule_id.as_str()=="many")));
}
