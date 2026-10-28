#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use sunstack_evidence::*;
use support::*;
#[path = "support/code_context_fixture.rs"]
mod fixture;
use fixture::*;

#[tokio::test]
async fn frozen_diff_has_both_whole_sides_and_actual_related_revision_bytes() {
    let (fixture, base, head) = repository(false);
    let mut session = session(&fixture).await;
    let source = SourceInput::Diff {
        repository: RepositoryId::new("fixture").unwrap(),
        scope: ChangeScope::RevisionDiff {
            base_revision: base.clone(),
            head: RevisionHead::Commit {
                revision: head.clone(),
            },
        },
    };
    let index = session.build_code_context(&source).await.unwrap();
    let read = session.measurements().files_read;
    let repeated = session.build_code_context(&source).await.unwrap();
    assert_eq!(session.measurements().files_read, read);
    assert_eq!(index.input_id, repeated.input_id);
    let found = index.discover(Some(&pass("flagged($VALUE)")), 20).unwrap();
    assert_eq!(found.matches.len(), 2);
    for matched in found.matches {
        let context = matched.context;
        let rendered = serde_json::to_string(&context).unwrap();
        assert!(rendered.contains("COMMITTED_CALLER"));
        assert!(rendered.contains("COMMITTED_CALLEE"));
        assert!(!rendered.contains("DIRTY_NOT_AT_REVISION"));
        assert!(
            context
                .units
                .iter()
                .any(|unit| unit.text.contains("unflagged(\"old\")"))
        );
        assert!(
            context
                .units
                .iter()
                .any(|unit| unit.text.contains("CHANGED_MATCH"))
        );
        assert!(
            context
                .imports
                .iter()
                .any(|import| import.text.contains("use crate::helper::callee"))
        );
        assert!(context.related.iter().any(|piece|matches!(piece,RelatedPiece::Unresolved {name_as_written,..} if name_as_written=="missing_name")));
        assert_eq!(context.changes.len(), 1);
        let before = context.changes[0].before.as_ref().unwrap();
        let after = context.changes[0].after.as_ref().unwrap();
        assert!(before.text.contains("unflagged(\"old\")"));
        assert!(after.text.contains("flagged(\"CHANGED_MATCH\")"));
        assert!(
            matches!(&before.span.source,SourceRef::RepositoryFile {revision:SourceRevision::Commit {commit_id,side:SourceSide::Before},..} if commit_id.as_str()==base)
        );
        assert!(
            matches!(&after.span.source,SourceRef::RepositoryFile {revision:SourceRevision::Commit {commit_id,side:SourceSide::After},..} if commit_id.as_str()==head)
        );
    }
}

#[tokio::test]
async fn default_diff_sides_keep_deleted_hits_once_and_explicit_overrides_are_honored() {
    let (fixture, base, head) = repository(true);
    let mut session = session(&fixture).await;
    let source = SourceInput::Diff {
        repository: RepositoryId::new("fixture").unwrap(),
        scope: ChangeScope::RevisionDiff {
            base_revision: base.clone(),
            head: RevisionHead::Commit {
                revision: head.clone(),
            },
        },
    };
    let index = session.build_code_context(&source).await.unwrap();
    let before_text = frozen_text(&fixture, &base, "src/lib.rs");
    let after_text = frozen_text(&fixture, &head, "src/lib.rs");
    for (sides, expected) in [
        (
            None,
            vec![
                (SourceSide::After, "flagged(\"CHANGED_MATCH\")"),
                (SourceSide::After, "flagged(\"UNCHANGED_MATCH\")"),
                (SourceSide::Before, "flagged(\"DELETED_MATCH\")"),
            ],
        ),
        (
            Some(SourceSides::BeforeOnly),
            vec![
                (SourceSide::Before, "flagged(\"UNCHANGED_MATCH\")"),
                (SourceSide::Before, "flagged(\"DELETED_MATCH\")"),
            ],
        ),
        (
            Some(SourceSides::BothAvailable),
            vec![
                (SourceSide::After, "flagged(\"CHANGED_MATCH\")"),
                (SourceSide::After, "flagged(\"UNCHANGED_MATCH\")"),
                (SourceSide::Before, "flagged(\"UNCHANGED_MATCH\")"),
                (SourceSide::Before, "flagged(\"DELETED_MATCH\")"),
            ],
        ),
        (
            Some(SourceSides::AfterOnly),
            vec![
                (SourceSide::After, "flagged(\"CHANGED_MATCH\")"),
                (SourceSide::After, "flagged(\"UNCHANGED_MATCH\")"),
            ],
        ),
    ] {
        let mut pass = pass("flagged($VALUE)");
        pass.source_sides = sides;
        let discovery = index.discover(Some(&pass), 0).unwrap();
        assert_eq!(discovery.matches.len(), expected.len());
        let mut seen = std::collections::BTreeSet::new();
        for matched in discovery.matches {
            let span = &matched.context.matched.span;
            let SourceRef::RepositoryFile {
                revision: SourceRevision::Commit { side, .. },
                ..
            } = &span.source
            else {
                panic!("not commit side")
            };
            let snippet = matched.context.matched.text.as_str();
            assert!(expected.contains(&(*side, snippet)));
            assert!(seen.insert((*side, snippet.to_owned())));
            exact_span(
                span,
                if *side == SourceSide::Before {
                    &before_text
                } else {
                    &after_text
                },
                snippet,
                "src/lib.rs",
                if *side == SourceSide::Before {
                    &base
                } else {
                    &head
                },
                *side,
            );
        }
        assert_eq!(
            seen,
            expected
                .into_iter()
                .map(|(side, snippet)| (side, snippet.to_owned()))
                .collect()
        );
    }
}

#[tokio::test]
async fn out_of_unit_const_keeps_exact_span_and_code_identity_excludes_capture_instance() {
    let (fixture, _, head) = repository(false);
    let input = code(
        &head,
        vec![QueryPath::Exact {
            path: "src/lib.rs".into(),
        }],
    );
    let mut first = session(&fixture).await;
    let first_index = first.build_code_context(&input).await.unwrap();
    let first = first_index
        .discover(Some(&pass("const $NAME: $TYPE = $VALUE;")), 0)
        .unwrap();
    assert_eq!(first.matches.len(), 1);
    let SourceOccurrence::CodeMatch {
        input_id,
        unit,
        span,
    } = &first.matches[0].occurrence
    else {
        panic!("not a code occurrence")
    };
    assert!(unit.is_none());
    assert_eq!(*input_id, input.source_identity().unwrap());
    assert!(first.matches[0].context.matched.text.contains("TOP_MATCH"));
    assert_eq!(
        first.matches[0].context.matched.text,
        r#"const TOP: &str = "TOP_MATCH";"#
    );
    exact_span(
        span,
        &frozen_text(&fixture, &head, "src/lib.rs"),
        r#"const TOP: &str = "TOP_MATCH";"#,
        "src/lib.rs",
        &head,
        SourceSide::After,
    );
    let mut second = session(&fixture).await;
    let second_index = second.build_code_context(&input).await.unwrap();
    let second = second_index
        .discover(Some(&pass("const $NAME: $TYPE = $VALUE;")), 0)
        .unwrap();
    assert_eq!(
        first.matches[0].occurrence.occurrence_identity().unwrap(),
        second.matches[0].occurrence.occurrence_identity().unwrap()
    );
    assert_eq!(first_index.input_id, second_index.input_id);
    let mut ordinary = first_index
        .discover(Some(&pass("flagged($VALUE)")), 0)
        .unwrap()
        .matches
        .remove(0)
        .occurrence;
    let identity = ordinary.occurrence_identity().unwrap();
    if let SourceOccurrence::CodeMatch {
        unit: Some(unit), ..
    } = &mut ordinary
    {
        unit.snapshot = SnapshotId::new("different-capture-instance").unwrap();
    } else {
        panic!("call must have a unit")
    }
    assert_eq!(identity, ordinary.occurrence_identity().unwrap());
}

#[tokio::test]
async fn distinct_overlapping_input_scopes_have_separate_stable_namespaces() {
    let (fixture, _, head) = repository(false);
    let mut session = session(&fixture).await;
    let exact = code(
        &head,
        vec![QueryPath::Exact {
            path: "src/lib.rs".into(),
        }],
    );
    let broad = code(
        &head,
        vec![QueryPath::Glob {
            pattern: "src/**".into(),
        }],
    );
    let first = session.build_code_context(&exact).await.unwrap();
    let second = session.build_code_context(&broad).await.unwrap();
    assert_ne!(first.input_id, second.input_id);
    let first = first.discover(Some(&pass("flagged($VALUE)")), 0).unwrap();
    let second = second.discover(Some(&pass("flagged($VALUE)")), 0).unwrap();
    assert_eq!(first.matches.len(), 2);
    assert_eq!(second.matches.len(), 2);
    let identities = first
        .matches
        .iter()
        .map(|matched| {
            matched
                .occurrence
                .occurrence_identity()
                .unwrap()
                .as_str()
                .to_owned()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(second.matches.iter().all(|matched| {
        !identities.contains(matched.occurrence.occurrence_identity().unwrap().as_str())
    }));
}

#[tokio::test]
async fn worktree_occurrences_are_stable_for_identical_content_and_cached_capture_is_frozen() {
    let (fixture, _, _) = repository(false);
    let input = SourceInput::Code {
        repository: RepositoryId::new("fixture").unwrap(),
        revision: RevisionHead::Worktree {},
        paths: vec![QueryPath::Exact {
            path: "src/lib.rs".into(),
        }],
    };
    let mut first = session(&fixture).await;
    let index = first.build_code_context(&input).await.unwrap();
    let before = index.discover(Some(&pass("flagged($VALUE)")), 0).unwrap();
    let reads = first.measurements().files_read;
    let mut second = session(&fixture).await;
    let identical = second
        .build_code_context(&input)
        .await
        .unwrap()
        .discover(Some(&pass("flagged($VALUE)")), 0)
        .unwrap();
    let ids = |found: &Discovery<DiscoveredCodeMatch>| {
        found
            .matches
            .iter()
            .map(|matched| {
                matched
                    .occurrence
                    .occurrence_identity()
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(ids(&before), ids(&identical));
    let text = std::fs::read_to_string(fixture.directory.path().join("src/lib.rs")).unwrap();
    fixture.write(
        "src/lib.rs",
        &text.replace("\"CHANGED_MATCH\"", "\"ALTERED_MATCH\""),
    );
    let cached = first
        .build_code_context(&input)
        .await
        .unwrap()
        .discover(Some(&pass("flagged($VALUE)")), 0)
        .unwrap();
    assert_eq!(reads, first.measurements().files_read);
    assert_eq!(ids(&before), ids(&cached));
    assert!(cached.matches.iter().any(|matched| {
        matched.context.matched.text.contains("CHANGED_MATCH")
            && !matched.context.matched.text.contains("UNCHANGED_MATCH")
    }));
    let mut fresh = session(&fixture).await;
    let changed = fresh
        .build_code_context(&input)
        .await
        .unwrap()
        .discover(Some(&pass("flagged($VALUE)")), 0)
        .unwrap();
    assert_ne!(ids(&before), ids(&changed));
}

#[tokio::test]
async fn supported_code_languages_match_their_actual_ast_nodes() {
    for (language, path, text) in [
        (
            "typescript",
            "code.ts",
            "function example() { flagged('TYPE_SCRIPT_MATCH'); }",
        ),
        (
            "tsx",
            "code.tsx",
            "function Box() { flagged('TSX_MATCH'); return <div/>; }",
        ),
    ] {
        let fixture = GitFixture::new();
        fixture.write(path, text);
        fixture.stage(path);
        let head = fixture.commit(1000).to_string();
        let mut session = session(&fixture).await;
        let input = code(&head, vec![QueryPath::Exact { path: path.into() }]);
        let index = session.build_code_context(&input).await.unwrap();
        let pass: MechanicalPass = serde_json::from_value(
            json!({"predicate":{"kind":"ast","language":language,"pattern":"flagged($VALUE)"}}),
        )
        .unwrap();
        let matches = index.discover(Some(&pass), 0).unwrap();
        assert_eq!(matches.matches.len(), 1);
        assert!(
            matches.matches[0]
                .context
                .matched
                .text
                .starts_with("flagged(")
        );
    }
}
