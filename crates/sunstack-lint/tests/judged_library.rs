#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled;
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::json;
use sunstack_decisions::{CancellationToken, JudgmentAnswer};
use sunstack_lint::*;
#[tokio::test]
async fn reachable_models_require_a_key_and_reports_resolve_real_answers_evidence_and_optional_cost()
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
            assert_eq!(report.status, LintStatus::Invalid);
            assert_eq!(
                report.diagnostics[0].code,
                LintInvalidCode::CredentialMissing
            );
            assert_eq!(report.usage.checked_files_acquired, 0);
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
