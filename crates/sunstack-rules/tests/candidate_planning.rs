#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "support/controlled_http.rs"]
mod controlled_http;
use controlled_http::ControlledServer;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_rules::*;

#[tokio::test]
async fn two_rules_on_one_record_have_separate_requests_and_rule_local_thinking_grants() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
    input["sources"].as_array_mut().unwrap().truncate(1);
    let rules_path = root.path().join("rules.jsonl");
    let mut granted: Value = serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    granted["rule_id"] = json!("granted.rule");
    granted["private_access"] = json!(["thinking"]);
    granted["verdict"]["question"]["instructions"] = json!("GRANTED_RULE judge this occurrence");
    let mut ordinary = granted.clone();
    ordinary["rule_id"] = json!("ordinary.rule");
    ordinary["private_access"] = json!([]);
    ordinary["verdict"]["question"]["instructions"] = json!("ORDINARY_RULE judge this occurrence");
    std::fs::write(&rules_path, format!("{granted}\n{ordinary}\n")).unwrap();
    std::fs::write(root.path().join("first.jsonl"),format!("{}\n",json!({"type":"assistant","message":{"id":"one","content":[{"type":"text","text":"MATCH PUBLIC_TEXT"},{"type":"thinking","thinking":"PRIVATE_THINKING_SENTINEL"}]}}))).unwrap();
    let typed = decode_candidate_run(&serde_json::to_vec(&input).unwrap()).unwrap();
    let context = RuleExecutionContext {
        backend: Arc::new(BackendWithCredential {
            selection: typed.backend.clone(),
        }),
        adapters: sunstack_evidence::AdapterRegistry::builtins().unwrap(),
        cancellation: tokio_util::sync::CancellationToken::new(),
    };
    let run = tokio::spawn(evaluate_candidates(typed, context));
    for _ in 0..2 {
        let request = server.next().await;
        let rendered = request.body.to_string();
        if rendered.contains("GRANTED_RULE") {
            assert!(!rendered.contains("ORDINARY_RULE"));
            assert!(rendered.contains("PRIVATE_THINKING_SENTINEL"));
            request.noul(0.9);
        } else {
            assert!(rendered.contains("ORDINARY_RULE"));
            assert!(!rendered.contains("PRIVATE_THINKING_SENTINEL"));
            request.noul(0.1);
        }
    }
    let result = run.await.unwrap();
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas/evaluate-candidates-outcome.schema.json");
    let schema: Value = serde_json::from_slice(&std::fs::read(schema_path).unwrap()).unwrap();
    assert!(
        jsonschema::options()
            .offline()
            .build(&schema)
            .unwrap()
            .is_valid(&serde_json::to_value(&result).unwrap())
    );
    let CandidateRunOutcome::CandidateReport { report } = result else {
        panic!("{result:?}")
    };
    assert_eq!(report.candidates.len(), 2);
    assert_eq!(report.usage.requests, 2);
    assert_eq!(report.usage.input_tokens, 40);
    assert_eq!(report.usage.output_tokens, 10);
    assert_eq!(report.usage.cost_usd, Some(0.002));
    assert_eq!(report.privacy.granted.len(), 1);
    for candidate in report.candidates {
        if candidate.rule.rule_id.as_str() == "granted.rule" {
            assert!(matches!(
                candidate.outcome,
                CandidateOutcome::Finding { .. }
            ));
        } else {
            assert!(matches!(
                candidate.outcome,
                CandidateOutcome::Dropped { .. }
            ));
        }
        assert_eq!(candidate.answers.len(), 1);
    }
    assert_eq!(server.finish().await, 2);
}

struct BackendWithCredential {
    selection: sunstack_decisions::BackendSelection,
}
impl RuleBackendFactory for BackendWithCredential {
    fn selection(&self) -> &sunstack_decisions::BackendSelection {
        &self.selection
    }
    fn prepare(
        self: Arc<Self>,
    ) -> Result<Arc<dyn sunstack_decisions::DecisionBackend>, DiagnosticList> {
        match sunstack_decisions::JevOpenRouterBackend::prepare(
            self.selection.clone(),
            "SYNTHETIC_TEST_CREDENTIAL",
        ) {
            sunstack_decisions::BackendPreparation::Ready { backend } => Ok(Arc::new(backend)),
            sunstack_decisions::BackendPreparation::Refused { diagnostics } => {
                Err(diagnostics.into())
            }
        }
    }
}
