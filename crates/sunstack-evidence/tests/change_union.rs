#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn full_union_member_is_context_and_never_fabricates_changed_units() {
    let fixture = GitFixture::new();
    fixture.write(
        "source.ts",
        "function unchanged() {}\nfunction changed() { old(); }",
    );
    fixture.stage("source.ts");
    fixture.commit(1000);
    fixture.write(
        "source.ts",
        "function unchanged() {}\nfunction changed() { newer(); }",
    );
    let scope = ChangeScope::Union {
        members: vec![
            ChangeScope::Full {},
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        ],
    };
    let captured = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(scope),
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(captured.members.len(), 2);
    assert!(captured.members[0].changes.is_empty());
    let units = changed_source_units(
        &captured,
        &[SourceLanguage::Typescript],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].text, "function changed() { newer(); }");
    let full_only = ChangeScope::Union {
        members: vec![ChangeScope::Full {}],
    };
    assert_eq!(
        capture_change_selection(
            ChangeCaptureRequest {
                binding: fixture.binding(full_only),
                limits: discovery_limits(),
                anchor_utc: chrono::Utc::now()
            },
            CancellationToken::new()
        )
        .await
        .unwrap_err()
        .code,
        EvidenceCode::NoChangeSelection
    );
}

#[tokio::test]
async fn full_union_context_does_not_widen_change_set_text_or_ast_queries() {
    let fixture = GitFixture::new();
    fixture.write("unchanged.ts", "function marker() {}\n");
    fixture.write("changed.ts", "function original() {}\n");
    fixture.stage("unchanged.ts");
    fixture.stage("changed.ts");
    fixture.commit(1000);
    fixture.write("changed.ts", "function marker() {}\n");
    let scope = ChangeScope::Union {
        members: vec![
            ChangeScope::Full {},
            ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            },
        ],
    };
    let paths = vec![QueryPath::Glob {
        pattern: "*.ts".into(),
    }];
    for select in [
        EvidenceSelection::ChangeSetLiteralText {
            paths: paths.clone(),
            text: "marker".into(),
        },
        EvidenceSelection::ChangeSetRegexText {
            paths: paths.clone(),
            pattern: "marker".into(),
        },
        EvidenceSelection::ChangeSetAstPattern {
            paths: paths.clone(),
            language: "typescript".into(),
            pattern: "function marker() {}".into(),
        },
    ] {
        let outcome = prepare_evidence(
            support::connected_inputs::plan(
                vec![fixture.binding(scope.clone())],
                select,
                EvidenceRequirement::Required,
            ),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        let EvidencePlanOutcome::Prepared { selections, .. } = outcome else {
            panic!("query should be prepared: {outcome:?}")
        };
        assert!(
            matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::Matches { matches }, .. } if matches.len() == 1),
            "Full contributes context, not unchanged query matches: {selections:?}"
        );
    }
}
