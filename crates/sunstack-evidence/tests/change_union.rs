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
