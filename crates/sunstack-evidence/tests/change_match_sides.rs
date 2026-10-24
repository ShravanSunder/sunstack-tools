#![allow(clippy::unwrap_used, clippy::panic)]
mod support;
use serde_json::{Value, json};
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

fn selectors(sides: Value) -> Vec<EvidenceSelection> {
    [
        json!({"kind":"change_set_literal_text","text":"this.ctx.storage.put"}),
        json!({"kind":"change_set_regex_text","pattern":"this\\.ctx\\.storage\\.put"}),
        json!({"kind":"change_set_ast_pattern","language":"typescript","pattern":"$OBJ.storage.put($$$)"}),
    ].into_iter().map(|mut select| {
        select["paths"] = json!([{"kind":"exact","path":"source.ts"}]);
        select["sides"] = sides.clone();
        serde_json::from_value(select).unwrap()
    }).collect()
}

#[tokio::test]
async fn each_match_side_retains_exact_changed_or_whole_file_spans_for_every_selector() {
    let fixture = GitFixture::new();
    fixture.write(
        "source.ts",
        "this.ctx.storage.put('existing', 0);\nunchanged();\nthis.ctx.storage.put('removed', 1);\n",
    );
    fixture.stage("source.ts");
    fixture.commit(1000);
    fixture.write(
        "source.ts",
        "this.ctx.storage.put('existing', 0);\nunchanged();\nthis.ctx.storage.put('added', 2);\n",
    );
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
    for (sides, expected) in [
        (json!(["added"]), vec![(SourceSide::After, 3)]),
        (json!(["removed"]), vec![(SourceSide::Before, 3)]),
        (
            json!(["after_file"]),
            vec![(SourceSide::After, 1), (SourceSide::After, 3)],
        ),
        (
            json!(["before_file"]),
            vec![(SourceSide::Before, 1), (SourceSide::Before, 3)],
        ),
        (
            json!(["added", "after_file"]),
            vec![(SourceSide::After, 1), (SourceSide::After, 3)],
        ),
    ] {
        for selector in selectors(sides.clone()) {
            let selection = select_change_set(&capture, &selector, 100).unwrap();
            let SelectionContent::Matches { matches } = selection.content else {
                panic!("expected matches")
            };
            let locations = matches
                .iter()
                .map(|found| {
                    let SourceRef::RepositoryFile { path, revision, .. } = &found.span.source
                    else {
                        panic!("expected file source")
                    };
                    assert_eq!(path.to_str(), Some("source.ts"));
                    assert!(found.span.byte_end > found.span.byte_start);
                    let side = match revision {
                        SourceRevision::Commit { side, .. }
                        | SourceRevision::Index { side, .. }
                        | SourceRevision::Worktree { side, .. } => *side,
                    };
                    (side, found.span.start_line)
                })
                .collect::<Vec<_>>();
            assert_eq!(locations, expected, "{selector:?}");
        }
    }
}

#[tokio::test]
async fn disjoint_edits_do_not_select_existing_calls_and_removed_lines_have_no_added_match() {
    let fixture = GitFixture::new();
    fixture.write(
        "source.ts",
        "this.ctx.storage.put('existing', 0);\nunchanged();\n",
    );
    fixture.stage("source.ts");
    fixture.commit(1000);
    for after in [
        "this.ctx.storage.put('existing', 0);\nchanged();\n",
        "unchanged();\n",
    ] {
        fixture.write("source.ts", after);
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
        for selector in selectors(json!(["added"])) {
            assert!(
                matches!(select_change_set(&capture,&selector,100).unwrap().content,
                SelectionContent::Matches { matches } if matches.is_empty())
            );
        }
    }
}

#[test]
fn missing_or_empty_sides_refuse_before_any_change_source_is_opened() {
    let sources = std::collections::BTreeMap::from([(
        SourceName::new("changes").unwrap(),
        SourceSpec::ChangeSet {
            repository: RepositoryId::new("unread").unwrap(),
        },
    )]);
    for mut selector in [
        json!({"kind":"change_set_literal_text","text":"call"}),
        json!({"kind":"change_set_regex_text","pattern":"call"}),
        json!({"kind":"change_set_ast_pattern","language":"typescript","pattern":"call()"}),
    ] {
        selector["paths"] = json!([{"kind":"exact","path":"not-created.ts"}]);
        assert!(serde_json::from_value::<EvidenceSelection>(selector.clone()).is_err());
        selector["sides"] = json!([]);
        let declaration = EvidenceDeclaration {
            id: DeclarationId::new("target").unwrap(),
            input: SourceName::new("changes").unwrap(),
            requirement: EvidenceRequirement::Required,
            select: serde_json::from_value(selector).unwrap(),
            max_items: None,
            max_bytes: None,
        };
        assert_eq!(
            admit_evidence_declaration(&declaration, &sources)
                .unwrap_err()
                .code,
            EvidenceCode::InvalidSelector
        );
    }
}
