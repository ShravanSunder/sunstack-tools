#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::*;
#[path = "support/code_context_fixture.rs"]
mod fixture;
use fixture::*;
#[tokio::test]
async fn repository_scope_keeps_all_matches_diff_scope_only_touched_and_callees_and_nested_once() {
    let fixture = GitFixture::new();
    let before = "mod helper; mod caller; mod imported;\nuse crate::helper::callee; use crate::imported::only_imported;\npub fn touched() {\n unflagged(\"old\");\n flagged(\"TOUCHED_SCOPE\");\n fn inner() { flagged(\"NESTED_SCOPE\"); }\n inner(); callee();\n}\npub fn unrelated() { flagged(\"UNRELATED_SCOPE\"); }\n";
    let files = [
        ("src/lib.rs", before),
        (
            "src/helper.rs",
            "pub fn callee() { flagged(\"CALLEE_SCOPE\"); }\n",
        ),
        (
            "src/caller.rs",
            "pub fn caller() { crate::touched(); flagged(\"CALLER_SCOPE\"); }\n",
        ),
        (
            "src/imported.rs",
            "pub fn only_imported() { flagged(\"IMPORT_ONLY_SCOPE\"); }\n",
        ),
    ];
    for (path, text) in files {
        fixture.write(path, text);
        fixture.stage(path);
    }
    let base = fixture.commit(1000).to_string();
    fixture.write(
        "src/lib.rs",
        &before.replace("unflagged(\"old\")", "unflagged(\"new\")"),
    );
    fixture.stage("src/lib.rs");
    let head = fixture.commit(1100).to_string();
    let mut session = session(&fixture).await;
    for diff in [false, true] {
        let input = if diff {
            SourceInput::Diff {
                repository: RepositoryId::new("fixture").unwrap(),
                scope: ChangeScope::RevisionDiff {
                    base_revision: base.clone(),
                    head: RevisionHead::Commit {
                        revision: head.clone(),
                    },
                },
            }
        } else {
            code(
                &head,
                vec![QueryPath::Glob {
                    pattern: "src/**".into(),
                }],
            )
        };
        let index = session.build_code_context(&input).await.unwrap();
        let found = index.discover(Some(&pass("flagged($VALUE)")), 0).unwrap();
        let expected = if diff {
            vec!["TOUCHED_SCOPE", "NESTED_SCOPE", "CALLEE_SCOPE"]
        } else {
            vec![
                "TOUCHED_SCOPE",
                "NESTED_SCOPE",
                "CALLEE_SCOPE",
                "CALLER_SCOPE",
                "IMPORT_ONLY_SCOPE",
                "UNRELATED_SCOPE",
            ]
        };
        assert_eq!(
            found.matches.len(),
            expected.len(),
            "{:?}",
            found
                .matches
                .iter()
                .map(|matched| &matched.context.matched.text)
                .collect::<Vec<_>>()
        );
        let mut seen = std::collections::BTreeSet::new();
        for matched in found.matches {
            let label = expected
                .iter()
                .find(|label| matched.context.matched.text == format!("flagged(\"{label}\")"))
                .unwrap();
            assert!(seen.insert(*label));
            let (unit, span) = match &matched.occurrence {
                SourceOccurrence::CodeMatch { unit, span, .. }
                | SourceOccurrence::ContextMatch { unit, span, .. } => {
                    (unit.as_ref().unwrap(), span)
                }
                _ => panic!("not code"),
            };
            assert_eq!(matched.context.matched.span, *span);
            assert!(
                matched
                    .context
                    .units
                    .iter()
                    .any(|candidate| candidate.unit.key == *unit)
            );
            let path = unit.path.to_str().unwrap();
            exact_span(
                span,
                &frozen_text(&fixture, &head, path),
                &format!("flagged(\"{label}\")"),
                path,
                &head,
                SourceSide::After,
            );
            if *label == "NESTED_SCOPE" {
                assert!(
                    matched
                        .context
                        .units
                        .iter()
                        .find(|candidate| candidate.unit.key == *unit)
                        .unwrap()
                        .text
                        .starts_with("fn inner")
                );
            }
            if diff {
                let SourceOccurrence::ContextMatch { part, .. } = matched.occurrence else {
                    panic!("not diff")
                };
                assert_eq!(
                    part,
                    if *label == "CALLEE_SCOPE" {
                        ContextPart::Related
                    } else {
                        ContextPart::TouchedUnit
                    }
                );
                assert!(
                    !matched.context.changes.is_empty(),
                    "related/nested match keeps its changed seed context"
                );
                assert!(
                    matched
                        .context
                        .units
                        .iter()
                        .any(|unit| unit.text.starts_with("pub fn touched")
                            && unit.text.contains("unflagged(\"old\")"))
                );
                assert!(
                    matched
                        .context
                        .units
                        .iter()
                        .any(|unit| unit.text.starts_with("pub fn touched")
                            && unit.text.contains("unflagged(\"new\")"))
                );
            }
        }
        assert_eq!(seen, expected.into_iter().collect());
    }
}

#[tokio::test]
async fn overlapping_worktree_inputs_share_frozen_file_truth_across_distinct_scope_caches() {
    let (fixture, _, _) = repository(false);
    let mut session = session(&fixture).await;
    let first = SourceInput::Code {
        repository: RepositoryId::new("fixture").unwrap(),
        revision: RevisionHead::Worktree {},
        paths: vec![QueryPath::Exact {
            path: "src/lib.rs".into(),
        }],
    };
    let second = SourceInput::Code {
        repository: RepositoryId::new("fixture").unwrap(),
        revision: RevisionHead::Worktree {},
        paths: vec![QueryPath::Glob {
            pattern: "src/**".into(),
        }],
    };
    let frozen = session.build_code_context(&first).await.unwrap();
    let reads = session.measurements().files_read;
    let text = std::fs::read_to_string(fixture.directory.path().join("src/lib.rs")).unwrap();
    fixture.write(
        "src/lib.rs",
        &text.replace("\"CHANGED_MATCH\"", "\"ALTERED_MATCH\""),
    );
    let other = session.build_code_context(&second).await.unwrap();
    assert_ne!(frozen.input_id, other.input_id);
    assert_eq!(
        reads,
        session.measurements().files_read,
        "same file objects are already frozen under this session"
    );
    let found = other.discover(Some(&pass("flagged($VALUE)")), 0).unwrap();
    assert!(
        found
            .matches
            .iter()
            .any(|matched| matched.context.matched.text == "flagged(\"CHANGED_MATCH\")")
    );
    assert!(
        !found
            .matches
            .iter()
            .any(|matched| matched.context.matched.text.contains("ALTERED_MATCH"))
    );
}

#[tokio::test]
async fn paired_required_keeps_missing_counterpart_unknown_instead_of_claiming_complete_discovery()
{
    let repository = GitFixture::new();
    repository.write("src/lib.rs", "fn stable() {}\n");
    repository.stage("src/lib.rs");
    let before = repository.commit(1000).to_string();
    repository.write(
        "src/lib.rs",
        "fn stable() {}\nfn added() { flagged(\"NEW_UNIT\"); }\n",
    );
    repository.stage("src/lib.rs");
    let after = repository.commit(1100).to_string();
    let mut session = session(&repository).await;
    let input = SourceInput::Diff {
        repository: RepositoryId::new("fixture").unwrap(),
        scope: ChangeScope::RevisionDiff {
            base_revision: before,
            head: RevisionHead::Commit { revision: after },
        },
    };
    let index = session.build_code_context(&input).await.unwrap();
    let mut pass = pass("flagged($VALUE)");
    pass.source_sides = Some(SourceSides::PairedRequired);
    let discovered = index.discover(Some(&pass), 0).unwrap();
    assert!(!discovered.completed);
    assert!(discovered.matches.is_empty());
    assert!(
        matches!(discovered.coverage,Some(DiscoveryCoverage::Unavailable {diagnostics}) if diagnostics.items.iter().any(|diagnostic|diagnostic.code==EvidenceCode::RequiredGroupIncomplete))
    );
}
