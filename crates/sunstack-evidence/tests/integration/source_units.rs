#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/mod.rs"]
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
