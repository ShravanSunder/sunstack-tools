#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;

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
