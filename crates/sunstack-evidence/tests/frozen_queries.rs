#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn change_set_primitives_keep_sides_paths_and_every_read_source() {
    let fixture = GitFixture::new();
    fixture.write("changed.ts", "function selected() { old(); }");
    fixture.write("unchanged.ts", "function selected() { untouched(); }");
    fixture.stage("changed.ts");
    fixture.stage("unchanged.ts");
    fixture.commit(1000);
    fixture.write("changed.ts", "function selected() { newer(); }");
    let captured = capture_change_selection(
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
    assert_eq!(
        captured.sources.len(),
        4,
        "both read versions of unchanged input remain attributable"
    );
    for selector in [
        EvidenceSelection::ChangeSetLiteralText {
            paths: vec![QueryPath::Exact {
                path: "./changed.ts".into(),
            }],
            text: "selected".into(),
        },
        EvidenceSelection::ChangeSetRegexText {
            paths: vec![QueryPath::Glob {
                pattern: "*.ts".into(),
            }],
            pattern: "selected".into(),
        },
        EvidenceSelection::ChangeSetAstPattern {
            paths: vec![QueryPath::Glob {
                pattern: "*.ts".into(),
            }],
            language: "typescript".into(),
            pattern: "function $NAME() { $$$BODY }".into(),
        },
    ] {
        let SelectionContent::Matches { matches } =
            select_change_set(&captured, &selector, 10).unwrap().content
        else {
            panic!("expected matches");
        };
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|found| matches!(&found.span.source, SourceRef::RepositoryFile { path, .. } if path.to_str() == Some("changed.ts"))));
        let limited = select_change_set(&captured, &selector, 1).unwrap();
        assert!(
            limited
                .cutoffs
                .iter()
                .any(|cutoff| cutoff.limit == DiscoveryLimit::Matches)
        );
        assert!(
            matches!(limited.content, SelectionContent::Matches { matches } if matches.len() == 1)
        );
    }
}
#[tokio::test]
async fn native_selector_seeds_work_on_full_plain_root_and_changed_seeds_refuse() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("source.ts"),
        "function chosen() {}\nfunction other() {}",
    )
    .unwrap();
    let captured = capture_repository_context(
        ChangeCaptureRequest {
            binding: RepositoryBinding {
                key: RepositoryId::new("plain").unwrap(),
                root: directory.path().into(),
                input_paths: vec![QueryPath::Glob {
                    pattern: "*.ts".into(),
                }],
                change_scope: ChangeScope::Full {},
            },
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let query = SourceUnitQuery {
        seeds: SourceUnitSeeds::SelectorUnits {
            selectors: vec![EvidenceSelector::LiteralText {
                repository_id: RepositoryId::new("plain").unwrap(),
                paths: vec![QueryPath::Exact {
                    path: "source.ts".into(),
                }],
                text: "chosen".into(),
            }],
        },
        languages: vec![SourceLanguage::Typescript],
        unit_kinds: vec![UnitKindFilter::Function],
        source_sides: SourceSides::AfterOnly,
    };
    let captures = [captured];
    let units = select_source_units(&captures, &query, 10).unwrap().units;
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].text, "function chosen() {}");
    let changed = SourceUnitQuery {
        seeds: SourceUnitSeeds::ChangedUnits {
            repository_ids: vec![RepositoryId::new("plain").unwrap()],
        },
        ..query
    };
    assert_eq!(
        select_source_units(&captures, &changed, 10)
            .unwrap_err()
            .code,
        EvidenceCode::NoChangeSelection
    );
}
