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
    invalid.select = EvidenceSelection::WholePayload;
    assert!(
        matches!(session.select(&invalid).await.readiness,SelectionReadiness::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::InvalidSelector)
    );
    assert_eq!(session.close().measurements.files_read, 0);
}
