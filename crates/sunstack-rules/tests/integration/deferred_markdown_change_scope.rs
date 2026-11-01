#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_rules::test_support::change_fixture as changes;
use sunstack_rules::test_support::controlled_http as controlled;
#[path = "../support/markdown_rule_fixture.rs"]
mod markdown_fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;

#[tokio::test]
async fn changed_scope_keeps_prior_snapshot_but_defers_target_selection_behind_gate() {
    for stop in ["pass", "nonpass", "failure", "cancel"] {
        let repository = changes::ChangeFixture::new();
        let root = repository.root.path();
        let mut server = controlled::ControlledServer::start().await;
        let mut input = markdown_fixture::input(root, &server.endpoint);
        for group in ["gated", "peer"] {
            for file in ["set.json", "rules.jsonl", "source.md", "other.md"] {
                repository.stage(&format!("{group}/{file}"));
            }
        }
        repository.commit();
        let snapshot = "- Use alpha; Use beta; Use gamma. SNAPSHOT\n";
        repository.write("gated/source.md", snapshot);
        let peer_rule = std::fs::read_to_string(root.join("peer/rules.jsonl")).unwrap();
        repository.write("peer/rules.jsonl", &format!("{peer_rule}\n"));
        input.repositories[0] = repository.binding(
            "repo",
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        );
        input.discovery = changes::discovery();
        // capture_pair reads both sides of every one of these eight paths
        // before comparing them, including unchanged paths: exactly 16 reads.
        input.limits.max_files = 16.try_into().unwrap();
        let preparation = prepare_rules(
            input.clone(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: AdapterRegistry::builtins().unwrap(),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let RulePreparationOutcome::Prepared { preparation } = preparation else {
            panic!("{preparation:?}")
        };
        let prepared = serde_json::to_value(&preparation).unwrap();
        assert_eq!(
            preparation.measurements.files_parsed, 2,
            "only real peer and gate selectors before dispatch"
        );
        let waiting = prepared["rule_evaluations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|evaluation| evaluation["key"]["rule_id"] == "many")
            .unwrap();
        assert_eq!(waiting["kind"], "waiting_for_expansion");
        assert_eq!(preparation.occurrences.len(), 1);
        let cancellation = CancellationToken::new();
        let execution = tokio::spawn(evaluate_rules(
            input,
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
        assert_eq!(gate.body["questions"].as_object().unwrap().len(), 1);
        assert!(
            !serde_json::to_string(&gate.body["state"])
                .unwrap()
                .contains("SNAPSHOT")
        );
        repository.write(
            "gated/source.md",
            "- Use alpha; Use beta; Use gamma. LIVE_AFTER_GATE\n",
        );
        match stop {
            "pass" => {
                gate.noul(0.95);
                let children = server.next().await;
                let state = serde_json::to_string(&children.body["state"]).unwrap();
                assert!(state.contains("SNAPSHOT"));
                assert!(!state.contains("LIVE_AFTER_GATE"));
                assert_eq!(children.body["questions"].as_object().unwrap().len(), 3);
                children.noul(0.95);
            }
            "nonpass" => gate.noul(0.1),
            "failure" => gate.respond(503, serde_json::json!({"error":"controlled"})),
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
            other => panic!("{other:?}"),
        };
        // Eight Index and eight Worktree files are expected prior snapshot
        // reads. All canonical selectors reuse those current captures, adding
        // no filesystem reads after the staged gate is released.
        assert_eq!(report.measurements.files_read, 16);
        assert_eq!(sent, if stop == "pass" { 3 } else { 2 });
        // Path comparison freezes bytes without selecting them. Only pass may
        // perform two Content-side selector parses plus canonical selection.
        // Eager target selection on a stopped path adds a third parse and fails.
        assert_eq!(
            report.measurements.files_parsed,
            if stop == "pass" { 5 } else { 2 },
            "{stop}"
        );
        assert_eq!(
            report.measurements.bytes_scanned,
            if stop == "pass" {
                "U001: original content\n".len()
                    + "U001: gate content\n".len()
                    + "- Use alpha; Use beta; Use gamma. INITIAL\n".len()
                    + 2 * snapshot.len()
            } else {
                "U001: original content\n".len() + "U001: gate content\n".len()
            }
        );
        assert!(report.occurrences.iter().any(|occurrence|matches!(occurrence,RuleOccurrenceResult::Judged {key,..} if key.rule_id.as_str()=="peer")));
        let children = report
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.key().rule_id.as_str() == "many")
            .collect::<Vec<_>>();
        assert_eq!(children.len(), if stop == "pass" { 3 } else { 0 });
        for child in children {
            assert!(matches!(
                child.key().target.as_ref(),
                RuleOccurrenceTarget::CallSite {
                    source: SourceRef::RepositoryFile {
                        revision: SourceRevision::Worktree { .. },
                        ..
                    },
                    ..
                }
            ));
        }
        let wire: Value = serde_json::to_value(report).unwrap();
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
    }
}
