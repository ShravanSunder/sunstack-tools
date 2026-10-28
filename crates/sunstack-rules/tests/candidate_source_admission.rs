#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_fixture.rs"]
mod fixture;
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use sunstack_evidence::*;
use sunstack_rules::*;
use tokio_util::sync::CancellationToken;
struct LegacyAdapter {
    name: TranscriptFormatName,
    calls: Arc<AtomicUsize>,
}
impl TranscriptAdapter for LegacyAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.name
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        self.calls.fetch_add(1, Ordering::SeqCst);
        AdapterRegistry::builtins()
            .unwrap()
            .adapter(&TranscriptFormatName::new("claude_code_jsonl").unwrap())
            .unwrap()
            .normalize(capture)
    }
}
struct UnusedBackend {
    selection: sunstack_decisions::BackendSelection,
    calls: Arc<AtomicUsize>,
}
impl RuleBackendFactory for UnusedBackend {
    fn selection(&self) -> &sunstack_decisions::BackendSelection {
        &self.selection
    }
    fn prepare(
        self: Arc<Self>,
    ) -> Result<Arc<dyn sunstack_decisions::DecisionBackend>, sunstack_rules::DiagnosticList> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(sunstack_rules::DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ))
    }
}
#[tokio::test]
async fn adapter_only_format_still_serves_legacy_capture_but_candidate_input_is_unreadable() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("first.jsonl");
    std::fs::write(
        &path,
        json!({"type":"assistant","message":{"content":"MATCH legacy adapter"}}).to_string() + "\n",
    )
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let name = TranscriptFormatName::new("legacy_only").unwrap();
    let registry = AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(Arc::new(LegacyAdapter {
            name: name.clone(),
            calls: calls.clone(),
        }))
        .unwrap();
    let capture = capture_conversation(
        ConversationCaptureRequest {
            transcript_path: path,
            format: name,
            cursor: None,
            final_assistant_text: None,
            max_bytes: 10000,
        },
        registry.clone(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(capture.records.len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut input = fixture::fixture(root.path(), "http://127.0.0.1:9/v1/systemone");
    input["sources"] =
        json!([{"source":{"kind":"session_pair","provider":"fourth-host","session":"first"}}]);
    input["recipes"] = json!([{"provider":"fourth-host","locate":{"roots":[root.path()],"pattern":"{session}.jsonl"},"read":{"format":"legacy_only"},"map":{"fields":{}}}]);
    let input: CandidateRunInput = serde_json::from_value(input).unwrap();
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(UnusedBackend {
        selection: input.backend.clone(),
        calls: backend_calls.clone(),
    });
    let outcome = evaluate_candidates(
        input,
        RuleExecutionContext {
            backend,
            adapters: registry,
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let CandidateRunOutcome::Incomplete { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(matches!(
        report.inputs[0].location,
        InputLocation::Unlocatable {
            reason: UnlocatableReason::Unreadable
        }
    ));
    assert!(matches!(
        report.completeness,
        RunCompleteness::Partial {
            stopped_by: CandidateStopReason::UnlocatableInput,
            ..
        }
    ));
    assert!(report.candidates.is_empty());
    assert_eq!(report.usage.requests, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(backend_calls.load(Ordering::SeqCst), 0);
}
