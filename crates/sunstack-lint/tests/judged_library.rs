#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled;
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use sunstack_decisions::{CancellationToken, JudgmentAnswer};
use sunstack_lint::*;

#[test]
fn reachable_models_require_a_key_and_reports_resolve_real_answers_evidence_and_optional_cost() {
    let Ok(case) = std::env::var("SUNSTACK_LINT_TEST_CASE") else {
        // Process-scoped environments avoid unsafe global environment mutation
        // and make this test independent of a developer's real credential.
        for case in ["missing-key", "with-cost", "without-cost"] {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command.args(["--exact","reachable_models_require_a_key_and_reports_resolve_real_answers_evidence_and_optional_cost"]).env("SUNSTACK_LINT_TEST_CASE",case).env_remove("OPENROUTER_API_KEY");
            if case != "missing-key" {
                command.env("OPENROUTER_API_KEY", "SYNTHETIC_LINT_CREDENTIAL");
            }
            assert!(command.status().unwrap().success(), "{case}");
        }
        return;
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async{
        let root=tempfile::tempdir().unwrap();let mut config=fixture::write(root.path());let mut server=controlled::ControlledServer::start().await;
        config["backend"]=json!({"kind":"jev_openrouter","model":"jev-latest","endpoint":server.endpoint});fixture::config_file(root.path(),&config);
        let path=root.path().join("rules.jsonl");let mut rule:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();rule["evidence"][0]["select"]=json!({"kind":"paragraph","target_id":"U001"});rule["questions"]=json!({"details":{"type":"noul","instructions":"Is the exact evidence useful?"}});rule["verdict"]=json!({"kind":"noul","question":{"type":"noul","instructions":"Does the paragraph comply?"},"threshold":0.8});std::fs::write(path,serde_json::to_vec(&rule).unwrap()).unwrap();std::fs::write(root.path().join("docs/one.md"),"U001: exact failing fixture evidence.\n").unwrap();
        if case=="missing-key"{
            let report=run(fixture::request(root.path()),CancellationToken::new()).await;
            assert_eq!(report.status,LintStatus::Invalid,"{report:?}");assert_eq!(report.diagnostics[0].code,LintInvalidCode::CredentialMissing);assert_eq!(report.usage.model_requests,0);assert_eq!(server.finish().await,0);return;
        }
        let mut operation=tokio::spawn(run(fixture::request(root.path()),CancellationToken::new()));
        let request=tokio::select!{request=server.next()=>request,result=&mut operation=>panic!("No HTTP request: {:?}",result.unwrap())};
        let sent=request.body.clone();
        let answers=sent["questions"].as_object().unwrap().keys().map(|name|(name.clone(),json!({"type":"noul","noul":0.2}))).collect::<serde_json::Map<_,_>>();
        let mut response=json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":10,"output_tokens":3}});if case=="with-cost"{response["usage"]["cost"]=json!(0.001);}
        request.respond(200,response);
        let report=tokio::time::timeout(std::time::Duration::from_secs(3),operation).await.unwrap().unwrap();
        assert_eq!(server.finish().await,1);assert_eq!(report.status,LintStatus::Findings,"{report:?}");assert_eq!(report.counts.error,1);assert_eq!(report.usage.model_requests,1);assert_eq!(report.usage.input_tokens,Some(10));assert_eq!(report.usage.output_tokens,Some(3));
        assert_eq!(report.usage.cost_usd,if case=="with-cost"{Some(0.001)}else{None});assert_eq!((report.usage.cost_reported_requests,report.usage.cost_missing_requests),if case=="with-cost"{(1,0)}else{(0,1)});
        let judgment=report.findings[0].judgment.as_ref().unwrap();assert_eq!(judgment.request_index,0);assert_eq!(judgment.answers.len(),2);assert!(judgment.answers.iter().all(|answer|matches!(answer.answer,JudgmentAnswer::Noul{yes_probability:0.2})));assert_eq!(serde_json::to_value(&report.evidence_seen[&judgment.request_index]).unwrap(),sent["state"]);
        assert!(!render(&report,OutputFormat::Json).contains("SYNTHETIC_LINT_CREDENTIAL"));
    });
}

async fn run(request: LintRequest, cancellation: CancellationToken) -> LintReport {
    sunstack_lint::run(
        request,
        cancellation,
        std::env::var("OPENROUTER_API_KEY").ok(),
    )
    .await
}
