#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn refused_owner_leaves_peer_questions_and_nonpassing_gate_dominates_its_selection_failure() {
    for gate in [None, Some(0.9), Some(0.1)] {
        let root = tempfile::tempdir().unwrap();
        let mut refused = fixture::rule("refused");
        refused["evidence"][0]["select"]["target_id"] = json!("U404");
        fixture::write_set(
            root.path(),
            &[refused, fixture::rule("peer")],
            gate.map(|_| vec![fixture::decision("subject")])
                .unwrap_or_default(),
        );
        let mut server = controlled::ControlledServer::start().await;
        let input = fixture::input(root.path(), &server.endpoint);
        let execution = evaluate_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let mut bodies = Vec::new();
        let outcome=tokio::time::timeout(Duration::from_secs(4),async {loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>{
            bodies.push(request.body.clone());let answers=request.body["questions"].as_object().unwrap().iter().map(|(name,question)|{let is_gate=question["instructions"]["owner"].as_str().unwrap().starts_with("rule_set_decision:");(name.clone(),json!({"type":"noul","noul":if is_gate{gate.unwrap()}else{0.9}}))}).collect::<serde_json::Map<_,_>>();request.respond(200,json!({"model":"typesafe/test","answers":answers,"usage":{"input_tokens":12,"output_tokens":3}}));
        }}}}).await.unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            _ => panic!("{outcome:?}"),
        };
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            bodies[0]["questions"].as_object().unwrap().len(),
            if gate.is_some() { 2 } else { 1 }
        );
        assert!(
            !serde_json::to_string(&bodies[0]["questions"])
                .unwrap()
                .contains("rule:refused")
        );
        if gate == Some(0.1) {
            assert!(
                report
                    .occurrences
                    .iter()
                    .all(|result| matches!(result, RuleOccurrenceResult::NotApplicable { .. }))
            );
            assert_eq!(report.requests[0].discarded.len(), 1);
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::SelectionFailed { .. }
            ));
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::Judged { .. }
            ));
        }
        assert_eq!(server.finish().await, 1);
    }
}
#[tokio::test]
async fn all_refused_owners_and_gate_required_refusal_record_evidence_refused_without_http() {
    for gate_refused in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut rule = fixture::rule("one");
        let mut gate = fixture::decision("subject");
        if gate_refused {
            gate["evidence"][0]["select"]["target_id"] = json!("U404");
        } else {
            rule["evidence"][0]["select"]["target_id"] = json!("U404");
        }
        fixture::write_set(
            root.path(),
            &[rule],
            if gate_refused { vec![gate] } else { vec![] },
        );
        let server = controlled::ControlledServer::start().await;
        let outcome = evaluate_rules(
            fixture::input(root.path(), &server.endpoint),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.model_requests, 0);
        assert!(matches!(
            report.requests[0].status,
            RequestStatus::NotSent {
                reason: NotSentReason::EvidenceRefused { .. }
            }
        ));
        if gate_refused {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::EvaluationFailed {
                    cause: Cause::Evidence { .. },
                    ..
                }
            ));
        } else {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::SelectionFailed { .. }
            ));
        }
        assert_eq!(server.finish().await, 0);
    }
}
