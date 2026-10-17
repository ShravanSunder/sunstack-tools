#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn real_git_change_maps_to_function_and_keeps_deleted_before_unit() {
    let fixture = GitFixture::new();
    let before = "fn untouched() {}\nfn changed() { old(); }\nfn deleted() {}\n";
    fixture.write("source.rs", before);
    fixture.stage("source.rs");
    fixture.commit(1000);
    fixture.write("source.rs", "fn untouched() {}\nfn changed() { new(); }\n");
    let capture = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let units = changed_source_units(
        &capture,
        &[SourceLanguage::Rust],
        &[UnitKindFilter::Function],
        SourceSides::BothAvailable,
    )
    .unwrap();
    assert_eq!(units.len(), 3);
    assert!(
        units
            .iter()
            .all(|selected| !selected.text.contains("untouched"))
    );
    let deleted = units
        .iter()
        .find(|selected| selected.text == "fn deleted() {}")
        .unwrap();
    assert!(matches!(
        &deleted.unit.key.revision,
        SourceRevision::Index {
            side: SourceSide::Before,
            ..
        }
    ));
    assert_eq!(
        &before[deleted.unit.key.byte_start..deleted.unit.key.byte_end],
        deleted.text
    );
    assert!(
        changed_source_units(
            &capture,
            &[SourceLanguage::Rust],
            &[UnitKindFilter::Function],
            SourceSides::PairedRequired
        )
        .is_err()
    );
}

#[test]
fn real_ast_units_distinguish_methods_classes_and_recovered_syntax() {
    let snapshot = SnapshotId::new("fixture-snapshot").unwrap();
    for (path, text, expected) in [
        (
            "source.ts",
            "export class Thing { run() { return 1; } }\nfunction top() {}",
            vec![
                UnitKindFilter::Class,
                UnitKindFilter::Method,
                UnitKindFilter::Function,
            ],
        ),
        (
            "source.tsx",
            "function View() { return <div/>; }",
            vec![UnitKindFilter::Function],
        ),
        (
            "source.rs",
            "struct Thing; impl Thing { fn run(&self) {} }\nfn top() {}",
            vec![UnitKindFilter::Method, UnitKindFilter::Function],
        ),
    ] {
        let file = frozen(path, text, &snapshot);
        let units = extract_source_units(&file, &snapshot).unwrap();
        assert_eq!(
            units
                .iter()
                .map(|selected| match selected.unit.kind {
                    SourceUnitKind::Function { .. } => UnitKindFilter::Function,
                    SourceUnitKind::Method { .. } => UnitKindFilter::Method,
                    SourceUnitKind::Class { .. } => UnitKindFilter::Class,
                    _ => panic!("unexpected kind"),
                })
                .collect::<Vec<_>>(),
            expected
        );
        for selected in units {
            assert_eq!(
                &text[selected.unit.key.byte_start..selected.unit.key.byte_end],
                selected.text
            );
        }
    }
    let units = extract_source_units(
        &frozen(
            "broken.ts",
            "function broken() { const value = ; }",
            &snapshot,
        ),
        &snapshot,
    )
    .unwrap();
    assert!(
        units
            .iter()
            .any(|selected| selected.unit.syntax_quality == SyntaxQuality::RecoveredWithErrors)
    );
    let unlocated = extract_source_units(
        &frozen("unlocated.ts", "function broken() { return (", &snapshot),
        &snapshot,
    )
    .unwrap();
    assert!(
        unlocated.is_empty(),
        "an ERROR root is not a located function declaration"
    );
}

#[test]
fn markdown_units_preserve_section_and_tight_paragraph_ranges() {
    let snapshot = SnapshotId::new("markdown-snapshot").unwrap();
    let text = "# Parent\n\nFirst paragraph.\n\n- Tight paragraph\n\n## Child\n\nChild text.\n";
    let units = extract_source_units(&frozen("notes.md", text, &snapshot), &snapshot).unwrap();
    let paragraphs = units
        .iter()
        .filter(|selected| matches!(selected.unit.kind, SourceUnitKind::Paragraph))
        .map(|selected| selected.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paragraphs,
        ["First paragraph.", "Tight paragraph", "Child text."]
    );
    let sections = units
        .iter()
        .filter(|selected| matches!(selected.unit.kind, SourceUnitKind::Section))
        .collect::<Vec<_>>();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].text, text.trim_end());
    assert_eq!(sections[1].text, "## Child\n\nChild text.");
}

fn frozen(path: &str, text: &str, snapshot: &SnapshotId) -> FrozenRepositoryFile {
    FrozenRepositoryFile {
        source: SourceRef::RepositoryFile {
            repository: RepositoryId::new("fixture").unwrap(),
            revision: SourceRevision::Worktree {
                snapshot: snapshot.clone(),
                side: SourceSide::After,
            },
            path: path.into(),
            content_identity: content_digest(text.as_bytes()),
        },
        text: text.into(),
        side: SourceSide::After,
    }
}

#[tokio::test]
async fn top_level_insertion_and_deletion_do_not_seed_an_unchanged_neighbor() {
    let fixture = GitFixture::new();
    fixture.write("source.rs", "fn unchanged() {}\n");
    fixture.stage("source.rs");
    fixture.commit(1000);
    fixture.write("source.rs", "fn inserted() {}\nfn unchanged() {}\n");
    let capture = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let units = changed_source_units(
        &capture,
        &[SourceLanguage::Rust],
        &[UnitKindFilter::Function],
        SourceSides::BothAvailable,
    )
    .unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].text, "fn inserted() {}");
    fixture.stage("source.rs");
    fixture.commit(1100);
    fixture.write("source.rs", "fn unchanged() {}\n");
    let capture = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            }),
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let units = changed_source_units(
        &capture,
        &[SourceLanguage::Rust],
        &[UnitKindFilter::Function],
        SourceSides::BothAvailable,
    )
    .unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].text, "fn inserted() {}");
}
