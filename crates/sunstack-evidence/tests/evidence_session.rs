#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::json;
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
fn request(root: &std::path::Path) -> EvidencePlanInput {
    serde_json::from_value(json!({
        "repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}],
        "sources":{"first":{"kind":"document","repository":"repo","path":"first.md"},"alias":{"kind":"document","repository":"repo","path":"./first.md"},"later":{"kind":"document","repository":"repo","path":"later.md"}},
        "payload_schemas":[],"cursors":[],"declarations":[],"native_queries":[],"package_bindings":[],
        "limits":{"max_files":1,"max_file_bytes":4096,"max_total_bytes":8192,"max_matches":8,"max_transcript_bytes":4096,"include_ignored":false},"discovery":{"kind":"none"}
    })).unwrap()
}
fn selection(id: &str, input: &str) -> EvidenceDeclaration {
    serde_json::from_value(json!({"id":id,"input":input,"select":{"kind":"whole_document"}}))
        .unwrap()
}
#[tokio::test]
async fn sequential_alias_selection_uses_frozen_bytes_after_file_changes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.md"), "ORIGINAL CAPTURE").unwrap();
    let mut session = EvidenceSession::open(
        request(root.path()),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let first = session.select(&selection("first_selection", "first")).await;
    std::fs::write(root.path().join("first.md"), "MUTATED CONTENT").unwrap();
    let alias = session.select(&selection("alias_selection", "alias")).await;
    for result in [first, alias] {
        assert!(
            matches!(result.readiness,SelectionReadiness::Ready{content:SelectionContent::DocumentText{text,..},..} if text=="ORIGINAL CAPTURE")
        );
    }
    let closed = session.close();
    assert_eq!(closed.measurements.files_read, 1);
    assert_eq!(closed.measurements.bytes_scanned, 32);
}
#[tokio::test]
async fn distinct_later_selection_obeys_the_same_cumulative_file_limit() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.md"), "first").unwrap();
    std::fs::write(root.path().join("later.md"), "later").unwrap();
    let mut session = EvidenceSession::open(
        request(root.path()),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        session.select(&selection("one", "first")).await.readiness,
        SelectionReadiness::Ready { .. }
    ));
    let second = session.select(&selection("two", "later")).await;
    assert!(
        matches!(second.readiness,SelectionReadiness::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::EvidenceLimit)
    );
    assert_eq!(session.close().measurements.files_read, 1);
}
#[tokio::test]
async fn opening_and_refusing_invalid_selection_reads_no_bound_document() {
    let root = tempfile::tempdir().unwrap();
    let mut session = EvidenceSession::open(
        request(root.path()),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let mut invalid = selection("invalid", "first");
    invalid.select = EvidenceSelection::WholePayload {};
    assert!(
        matches!(session.select(&invalid).await.readiness,SelectionReadiness::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::InvalidSelector)
    );
    assert_eq!(session.close().measurements.files_read, 0);
}

async fn frozen_alias_after_path_removal(replace: bool) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("first.md");
    std::fs::write(&path, "FROZEN ORIGINAL").unwrap();
    let mut session = EvidenceSession::open(
        request(root.path()),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(matches!(
        session
            .select(&selection("first_capture", "first"))
            .await
            .readiness,
        SelectionReadiness::Ready { .. }
    ));
    std::fs::remove_file(&path).unwrap();
    if replace {
        std::fs::write(&path, "REPLACEMENT BYTES").unwrap();
    }
    let alias = session.select(&selection("later_alias", "alias")).await;
    assert!(
        matches!(alias.readiness,SelectionReadiness::Ready{content:SelectionContent::DocumentText{text,..},..} if text=="FROZEN ORIGINAL")
    );
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 1);
    assert_eq!(summary.measurements.files_considered, 1);
}
#[tokio::test]
async fn deleted_source_alias_keeps_its_final_capture_without_live_path_access() {
    frozen_alias_after_path_removal(false).await;
}
#[tokio::test]
async fn replaced_source_alias_keeps_its_final_capture_without_live_path_access() {
    frozen_alias_after_path_removal(true).await;
}
#[tokio::test]
async fn deleted_transcript_alias_keeps_the_same_frozen_capture() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("session.jsonl");
    let bytes = include_str!("fixtures/codex-composed.jsonl");
    std::fs::write(&path, bytes).unwrap();
    let mut input = request(root.path());
    let spec = |path| SourceSpec::Conversation {
        transcript_path: path,
        format: TranscriptFormatName::new("codex_rollout_jsonl").unwrap(),
        final_assistant_text: None,
    };
    input.sources = std::collections::BTreeMap::from([
        (SourceName::new("first").unwrap(), spec(path.clone())),
        (
            SourceName::new("alias").unwrap(),
            spec(root.path().join("./session.jsonl")),
        ),
    ]);
    let mut first = selection("first_capture", "first");
    first.select = EvidenceSelection::Conversation {
        turns: TurnWindow::All {},
        user_messages: Inclusion::Included,
        assistant_messages: AssistantSelection::All,
        tool_use: ToolUseSelection::Included {
            tools: ToolFilter::All {},
            results: Inclusion::Included,
            input_pattern: None,
        },
        hook_injected: Inclusion::Included,
    };
    let mut alias = first.clone();
    alias.id = DeclarationId::new("later_alias").unwrap();
    alias.input = SourceName::new("alias").unwrap();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let captured = session.select(&first).await;
    assert!(matches!(
        captured.readiness,
        SelectionReadiness::Ready { .. }
    ));
    std::fs::remove_file(&path).unwrap();
    let reused = session.select(&alias).await;
    assert_eq!(
        serde_json::to_string(&captured.readiness).unwrap(),
        serde_json::to_string(&reused.readiness).unwrap()
    );
    let summary = session.close();
    assert_eq!(summary.captures.len(), 1);
    assert_eq!(
        summary.measurements.transcript_bytes_read,
        bytes.len() as u64
    );
}
#[cfg(unix)]
#[tokio::test]
async fn first_capture_still_refuses_an_outside_root_symlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("outside.md"), "OUTSIDE SENTINEL").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("outside.md"),
        root.path().join("first.md"),
    )
    .unwrap();
    let mut session = EvidenceSession::open(
        request(root.path()),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let result = session.select(&selection("first_capture", "first")).await;
    assert!(
        matches!(result.readiness,SelectionReadiness::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::PathOutsideRoot)
    );
    assert_eq!(session.close().measurements.files_read, 0);
}
