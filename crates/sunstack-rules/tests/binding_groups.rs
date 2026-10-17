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
async fn three_groups_keep_threshold_answers_and_identical_bindings_as_separate_occurrences() {
    for identical in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![fixture::decision("subject")],
        );
        for position in 0..3 {
            std::fs::write(
                root.path().join(format!("group{position}.md")),
                format!("U001: GROUP_{position}\n"),
            )
            .unwrap();
        }
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        let base = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"]["rule_sets"][0]["binding_groups"] = json!(
            (0..if identical { 2 } else { 3 })
                .map(|position| {
                    let mut group = base.clone();
                    group["bindings"]["subject"]["path"] =
                        json!(format!("group{}.md", if identical { 0 } else { position }));
                    group
                })
                .collect::<Vec<_>>()
        );
        input["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"*.md"}]);
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>{
            let contents=serde_json::to_string(&request.body["state"]).unwrap();let probability=if identical{0.9}else if contents.contains("GROUP_0"){0.5999}else if contents.contains("GROUP_1"){0.6}else{0.6001};request.respond(200,json!({"model":"typesafe/groups","answers":{"q000001":{"type":"noul","noul":probability},"q000002":{"type":"noul","noul":0.9}},"usage":{"input_tokens":20,"output_tokens":4}}));
        }}}}).await.unwrap();
        let RuleInvocationOutcome::CompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        let expected = if identical { 2 } else { 3 };
        assert_eq!(report.groups.len(), expected);
        assert_eq!(report.occurrences.len(), expected);
        assert_eq!(report.requests.len(), expected);
        assert_eq!(
            report.measurements.files_read,
            if identical { 1 } else { 3 }
        );
        for (position, occurrence) in report.occurrences.iter().enumerate() {
            assert_eq!(occurrence.key().group, position);
            if !identical && position == 0 {
                assert!(matches!(
                    occurrence,
                    RuleOccurrenceResult::NotApplicable { .. }
                ));
            } else {
                assert!(matches!(occurrence, RuleOccurrenceResult::Judged { .. }));
            }
        }
        assert_eq!(server.finish().await, expected);
    }
}
#[tokio::test]
async fn file_and_byte_limits_are_cumulative_across_binding_groups() {
    for bound in ["files", "bytes"] {
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[fixture::rule("one")],
            vec![fixture::decision("subject")],
        );
        let text = "U001: SAME SIZE\n";
        for position in 0..3 {
            std::fs::write(root.path().join(format!("group{position}.md")), text).unwrap();
        }
        let mut server = controlled::ControlledServer::start().await;
        let mut input =
            serde_json::to_value(fixture::input(root.path(), &server.endpoint)).unwrap();
        let base = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"]["rule_sets"][0]["binding_groups"] = json!(
            (0..3)
                .map(|position| {
                    let mut group = base.clone();
                    group["bindings"]["subject"]["path"] = json!(format!("group{position}.md"));
                    group
                })
                .collect::<Vec<_>>()
        );
        input["repositories"][0]["input_paths"] = json!([{"kind":"glob","pattern":"*.md"}]);
        if bound == "files" {
            input["limits"]["max_files"] = json!(2);
        } else {
            input["limits"]["max_total_bytes"] = json!(text.len() * 2);
        }
        let execution = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        );
        tokio::pin!(execution);
        let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>request.noul(0.9)}}}).await.unwrap();
        let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
            panic!("{outcome:?}")
        };
        assert_eq!(report.measurements.files_read, 2);
        assert_eq!(report.measurements.model_requests, 2);
        assert!(
            report.occurrences[..2]
                .iter()
                .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
        );
        assert!(matches!(
            report.occurrences[2],
            RuleOccurrenceResult::EvaluationFailed {
                cause: Cause::Evidence { .. },
                ..
            }
        ));
        assert!(
            serde_json::to_string(&report.diagnostics)
                .unwrap()
                .contains("evidence_limit")
        );
        assert_eq!(server.finish().await, 2);
    }
}
