#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::json;
use std::{path::Path, sync::Arc};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;

enum BudgetBound {
    Files,
    Bytes,
}
#[tokio::test]
async fn refused_utf8_capture_consumes_the_file_budget() {
    refused_document_consumes_budget(BudgetBound::Files).await;
}
#[tokio::test]
async fn refused_utf8_capture_consumes_the_byte_budget() {
    refused_document_consumes_budget(BudgetBound::Bytes).await;
}
async fn refused_document_consumes_budget(bound: BudgetBound) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.md"), [0xff; 20]).unwrap();
    std::fs::write(root.path().join("later.md"), b"ok").unwrap();
    let mut input = request(root.path());
    match bound {
        BudgetBound::Files => input.limits.max_files = 1.try_into().unwrap(),
        BudgetBound::Bytes => input.limits.max_total_bytes = 20.try_into().unwrap(),
    }
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let failed = session.select(&selection("bad", "first")).await;
    assert!(
        matches!(failed.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::UnsupportedEncoding)
    );
    let later = session.select(&selection("later", "later")).await;
    assert!(
        matches!(later.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::EvidenceLimit)
    );
    assert_eq!(session.close().measurements.files_read, 1);
}
struct InvalidPositionAdapter {
    format: TranscriptFormatName,
}
impl TranscriptAdapter for InvalidPositionAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.format
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        assert_eq!(capture.bytes, b"bad\n");
        AdapterOutput {
            records: vec![ConversationRecord {
                record_id: RecordId::new("outside").unwrap(),
                position: 4,
                body: ConversationBody::AssistantMessage { blocks: vec![] },
                provenance: RecordProvenance::Transcript,
                injection: InjectionOrigin::NotInjected,
            }],
            turns: vec![],
            gaps: vec![],
        }
    }
}
#[tokio::test]
async fn refused_adapter_output_still_consumes_transcript_bytes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("transcript.jsonl"), b"bad\n").unwrap();
    std::fs::write(root.path().join("later.md"), b"ok").unwrap();
    let mut input = request(root.path());
    input.limits.max_total_bytes = 4.try_into().unwrap();
    let format = TranscriptFormatName::new("invalid-position").unwrap();
    input.sources.insert(
        SourceName::new("transcript").unwrap(),
        SourceSpec::Conversation {
            transcript_path: root.path().join("transcript.jsonl"),
            format: format.clone(),
            final_assistant_text: None,
        },
    );
    let registry = AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(Arc::new(InvalidPositionAdapter { format }))
        .unwrap();
    let mut session = EvidenceSession::open(input, registry, CancellationToken::new())
        .await
        .unwrap();
    let mut declaration = selection("transcript", "transcript");
    declaration.select = EvidenceSelection::Conversation {
        turns: TurnWindow::All {},
        user_messages: Inclusion::Included,
        assistant_messages: AssistantSelection::All,
        tool_use: ToolUseSelection::Excluded {},
        hook_injected: Inclusion::Excluded,
    };
    let failed = session.select(&declaration).await;
    assert!(
        matches!(failed.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::AdapterOutputInvalid)
    );
    let later = session.select(&selection("later", "later")).await;
    assert!(
        matches!(later.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::EvidenceLimit)
    );
    assert_eq!(session.close().measurements.transcript_bytes_read, 4);
}
#[tokio::test]
async fn metadata_rejection_consumes_no_content_budget() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.md"), [b'x'; 100]).unwrap();
    std::fs::write(root.path().join("later.md"), b"ok").unwrap();
    let mut input = request(root.path());
    input.limits.max_files = 1.try_into().unwrap();
    input.limits.max_file_bytes = 20.try_into().unwrap();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        session
            .select(&selection("too_big", "first"))
            .await
            .readiness,
        SelectionReadiness::Refused { .. }
    ));
    assert!(
        matches!(session.select(&selection("later", "later")).await.readiness, SelectionReadiness::Ready { content: SelectionContent::DocumentText { text, .. }, .. } if text == "ok")
    );
    assert_eq!(session.close().measurements.files_read, 1);
}
fn request(root: &Path) -> EvidencePlanInput {
    serde_json::from_value(json!({ "repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}], "sources":{"first":{"kind":"document","repository":"repo","path":"first.md"},"later":{"kind":"document","repository":"repo","path":"later.md"}}, "payload_schemas":[],"cursors":[],"declarations":[],"native_queries":[],"package_bindings":[], "limits":{"max_files":8,"max_file_bytes":4096,"max_total_bytes":8192,"max_matches":8,"max_transcript_bytes":4096,"include_ignored":false},"discovery":{"kind":"none"} })).unwrap()
}
fn selection(id: &str, input: &str) -> EvidenceDeclaration {
    EvidenceDeclaration {
        id: DeclarationId::new(id).unwrap(),
        input: SourceName::new(input).unwrap(),
        requirement: EvidenceRequirement::Required,
        select: EvidenceSelection::WholeDocument {},
        max_items: None,
        max_bytes: None,
    }
}
