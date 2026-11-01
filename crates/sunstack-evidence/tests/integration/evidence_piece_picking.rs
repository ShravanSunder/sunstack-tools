#![allow(clippy::unwrap_used)]
#[path = "../support/mod.rs"]
mod support;
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;
fn source_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sunstack/tests/fixtures/rulebook-lint")
}
fn build(root: &std::path::Path, files: &[PathBuf]) -> EvidenceCatalog {
    let source = source_fixture();
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(source.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> =
        serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let project = load_evidence_project(&source.join("evidence/sgconfig.yml")).unwrap();
    build_pieces(
        &project,
        &kinds,
        files,
        &mut FilesystemCheckedSource::new(root).unwrap(),
        PieceBuildLimits {
            max_files: 2000,
            max_file_bytes: 100000,
            max_total_bytes: 1000000,
            max_matches: 10000,
            evidence_max_chars: 6000,
        },
    )
}
async fn changes(fixture: &GitFixture) -> Vec<CapturedChange> {
    capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![
                    DirtyLayer::Staged,
                    DirtyLayer::Unstaged,
                    DirtyLayer::Untracked,
                ],
            }),
            limits: discovery_limits(),
            anchor_utc: chrono::DateTime::from_timestamp(2000, 0).unwrap(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
    .members
    .into_iter()
    .flat_map(|member| member.changes)
    .collect()
}
#[tokio::test]
async fn real_git_hunk_picks_only_step_three_and_a_changed_target_picks_its_load() {
    let fixture = GitFixture::new();
    let source = source_fixture();
    let skill = "skills/verification-design/SKILL.md";
    let reference = "skills/verification-design/references/choosing-the-observation.md";
    let original = std::fs::read_to_string(source.join(skill)).unwrap();
    fixture.write(skill, &original);
    fixture.write(
        reference,
        &std::fs::read_to_string(source.join(reference)).unwrap(),
    );
    fixture.stage(skill);
    fixture.stage(reference);
    fixture.commit(1000);
    let changed = original
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 54 {
                format!("{line} Additional fixture obligation.")
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fixture.write(skill, &changed);
    let catalog = build(fixture.directory.path(), &[skill.into()]);
    let steps = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "step")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(steps.len(), 6);
    let (picked, outside) = pick_pieces(
        &steps,
        &changes(&fixture).await,
        false,
        fixture.directory.path(),
    );
    assert_eq!(outside, 5);
    assert_eq!(picked.len(), 1);
    assert_eq!(steps[picked[0]].location.first_line, 49);
    fixture.write(skill, &original);
    fixture.write(reference, "# Changed reference opening\n");
    let catalog = build(fixture.directory.path(), &[skill.into()]);
    let loads = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "load")
        .cloned()
        .collect::<Vec<_>>();
    let (picked, _) = pick_pieces(
        &loads,
        &changes(&fixture).await,
        false,
        fixture.directory.path(),
    );
    assert!(
        picked
            .iter()
            .any(|index| loads[*index].location.first_line == 51)
    );
    assert!(picked.iter().all(|index|loads[*index].fields.values().any(|field|matches!(field,FieldValue::Targets {values} if values.iter().any(|target|target.path==std::path::Path::new(reference))))));
}
#[tokio::test]
async fn an_untracked_skill_is_wholly_changed() {
    let fixture = GitFixture::new();
    fixture.write("base.md", "Baseline\n");
    fixture.stage("base.md");
    fixture.commit(1000);
    let skill = "skills/new/SKILL.md";
    fixture.write(
        skill,
        &std::fs::read_to_string(source_fixture().join("skills/verification-design/SKILL.md"))
            .unwrap(),
    );
    let catalog = build(fixture.directory.path(), &[skill.into()]);
    let steps = catalog
        .pieces
        .into_iter()
        .filter(|piece| piece.kind.as_str() == "step")
        .collect::<Vec<_>>();
    let (picked, outside) = pick_pieces(
        &steps,
        &changes(&fixture).await,
        false,
        fixture.directory.path(),
    );
    assert_eq!(picked.len(), 6);
    assert_eq!(outside, 0);
}
