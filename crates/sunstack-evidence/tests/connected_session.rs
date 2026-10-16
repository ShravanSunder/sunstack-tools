#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn document_then_worktree_change_set_reuses_one_capture_after_source_deletion() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function original() {}");
    let mut input = request(
        &fixture,
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Untracked],
        },
    );
    input.limits.max_files = 1.try_into().unwrap();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let document = session
        .select(&declaration(
            "doc",
            "document",
            EvidenceSelection::WholeDocument,
        ))
        .await;
    std::fs::remove_file(fixture.directory.path().join("source.ts")).unwrap();
    let changed = session
        .select(&declaration("changed", "changes", units()))
        .await;
    assert!(
        matches!(document.readiness, SelectionReadiness::Ready { content: SelectionContent::DocumentText { text, .. }, .. } if text == "function original() {}")
    );
    assert!(
        matches!(changed.readiness, SelectionReadiness::Ready { content: SelectionContent::ChangedUnits { units }, .. } if units.len() == 1 && units[0].text == "function original() {}")
    );
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 1);
    assert_eq!(summary.captures.len(), 1);
    assert_eq!(summary.change_selections.len(), 1);
}
#[tokio::test]
async fn worktree_change_set_then_document_alias_keeps_frozen_locations_after_replacement() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function original() {}");
    let mut session = EvidenceSession::open(
        request(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Untracked],
            },
        ),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let changed = session
        .select(&declaration("changed", "changes", units()))
        .await;
    fixture.write("source.ts", "function replacement() {}");
    let document = session
        .select(&declaration(
            "doc",
            "alias",
            EvidenceSelection::WholeDocument,
        ))
        .await;
    let SelectionReadiness::Ready {
        content: SelectionContent::ChangedUnits { units },
        ..
    } = changed.readiness
    else {
        panic!("changed unit missing");
    };
    let SelectionReadiness::Ready {
        content: SelectionContent::DocumentText { text, span },
        ..
    } = document.readiness
    else {
        panic!("frozen document missing");
    };
    assert_eq!(text, "function original() {}");
    assert_eq!(text, units[0].text);
    assert_eq!(span, units[0].unit.span);
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 1);
    assert_eq!(summary.captures.len(), 1);
}
#[tokio::test]
async fn staged_blob_and_worktree_document_at_one_path_are_distinct_captured_versions() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function staged() {}");
    fixture.stage("source.ts");
    fixture.write("source.ts", "function working() {}");
    let mut session = EvidenceSession::open(
        request(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Staged],
            },
        ),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let staged = session
        .select(&declaration("staged", "changes", units()))
        .await;
    let document = session
        .select(&declaration(
            "doc",
            "document",
            EvidenceSelection::WholeDocument,
        ))
        .await;
    assert!(
        matches!(staged.readiness, SelectionReadiness::Ready { content: SelectionContent::ChangedUnits { units }, .. } if units.len() == 1 && units[0].text == "function staged() {}" && matches!(units[0].unit.key.revision, SourceRevision::Index { .. }))
    );
    assert!(
        matches!(document.readiness, SelectionReadiness::Ready { content: SelectionContent::DocumentText { text, .. }, .. } if text == "function working() {}")
    );
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 2);
    assert_eq!(summary.captures.len(), 2);
}
fn units() -> EvidenceSelection {
    EvidenceSelection::ChangedUnits {
        languages: vec![SourceLanguage::Typescript],
        unit_kinds: vec![UnitKindFilter::Function],
        source_sides: SourceSides::AfterOnly,
    }
}
fn declaration(id: &str, input: &str, select: EvidenceSelection) -> EvidenceDeclaration {
    EvidenceDeclaration {
        id: DeclarationId::new(id).unwrap(),
        input: SourceName::new(input).unwrap(),
        requirement: EvidenceRequirement::Required,
        select,
        max_items: None,
        max_bytes: None,
    }
}
fn request(fixture: &GitFixture, scope: ChangeScope) -> EvidencePlanInput {
    serde_json::from_value(json!({
        "repositories": [fixture.binding(scope)],
        "sources": { "document": { "kind": "document", "repository": "fixture", "path": "source.ts" }, "alias": { "kind": "document", "repository": "fixture", "path": "./source.ts" }, "changes": { "kind": "change_set", "repository": "fixture" } },
        "payload_schemas": [], "cursors": [], "declarations": [], "native_queries": [], "package_bindings": [],
        "limits": { "max_files": 100, "max_file_bytes": 1000000, "max_total_bytes": 10000000, "max_matches": 100, "max_transcript_bytes": 10000, "include_ignored": false }, "discovery": { "kind": "bounded", "limits": discovery_limits() }
    })).unwrap()
}

#[tokio::test]
async fn unused_non_git_change_scope_is_lazy_and_the_used_scope_is_reported_once() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function selected() {}");
    let unused_root = tempfile::tempdir().unwrap();
    let mut input = request(
        &fixture,
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Untracked],
        },
    );
    input.repositories.push(RepositoryBinding {
        key: RepositoryId::new("unused").unwrap(),
        root: unused_root.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "**".into(),
        }],
        change_scope: ChangeScope::RevisionDiff {
            base_revision: "missing-baseline".into(),
            head: RevisionHead::Worktree,
        },
    });
    input.sources.insert(
        SourceName::new("unused_changes").unwrap(),
        SourceSpec::ChangeSet {
            repository: RepositoryId::new("unused").unwrap(),
        },
    );
    let mut session = EvidenceSession::open(
        input.clone(),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    for id in ["first", "alias"] {
        assert!(matches!(
            session
                .select(&declaration(id, "changes", units()))
                .await
                .readiness,
            SelectionReadiness::Ready { .. }
        ));
    }
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 1);
    assert_eq!(summary.change_selections.len(), 1);
    assert_eq!(summary.change_selections[0].repository.as_str(), "fixture");
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let result = session
        .select(&declaration("use_unused", "unused_changes", units()))
        .await;
    assert!(
        matches!(result.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::InvalidRepositoryBinding))
    );
}

#[tokio::test]
async fn failed_utf8_acquisition_still_consumes_the_invocation_file_and_byte_budget() {
    let fixture = GitFixture::new();
    std::fs::write(fixture.directory.path().join("source.ts"), [0xff; 20]).unwrap();
    fixture.write("later.ts", "function later() {}");
    for bound_files in [true, false] {
        let mut input = request(
            &fixture,
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Untracked],
            },
        );
        input.sources.insert(
            SourceName::new("later").unwrap(),
            SourceSpec::Document {
                repository: RepositoryId::new("fixture").unwrap(),
                path: "later.ts".into(),
            },
        );
        if bound_files {
            input.limits.max_files = 1.try_into().unwrap();
        } else {
            input.limits.max_total_bytes = 20.try_into().unwrap();
        }
        let mut session = EvidenceSession::open(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let failed = session
            .select(&declaration(
                "bad_utf8",
                "document",
                EvidenceSelection::WholeDocument,
            ))
            .await;
        assert!(
            matches!(failed.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::UnsupportedEncoding)
        );
        let later = session
            .select(&declaration(
                "later",
                "later",
                EvidenceSelection::WholeDocument,
            ))
            .await;
        assert!(
            matches!(later.readiness, SelectionReadiness::Refused { diagnostics } if diagnostics.items[0].code == EvidenceCode::EvidenceLimit),
            "failed acquisition already consumed the declared physical read budget"
        );
        assert_eq!(session.close().measurements.files_read, 1);
    }
}
