#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::*;
use serde_json::json;
use sunstack_evidence::{AdapterRegistry, EvidenceSession};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn a_missing_local_settlement_is_unresolved_and_incomplete_while_its_completed_peer_survives()
{
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::write(root.join("set.json"),serde_json::to_vec(&json!({"schema_version":1,"name":"local","description":"local projection","inputs":{"subject":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]})).unwrap()).unwrap();
    std::fs::write(root.join("rules.jsonl"),serde_json::to_vec(&json!({"rule_id":"local.one","evidence":[{"id":"target","input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"mechanical","explanation":"Use a prefix","check":{"kind":"expect","expect":{"starts_with":"Use"}}}})).unwrap()).unwrap();
    std::fs::write(root.join("source.md"), "Use first.\n\nUse second.\n").unwrap();
    let input:RuleInvocationInput=serde_json::from_value(json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"}}}]}]},"repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"origin":{"kind":"direct"},"cursors":[],"execution":{"speculation":"off","max_concurrent_requests":1},"limits":{"max_files":8,"max_file_bytes":65536,"max_total_bytes":131072,"max_matches":20,"max_transcript_bytes":65536,"include_ignored":false},"backend":{"kind":"jev_openrouter","endpoint":"http://127.0.0.1:9","model":"jev-latest"},"attempts":sunstack_decisions::AttemptSettings::default(),"request_budget":{"max_input_tokens":100000,"max_request_bytes":100000,"max_questions":1,"max_requests":1,"safety_margin_tokens":32,"accounting_policy":"allow_estimated"}})).unwrap();
    let registry = AdapterRegistry::builtins().unwrap();
    let cancellation = CancellationToken::new();
    let admitted = crate::rule_definitions::invocation_admission::admit_invocation(
        &input,
        &registry,
        &cancellation,
    )
    .await
    .unwrap();
    let mut session = EvidenceSession::open(admitted.evidence, registry, cancellation.clone())
        .await
        .unwrap();
    let mut plan = definition_topology(admitted.groups, admitted.router);
    plan_change_selection(&mut plan, &mut session).await;
    precompute_conditions(
        &mut plan,
        &input.origin,
        &mut session,
        &cancellation,
        input.execution.speculation,
    )
    .await;
    build_request_graph(&mut plan, &input.execution);
    let mut state = ExecutionState::new(&plan);
    advance_entries(
        &plan,
        &mut state,
        &input.origin,
        &mut session,
        &cancellation,
    )
    .await;
    discover_ready_rules(&mut plan, &mut state, &mut session, &cancellation).await;
    let request = plan.requests[0].clone();
    assert!(
        prepare_units(&plan, &mut state, &request, &mut session, &cancellation)
            .await
            .is_empty()
    );
    finish_empty_carriers(&plan, &mut state);
    assert_eq!(state.local_settlements.len(), 2);
    assert!(
        project_report(
            &plan,
            &state,
            &input.origin,
            InvocationMeasurements::default()
        )
        .is_complete()
    );

    // Simulate a lost local slot after real capture/discovery/preparation. The
    // report must not invent a passed check from a completed empty carrier.
    state.local_settlements.remove(&OccurrenceIndex(0));
    let report = project_report(
        &plan,
        &state,
        &input.origin,
        InvocationMeasurements::default(),
    );
    assert!(!report.is_complete());
    assert!(matches!(
        &report.occurrences[0],
        RuleOccurrenceResult::Checked {
            outcome: LocalSettlement::Unresolved { .. },
            ..
        }
    ));
    assert!(matches!(
        &report.occurrences[1],
        RuleOccurrenceResult::Checked {
            outcome: LocalSettlement::Completed {
                result: MechanicalResult::Pass,
                ..
            },
            ..
        }
    ));
    session.close();
}
