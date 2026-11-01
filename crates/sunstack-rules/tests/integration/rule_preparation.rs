#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[tokio::test]
async fn preparation_readies_only_pre_answer_requests_and_waiting_records_acquire_nothing() {
    for speculation in ["off", "on"] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![fixture::decision("other"), fixture::decision("later")],
        );
        std::fs::write(root.path().join("later.md"), "U001: later gate\n").unwrap();
        let path = root.path().join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["inputs"]["later"] = json!({"kind":"document"});
        std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
        let server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        input["execution"]["speculation"] = json!(speculation);
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["later"] =
            json!({"kind":"document","repository":"repo","path":"later.md"});
        input["repositories"][0]["input_paths"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"exact","path":"later.md"}));
        let outcome = prepare_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let wire = serde_json::to_value(&outcome).unwrap();
        assert_eq!(wire["kind"], "prepared");
        let preparation = &wire["preparation"];
        assert_eq!(preparation["measurements"]["model_requests"], 0);
        assert_eq!(
            preparation["measurements"]["files_read"],
            if speculation == "off" { 1 } else { 3 }
        );
        let requests = preparation["planned_requests"].as_array().unwrap();
        assert_eq!(requests.len(), 3);
        let prepared = requests
            .iter()
            .filter(|request| request["readiness"]["kind"] == "prepared")
            .count();
        assert_eq!(prepared, if speculation == "off" { 1 } else { 3 });
        if speculation == "off" {
            for request in requests
                .iter()
                .filter(|request| request["readiness"]["kind"] == "waiting")
            {
                assert!(request["readiness"].get("budget").is_none());
                assert!(request["readiness"].get("evidence").is_none());
                let on = request["readiness"]["on"].as_array().unwrap();
                assert_eq!(on.len(), 1);
                assert_eq!(on[0]["position"], 0);
            }
        }
        assert_eq!(server.finish().await, 0);
    }
}
#[tokio::test]
async fn preparation_assesses_carried_and_split_requests_without_dispatching_or_inventing_answers()
{
    for max_questions in [2, 1] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![fixture::decision("subject")],
        );
        let server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        input.request_budget.max_questions = max_questions;
        let outcome = prepare_rules(
            input,
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let wire = serde_json::to_value(&outcome).unwrap();
        assert_eq!(wire["kind"], "prepared");
        let preparation = &wire["preparation"];
        assert_eq!(preparation["measurements"]["files_read"], 1);
        assert_eq!(preparation["measurements"]["model_requests"], 0);
        let requests = preparation["planned_requests"].as_array().unwrap();
        assert_eq!(requests.len(), if max_questions == 1 { 2 } else { 1 });
        for request in requests {
            assert_eq!(request["readiness"]["kind"], "prepared");
            assert_eq!(
                request["readiness"]["budget"]["kind"],
                "within_configured_budget"
            );
            assert!(
                request["readiness"]["budget"]["counts"]["request_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert!(
                request["readiness"]["budget"]["counts"]["questions"]
                    .as_u64()
                    .unwrap()
                    <= max_questions as u64
            );
            assert!(
                !request["readiness"]["evidence"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(preparation["occurrences"][0]["status"]["kind"], "planned");
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn preparation_reports_evidence_budget_and_accounting_failures_without_dispatch() {
    for failure in ["selection", "budget", "unverifiable"] {
        let root = tempfile::tempdir().unwrap();
        let mut rule = fixture::rule("one");
        if failure == "selection" {
            rule["evidence"][0]["select"]["target_id"] = json!("U404");
        }
        fixture::write_set(root.path(), &[rule], vec![]);
        let server = controlled::ControlledServer::start().await;
        let mut input = fixture::input(root.path(), &server.endpoint);
        if failure == "budget" {
            input.request_budget.max_request_bytes = 1;
        }
        if failure == "unverifiable" {
            input.request_budget.accounting_policy =
                sunstack_decisions::AccountingPolicy::RequireExact;
        }
        let backend = transport::backend(&server.endpoint);
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            prepare_rules(
                input,
                RuleExecutionContext {
                    backend: Arc::new(backend),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: CancellationToken::new(),
                },
            ),
        )
        .await
        .unwrap();
        let wire = serde_json::to_value(&outcome).unwrap();
        assert_eq!(wire["kind"], "prepared_with_failures");
        assert_eq!(wire["preparation"]["measurements"]["model_requests"], 0);
        let records = wire["preparation"]["planned_requests"].as_array().unwrap();
        assert_eq!(records.len(), 1);
        let budget = &records[0]["readiness"]["budget"];
        assert_eq!(
            budget["kind"],
            if failure == "budget" {
                "exceeded"
            } else {
                "unverifiable"
            }
        );
        assert!(budget.get("counts").is_none());
        if failure == "selection" {
            assert_eq!(
                wire["preparation"]["occurrences"][0]["status"]["kind"],
                "selection_failed"
            );
            assert_eq!(
                records[0]["readiness"]["evidence"][0]["readiness"]["kind"],
                "refused"
            );
        }
        assert_eq!(server.finish().await, 0);
    }
}
#[tokio::test]
async fn routed_preparation_waits_for_scan_only_sources_and_respects_speculation_for_noncarrying_candidates()
 {
    for (scan_only, speculation) in [(false, "off"), (false, "on"), (true, "off"), (true, "on")] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            if scan_only {
                vec![
                    json!({"kind":"condition","test":{"kind":"pattern_present","input":"other","pattern":{"kind":"literal","text":"gate content"},"max_scan_bytes":4096}}),
                ]
            } else {
                vec![]
            },
        );
        let wiring = if scan_only {
            json!({"subject":"subject","other":"other"})
        } else {
            json!({"subject":"other","other":"subject"})
        };
        std::fs::write(root.path().join("router.json"),serde_json::to_vec(&json!({"schema_version":1,"name":"router","description":"preparation frontier","inputs":{"subject":{"kind":"document"},"other":{"kind":"document"}},"evidence":[fixture::declaration("subject","route_context")],"question":{"type":"choice","instructions":"route","criteria":{"yes":"selected","no":"unrouted"}},"routes":{"yes":[{"rule_set":"set.json","inputs":wiring}]},"confidence_floor":0.6})).unwrap()).unwrap();
        let server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"] =
            json!({"kind":"router","path":"router.json","binding_groups":[group]});
        input["execution"]["speculation"] = json!(speculation);
        let outcome = prepare_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let wire = serde_json::to_value(&outcome).unwrap();
        assert_eq!(wire["kind"], "prepared");
        let concurrent = !scan_only && speculation == "on";
        assert_eq!(
            wire["preparation"]["measurements"]["files_read"],
            if concurrent { 2 } else { 1 }
        );
        let requests = wire["preparation"]["planned_requests"].as_array().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request["readiness"]["kind"] == "waiting")
                .count(),
            usize::from(!concurrent)
        );
        for request in requests
            .iter()
            .filter(|request| request["readiness"]["kind"] == "waiting")
        {
            assert_eq!(request["readiness"]["on"][0]["owner"]["kind"], "router");
            assert!(request["readiness"].get("evidence").is_none());
        }
        assert_eq!(server.finish().await, 0);
    }
}
#[tokio::test]
async fn preparation_pre_abort_returns_no_preparation_and_starts_no_acquisition() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    let server = controlled::ControlledServer::start().await;
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let outcome = prepare_rules(
        fixture::input(root.path(), &server.endpoint),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation,
        },
    )
    .await;
    assert!(matches!(
        outcome,
        RulePreparationOutcome::Cancelled {
            progress: PreparationCancellationProgress::BeforePreparationAdmission,
            measurements: InvocationMeasurements {
                files_read: 0,
                model_requests: 0,
                ..
            },
            ..
        }
    ));
    assert_eq!(server.finish().await, 0);
}
