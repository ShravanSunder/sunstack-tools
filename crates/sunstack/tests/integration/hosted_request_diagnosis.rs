#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../support/journey_fixtures.rs"]
mod journey_fixtures;
#[path = "../support/mod.rs"]
mod support;
use journey_fixtures::HandoffFixture;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use sunstack_decisions::*;

struct EncodingObserver {
    backend: JevOpenRouterBackend,
    bodies: Mutex<Vec<Vec<u8>>>,
}
impl DecisionBackend for EncodingObserver {
    fn identity(&self) -> &BackendIdentity {
        self.backend.identity()
    }
    fn selection(&self) -> &BackendSelection {
        self.backend.selection()
    }
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic> {
        let encoded = self.backend.encode_request(request)?;
        self.bodies.lock().unwrap().push(encoded.body().to_vec());
        Ok(encoded)
    }
    fn account_request(&self, encoded: &EncodedBackendRequest) -> TokenAccounting {
        self.backend.account_request(encoded)
    }
    fn dispatch<'request>(
        &'request self,
        _request: &'request AdmittedBackendRequest,
        _max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        panic!("diagnostic preparation must not dispatch")
    }
    fn admit_response(
        &self,
        _request: &StructuredJudgmentRequest,
        _body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        panic!("diagnostic preparation must not admit a response")
    }
}

#[tokio::test]
#[ignore = "offline reconstruction of a preserved failed hosted invocation; no provider dispatch"]
async fn reconstruct_preserved_handoff_encoding_without_dispatch() {
    std::env::set_current_dir(support::workspace_root()).unwrap();
    let saved = PathBuf::from(std::env::var_os("SUNSTACK_FAILED_INVOCATION").unwrap());
    let wire: Value = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    let input = wire["input"].clone();
    let endpoint = input["backend"]["endpoint"].as_str().unwrap();
    let fixture = HandoffFixture::new(endpoint);
    let fresh = &fixture.input["definitions"]["binding_groups"][0]["bindings"]["session"]["transcript_path"];
    let original = PathBuf::from(
        input["definitions"]["binding_groups"][0]["bindings"]["session"]["transcript_path"]
            .as_str()
            .unwrap(),
    );
    assert!(original.starts_with(std::env::temp_dir()));
    if !original.exists() {
        std::fs::create_dir_all(original.parent().unwrap()).unwrap();
        std::fs::copy(fresh.as_str().unwrap(), &original).unwrap();
    }
    let reconstructed = std::fs::read(&original).unwrap();
    let frozen: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("SUNSTACK_FAILED_FINGERPRINTS").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        json!(sunstack_evidence::content_digest(&reconstructed)),
        frozen["fixture_fingerprints"]["reviewer_transcript"]
    );
    let admitted =
        sunstack_rules::decode_rule_invocation(&serde_json::to_vec(&input).unwrap()).unwrap();
    let observer = Arc::new(EncodingObserver {
        backend: JevOpenRouterBackend::new(admitted.backend.clone(), "OFFLINE_PREPARATION_ONLY")
            .unwrap(),
        bodies: Mutex::new(Vec::new()),
    });
    let prepared = sunstack_rules::prepare_rules(
        admitted,
        sunstack_rules::RuleExecutionContext {
            backend: observer.clone(),
            adapters: sunstack_evidence::AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    assert_eq!(prepared.exit_code(), 0, "{prepared:?}");
    let bodies = observer.bodies.lock().unwrap();
    assert_eq!(
        bodies.len(),
        1,
        "identify the sole first dispatch, rather than choosing a favorable body"
    );
    let body = &bodies[0];
    let destination = saved
        .parent()
        .unwrap()
        .join("reconstructed-request-body.json");
    std::fs::write(&destination, body).unwrap();
    let body_value: Value = serde_json::from_slice(body).unwrap();
    let characteristics = json!({"body_bytes":body.len(),"max_request_bytes":input["request_budget"]["max_request_bytes"],"body_digest":sunstack_evidence::content_digest(body),"questions":body_value["questions"].as_object().unwrap().iter().map(|(name,q)|json!({"name":name,"type":q["type"],"evidence_labels":q["instructions"]["reads"]})).collect::<Vec<_>>(),"preparation":prepared,"actual_provider_calls":0,"reconstructed_transcript_matches_failed_fingerprint":true});
    std::fs::write(
        saved.parent().unwrap().join("request-characteristics.json"),
        serde_json::to_vec_pretty(&characteristics).unwrap(),
    )
    .unwrap();
    println!(
        "encoded request: {} bytes; max {}; questions {}",
        body.len(),
        input["request_budget"]["max_request_bytes"],
        body_value["questions"].as_object().unwrap().len()
    );
}
