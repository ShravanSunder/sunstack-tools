#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_decisions::{CancellationToken, JudgmentAnswer};
use sunstack_lint::test_support::lint_fixture as fixture;
use sunstack_lint::*;
#[path = "judged_library/case_binding_failures.rs"]
mod case_binding_failures;
#[path = "judged_library/case_byte_binding.rs"]
mod case_byte_binding;
#[path = "judged_library/case_dispatch.rs"]
mod case_dispatch;
#[path = "judged_library/case_prerequisites.rs"]
mod case_prerequisites;
#[path = "judged_library/filters.rs"]
mod filters;
use sunstack_rules::test_support::controlled_http as controlled;
#[tokio::test]
async fn reached_models_are_unavailable_keyless_and_keyed_reports_keep_real_answers_evidence_and_cost()
 {
    for case in ["missing-key", "with-cost", "without-cost"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        fixture::judged(root.path(), &server.endpoint);
        if case == "missing-key" {
            let report = run(
                fixture::request(root.path()),
                CancellationToken::new(),
                None,
            )
            .await;
            assert_eq!(report.status, LintStatus::Incomplete);
            assert_eq!(report.status.exit_code(), 3);
            assert!(report.diagnostics.is_empty());
            assert_eq!(report.usage.checked_files_acquired, 1);
            assert_eq!(report.usage.model_requests, 0);
            assert!(report.requests.is_empty());
            assert!(report.findings.is_empty());
            assert_eq!(report.results.len(), 1);
            let result = &report.results[0];
            assert!(matches!(
                result.ending,
                RulePieceEnding::Unavailable {
                    reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {}
                }
            ));
            assert_eq!(result.name.as_str(), "description");
            assert_eq!(result.file, std::path::Path::new("docs/one.md"));
            assert_eq!((result.first_line, result.last_line), (1, 3));
            assert!(result.text_sha256.is_some());
            assert!(report.piece_evidence.contains_key(&result.id));
            assert!(result.request_ids.is_empty());
            assert_eq!(result.step.as_ref().unwrap().as_str(), "complies");
            assert!(result.answers.is_empty());
            assert!(result.basis.is_none());
            assert_eq!(report.unavailable_counts().rule_pieces, 1);
            assert_eq!(server.finish().await, 0);
            continue;
        }
        let mut operation = tokio::spawn(run(
            fixture::request(root.path()),
            CancellationToken::new(),
            Some("SYNTHETIC_LINT_CREDENTIAL".into()),
        ));
        let request = tokio::select! {request=server.next()=>request,result=&mut operation=>panic!("No HTTP request: {:?}",result.unwrap())};
        let sent = request.body.clone();
        let answers = sent["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(|name| (name.clone(), json!({"type":"noul","noul":0.2})))
            .collect::<serde_json::Map<_, _>>();
        let mut response = json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":10,"output_tokens":3}});
        if case == "with-cost" {
            response["usage"]["cost"] = json!(0.001);
        }
        request.respond(200, response);
        let report = tokio::time::timeout(std::time::Duration::from_secs(3), operation)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.finish().await, 1);
        assert_eq!(report.status, LintStatus::Findings, "{report:?}");
        assert_eq!(report.counts.error, 1);
        assert_eq!(report.usage.model_requests, 1);
        assert_eq!(report.usage.input_tokens, Some(10));
        assert_eq!(report.usage.output_tokens, Some(3));
        assert_eq!(
            report.usage.cost_usd,
            if case == "with-cost" {
                Some(0.001)
            } else {
                None
            }
        );
        assert_eq!(
            (
                report.usage.cost_reported_requests,
                report.usage.cost_missing_requests
            ),
            if case == "with-cost" { (1, 0) } else { (0, 1) }
        );
        let judgment = report.findings[0].judgment.as_ref().unwrap();
        assert_eq!(judgment.answers.len(), 2);
        assert!(judgment.answers.values().all(|answer| matches!(
            answer,
            JudgmentAnswer::Noul {
                yes_probability: 0.2
            }
        )));
        assert_eq!(
            serde_json::to_value(&report.evidence_seen[&0]).unwrap(),
            sent["state"]
        );
        assert!(!render(&report, OutputFormat::Json).contains("SYNTHETIC_LINT_CREDENTIAL"));
    }
}
