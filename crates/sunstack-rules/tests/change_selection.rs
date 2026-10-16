#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/scope_fixture.rs"]
mod scope_fixture;
#[path = "support/decision_http.rs"]
mod transport;
use scope_fixture::{GATE_TEXT, ScopeFixture, fixture};
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;
fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
fn preparation(outcome: RulePreparationOutcome) -> Value {
    match outcome {
        RulePreparationOutcome::Prepared { preparation } => {
            serde_json::to_value(preparation).unwrap()
        }
        other => panic!("{other:?}"),
    }
}
#[tokio::test]
async fn mixed_changed_and_unchanged_paragraphs_select_the_right_owner_through_real_http() {
    let fixture = ScopeFixture::new();
    fixture
        .repository
        .write("source.md", "U001: changed literal\n\nU002: neighbor\n");
    let mut server = controlled::ControlledServer::start().await;
    let input = fixture.input(&server.endpoint);
    let prepared = preparation(prepare_rules(input.clone(), context(&server.endpoint)).await);
    assert_eq!(
        prepared["occurrences"][0]["status"],
        json!({"kind":"planned","selection":{"kind":"selected","selected_by":["target_changed"]}})
    );
    assert_eq!(
        prepared["occurrences"][1]["status"],
        json!({"kind":"outside_change_scope","reason":"no_declared_change"})
    );
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
    assert_eq!(
        request.body["questions"]["q000001"]["instructions"]["question"],
        "judge changed"
    );
    assert!(
        request.body["state"]["evidence"]
            .get("rule:neighbor/target")
            .is_none()
    );
    request.noul(0.9);
    let result = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = result else {
        panic!("{result:?}")
    };
    let report = serde_json::to_value(report).unwrap();
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["target_changed"])
    );
    assert_eq!(report["occurrences"][1]["kind"], "outside_change_scope");
    assert_eq!(server.finish().await, 1);
}
#[tokio::test]
async fn line_movement_alone_becomes_outside_before_any_encoding_or_http() {
    let fixture = ScopeFixture::new();
    fixture.repository.write(
        "source.md",
        "# Added prefix\n\nU001: original\n\nU002: neighbor\n",
    );
    let server = controlled::ControlledServer::start().await;
    let input = fixture.input(&server.endpoint);
    let prepared = preparation(prepare_rules(input.clone(), context(&server.endpoint)).await);
    for occurrence in prepared["occurrences"].as_array().unwrap() {
        assert_eq!(occurrence["status"]["kind"], "outside_change_scope");
    }
    assert_eq!(prepared["planned_requests"], json!([]));
    let result = evaluate_rules(input, context(&server.endpoint)).await;
    let RuleInvocationOutcome::CompleteReport { report } = result else {
        panic!("{result:?}")
    };
    assert!(
        report.occurrences.iter().all(|occurrence| matches!(
            occurrence,
            RuleOccurrenceResult::OutsideChangeScope { .. }
        ))
    );
    assert_eq!(report.requests.len(), 1);
    assert!(matches!(
        report.requests[0].status,
        RequestStatus::NotSent {
            reason: NotSentReason::OutsideChangeScope
        }
    ));
    assert_eq!(server.finish().await, 0);
}
#[tokio::test]
async fn definition_changes_select_only_their_owners_and_common_instructions_select_both() {
    for common in [false, true] {
        let fixture = ScopeFixture::new();
        if common {
            let path = fixture.repository.root.path().join("set.json");
            let mut set: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            set["common_ruleset_instructions"] = json!("changed common");
            fixture.repository.write("set.json", &set.to_string());
        } else {
            let mut rule = fixture::rule("changed");
            rule["verdict"]["question"]["instructions"] = json!("changed definition");
            fixture.repository.write("changed.jsonl", &rule.to_string());
        }
        let server = controlled::ControlledServer::start().await;
        let prepared = preparation(
            prepare_rules(fixture.input(&server.endpoint), context(&server.endpoint)).await,
        );
        assert_eq!(
            prepared["occurrences"][0]["status"]["selection"]["selected_by"],
            json!(["rule_changed"])
        );
        if common {
            assert_eq!(
                prepared["occurrences"][1]["status"]["selection"]["selected_by"],
                json!(["rule_changed"])
            );
        } else {
            assert_eq!(
                prepared["occurrences"][1]["status"]["kind"],
                "outside_change_scope"
            );
        }
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn staged_gate_preserves_pending_frontier_and_pass_exposes_equality_change_or_refusal() {
    for (probability, text, expected) in [
        (
            0.1,
            "# Prefix\n\nU001: original\n\nU002: neighbor\n",
            "not_applicable",
        ),
        (
            0.9,
            "# Prefix\n\nU001: original\n\nU002: neighbor\n",
            "outside_change_scope",
        ),
        (0.9, "U001: changed\n\nU002: neighbor\n", "judged"),
        (
            0.9,
            "U001: duplicate A\n\nU001: duplicate B\n\nU002: neighbor\n",
            "selection_failed",
        ),
    ] {
        let fixture = ScopeFixture::new();
        let mut server = controlled::ControlledServer::start().await;
        let input = fixture.add_gate(&server.endpoint);
        fixture.repository.write("source.md", text);
        let prepared = preparation(prepare_rules(input.clone(), context(&server.endpoint)).await);
        for occurrence in prepared["occurrences"].as_array().unwrap() {
            assert_eq!(
                occurrence["status"],
                json!({"kind":"planned","selection":{"kind":"comparison_pending"}})
            );
        }
        assert_eq!(prepared["measurements"]["bytes_scanned"], GATE_TEXT.len());
        assert!(
            prepared["planned_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|request| request["readiness"]["kind"] == "waiting")
        );
        let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
        let gate = server.next().await;
        assert_eq!(
            gate.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        gate.noul(probability);
        if expected == "judged" {
            let rule = server.next().await;
            assert_eq!(rule.body["questions"].as_object().unwrap().len(), 1);
            rule.noul(0.9);
        }
        let outcome = task.await.unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => report,
            other => panic!("{other:?}"),
        };
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(serialized["occurrences"][0]["kind"], expected);
        if probability < 0.6 {
            assert_eq!(report.measurements.bytes_scanned, GATE_TEXT.len());
        }
        if expected == "selection_failed" {
            assert!(report.diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==EvidenceCode::DuplicateTarget)));
        }
        assert_eq!(
            server.finish().await,
            if expected == "judged" { 2 } else { 1 }
        );
    }
}
#[tokio::test]
async fn speculative_real_comparison_refusal_is_subordinate_to_the_observed_gate_outcome() {
    for probability in [0.1, 0.9] {
        let fixture = ScopeFixture::new();
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture.add_gate(&server.endpoint);
        input.execution.speculation = Speculation::On;
        fixture.repository.write(
            "source.md",
            "U001: duplicate A\n\nU001: duplicate B\n\nU002: neighbor\n",
        );
        let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
        let gate = server.next().await;
        assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
        assert_eq!(
            gate.body["questions"]["q000001"]["instructions"]["owner"],
            "rule_set_decision:0"
        );
        gate.noul(probability);
        let outcome = task.await.unwrap();
        let report = match outcome {
            RuleInvocationOutcome::CompleteReport { report } if probability < 0.6 => report,
            RuleInvocationOutcome::IncompleteReport { report } if probability >= 0.6 => report,
            other => panic!("{other:?}"),
        };
        assert!(
            matches!(
                report.occurrences[0],
                RuleOccurrenceResult::NotApplicable { .. }
            ) == (probability < 0.6)
        );
        if probability >= 0.6 {
            assert!(matches!(
                report.occurrences[0],
                RuleOccurrenceResult::SelectionFailed { .. }
            ));
        }
        assert!(report.diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==EvidenceCode::DuplicateTarget)));
        assert!(report.measurements.files_read > 0);
        assert_eq!(report.measurements.model_requests, 1);
        assert_eq!(report.measurements.inference_attempts, 1);
        assert!(report.requests.iter().any(|request| matches!(
            request.status,
            RequestStatus::NotSent {
                reason: NotSentReason::EvidenceRefused { .. }
            }
        )));
        assert_eq!(server.finish().await, 1);
    }
}
#[tokio::test]
async fn cancellation_and_request_limit_leave_unreached_content_comparison_pending() {
    for cancel in [false, true] {
        let fixture = ScopeFixture::new();
        let mut server = controlled::ControlledServer::start().await;
        let mut input = fixture.add_gate(&server.endpoint);
        fixture.repository.write(
            "source.md",
            "# Prefix\n\nU001: original\n\nU002: neighbor\n",
        );
        if !cancel {
            input.request_budget.max_requests = 1;
        }
        let cancellation = CancellationToken::new();
        let mut execution_context = context(&server.endpoint);
        execution_context.cancellation = cancellation.clone();
        let task = tokio::spawn(evaluate_rules(input, execution_context));
        let gate = server.next().await;
        if cancel {
            cancellation.cancel();
            drop(gate);
        } else {
            gate.noul(0.9);
        }
        let outcome = task.await.unwrap();
        let report = match outcome {
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } if cancel => *partial_report,
            RuleInvocationOutcome::IncompleteReport { report } if !cancel => report,
            other => panic!("{other:?}"),
        };
        for occurrence in serde_json::to_value(&report.occurrences)
            .unwrap()
            .as_array()
            .unwrap()
        {
            assert_eq!(
                occurrence["selection"],
                json!({"kind":"comparison_pending"})
            );
            assert!(occurrence.get("selected_by").is_none());
        }
        assert_eq!(report.change_selections.len(), 1);
        assert_eq!(report.measurements.bytes_scanned, GATE_TEXT.len());
        assert_eq!(server.finish().await, 1);
    }
}
#[tokio::test]
async fn a_later_http_barrier_mutation_does_not_replace_the_binding_capture_used_at_content_frontier()
 {
    let fixture = ScopeFixture::new();
    let mut server = controlled::ControlledServer::start().await;
    let input = fixture.add_gate(&server.endpoint);
    let captured = "# Prefix\n\nU001: original\n\nU002: neighbor\n";
    fixture.repository.write("source.md", captured);
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let gate = server.next().await;
    fixture
        .repository
        .write("source.md", "U001: later live mutation\n\nU002: neighbor\n");
    gate.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(
        report.occurrences.iter().all(|occurrence| matches!(
            occurrence,
            RuleOccurrenceResult::OutsideChangeScope { .. }
        ))
    );
    let after = report.change_selections[0].members[0]
        .changed_paths
        .iter()
        .find(|change| change.path.to_str() == Some("source.md"))
        .unwrap()
        .after
        .as_ref()
        .unwrap();
    assert!(
        matches!(after,SourceRef::RepositoryFile{content_identity,..} if content_identity==&content_digest(captured.as_bytes()))
    );
    assert_eq!(server.finish().await, 1);
}
#[tokio::test]
async fn target_only_deletion_is_a_selection_failure_and_a_new_ruleset_selects_unchanged_reused_rule()
 {
    let fixture = ScopeFixture::new();
    fixture.repository.write("source.md", "U002: neighbor\n");
    let server = controlled::ControlledServer::start().await;
    let outcome = evaluate_rules(fixture.input(&server.endpoint), context(&server.endpoint)).await;
    let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(matches!(
        report.occurrences[0],
        RuleOccurrenceResult::SelectionFailed { .. }
    ));
    assert!(report.diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==EvidenceCode::DeletedTarget)));
    assert_eq!(server.finish().await, 0);
    let fixture = ScopeFixture::new();
    let mut server = controlled::ControlledServer::start().await;
    let set = json!({"schema_version":1,"name":"new owner","description":"existing unchanged rule","inputs":{"subject":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["changed.jsonl"]});
    fixture.repository.write("new.json", &set.to_string());
    let mut input = fixture.input(&server.endpoint);
    input.repositories[0].change_scope = ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged, DirtyLayer::Untracked],
    };
    let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
        panic!("fixture owner")
    };
    let mut new = rule_sets[0].clone();
    new.path = "new.json".into();
    rule_sets.push(new);
    let prepared = preparation(prepare_rules(input.clone(), context(&server.endpoint)).await);
    assert_eq!(
        prepared["occurrences"][2]["status"]["selection"]["selected_by"],
        json!(["new_rule"])
    );
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    server.next().await.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(matches!(
        report.occurrences[2],
        RuleOccurrenceResult::Judged { .. }
    ));
    assert_eq!(server.finish().await, 1);
}
