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
async fn literal_regex_and_ast_conditions_scan_real_sources_and_keep_a_bounded_unresolved_outcome()
{
    for pattern in [
        json!({"kind":"literal","text":"answer = 42"}),
        json!({"kind":"regex","pattern":"answer\\s*=\\s*42"}),
        json!({"kind":"ast","language":"rust","pattern":"let $NAME = 42;"}),
    ] {
        for case in ["present", "absent", "bounded"] {
            let root = tempfile::tempdir().unwrap();
            fixture::write_set(
                root.path(),
                &[fixture::rule("one")],
                vec![
                    json!({"kind":"condition","test":{"kind":"pattern_present","input":"other","pattern":pattern,"max_scan_bytes":if case == "bounded" {2} else {4096}}}),
                ],
            );
            std::fs::write(
                root.path().join("other.md"),
                if case == "absent" {
                    "fn main() { let answer = 1; }\n"
                } else {
                    "fn main() { let answer = 42; }\n"
                },
            )
            .unwrap();
            let mut server = controlled::ControlledServer::start().await;
            let execution = evaluate_rules(
                fixture::input(root.path(), &server.endpoint),
                RuleExecutionContext {
                    backend: Arc::new(transport::backend(&server.endpoint)),
                    adapters: AdapterRegistry::builtins().unwrap(),
                    cancellation: CancellationToken::new(),
                },
            );
            tokio::pin!(execution);
            let outcome = tokio::time::timeout(Duration::from_secs(3), async {loop {tokio::select! { result = &mut execution => break result, request = server.next() => request.noul(0.9) }}}).await.unwrap();
            let report = match outcome {
                RuleInvocationOutcome::CompleteReport { report } if case != "bounded" => report,
                RuleInvocationOutcome::IncompleteReport { report } if case == "bounded" => report,
                other => panic!("{pattern}/{case}: {other:?}"),
            };
            match case {
                "present" => assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::Judged { .. }
                )),
                "absent" => assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::NotApplicable { .. }
                )),
                _ => assert!(matches!(
                    report.groups[0].applicability,
                    ApplicabilityOutcome::ApplicabilityFailed {
                        cause: Cause::Condition { .. },
                        ..
                    }
                )),
            }
            assert_eq!(
                report.measurements.files_read,
                if case == "present" { 2 } else { 1 }
            );
            assert_eq!(server.finish().await, usize::from(case == "present"));
        }
    }
}
