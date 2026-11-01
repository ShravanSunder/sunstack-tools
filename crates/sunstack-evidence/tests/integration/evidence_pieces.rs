#![allow(clippy::unwrap_used, clippy::panic)]
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use sunstack_evidence::*;
#[path = "evidence_pieces/nudge_context.rs"]
mod nudge_context;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sunstack/tests/fixtures/rulebook-lint")
}
fn settings() -> EvidenceSettings {
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture().join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(config["evidence"].clone()).unwrap()
}
fn limits() -> PieceBuildLimits {
    PieceBuildLimits {
        max_files: 2000,
        max_file_bytes: 100_000,
        max_total_bytes: 1_000_000,
        max_matches: 10_000,
        evidence_max_chars: 6000,
    }
}
fn field<'a>(piece: &'a EvidencePiece, name: &str) -> &'a FieldValue {
    &piece.fields[&FieldName::new(name).unwrap()]
}

struct RecordingSource {
    source: FilesystemCheckedSource,
    reads: Vec<PathBuf>,
    fail_path: Option<PathBuf>,
}
impl RecordingSource {
    fn new(root: &Path) -> Self {
        Self {
            source: FilesystemCheckedSource::new(root).unwrap(),
            reads: vec![],
            fail_path: None,
        }
    }
}
impl CheckedFileSource for RecordingSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf> {
        self.source.resolve_file(path)
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        self.reads.push(path.into());
        if self.fail_path.as_deref() == Some(path) {
            return Err(CheckedReadError::Unreadable);
        }
        self.source.read_file(path, max_bytes)
    }
    fn checked_files_acquired(&self) -> usize {
        self.source.checked_files_acquired()
    }
}

#[test]
fn resolved_target_acquisition_failure_is_unavailable_rather_than_a_missing_file() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/load.yml"),
        "id: load\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("SKILL.md"), "load `target.md`.\n").unwrap();
    std::fs::write(root.path().join("target.md"), "An existing target.\n").unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind:EvidenceKindDefinition=serde_json::from_str(r#"{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path"}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("load").unwrap(), kind)]);
    let mut source = RecordingSource::new(root.path());
    source.fail_path = Some("target.md".into());
    assert_eq!(
        source.resolve_file(Path::new("target.md")),
        Some("target.md".into())
    );
    let catalog = build_pieces(
        &project,
        &kinds,
        &["SKILL.md".into()],
        &mut source,
        limits(),
    );
    assert_eq!(
        field(&catalog.pieces[0], "targets"),
        &FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable
        }
    );
    assert_eq!(
        source.reads,
        vec![PathBuf::from("SKILL.md"), PathBuf::from("target.md")]
    );
}

#[test]
fn frozen_checked_and_target_content_share_one_distinct_byte_budget() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/load.yml"),
        "id: load\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let checked = "load `target.md`, `./target.md` and `SKILL.md`.\n";
    std::fs::write(root.path().join("SKILL.md"), checked).unwrap();
    // A fallback read cannot pass the per-file cap; the frozen target must supply the opening.
    std::fs::write(root.path().join("target.md"), "stale".repeat(1000)).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind: EvidenceKindDefinition = serde_json::from_str(r#"{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":10}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("load").unwrap(), kind)]);
    for (cap, complete) in [(400, false), (checked.len() + 400, true)] {
        let mut source = FrozenCheckedSource::new(
            BTreeMap::from([
                (PathBuf::from("SKILL.md"), checked.into()),
                (PathBuf::from("target.md"), "x".repeat(400)),
            ]),
            Box::new(RecordingSource::new(root.path())),
        );
        let catalog = build_pieces(
            &project,
            &kinds,
            &["SKILL.md".into()],
            &mut source,
            PieceBuildLimits {
                max_file_bytes: 500,
                max_total_bytes: cap,
                ..limits()
            },
        );
        if complete {
            assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
            let FieldValue::Targets { values } = field(&catalog.pieces[0], "targets") else {
                panic!()
            };
            assert_eq!(values.len(), 3);
            assert_eq!(values[0].opening.as_deref(), Some("xxxxxxxxxx"));
            assert_eq!(values[0].opening, values[1].opening);
            assert_eq!(values[2].opening.as_deref(), Some("load `targ"));
        } else {
            assert!(
                catalog
                    .failures
                    .iter()
                    .any(|failure| failure.reason == "max_total_bytes")
            );
        }
        assert_eq!(source.checked_files_acquired(), 2);
    }
}

#[test]
fn frozen_target_file_count_reuses_cached_alias_and_self_content() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/load.yml"),
        "id: load\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let checked = "load `SKILL.md`, `target.md`, `./target.md` and `target.md`.\n";
    std::fs::write(root.path().join("SKILL.md"), checked).unwrap();
    std::fs::write(root.path().join("target.md"), "stale".repeat(1000)).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind: EvidenceKindDefinition = serde_json::from_str(r#"{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":10}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("load").unwrap(), kind)]);
    for max_files in [1, 2] {
        let mut source = FrozenCheckedSource::new(
            BTreeMap::from([
                (PathBuf::from("SKILL.md"), checked.into()),
                (PathBuf::from("target.md"), "x".repeat(400)),
            ]),
            Box::new(RecordingSource::new(root.path())),
        );
        let catalog = build_pieces(
            &project,
            &kinds,
            &["SKILL.md".into()],
            &mut source,
            PieceBuildLimits {
                max_files,
                max_file_bytes: 500,
                ..limits()
            },
        );
        assert_eq!(source.checked_files_acquired(), max_files);
        if max_files == 1 {
            assert_eq!(catalog.failures.len(), 1);
            assert_eq!(catalog.failures[0].reason, "max_files");
        } else {
            assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
            let FieldValue::Targets { values } = field(&catalog.pieces[0], "targets") else {
                panic!()
            };
            assert_eq!(values.len(), 4);
            assert!(values.iter().all(|value| value.exists));
            for value in &values[1..] {
                assert_eq!(value.opening.as_deref(), Some("xxxxxxxxxx"));
            }
        }
    }
}

#[test]
fn pinned_step_fields_and_path_and_skill_loads_keep_their_sources() {
    let root = fixture();
    let settings = settings();
    let project = load_evidence_project(&root.join(&settings.ast_grep)).unwrap();
    let files = [
        "verification-design",
        "practices-retrospective",
        "practices-research",
    ]
    .map(|name| PathBuf::from(format!("skills/{name}/SKILL.md")));
    let mut source = RecordingSource::new(&root);
    let catalog = build_pieces(&project, &settings.kinds, &files, &mut source, limits());
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let step = catalog
        .pieces
        .iter()
        .find(|p| {
            p.kind.as_str() == "step" && p.location.file == files[0] && p.location.first_line == 49
        })
        .unwrap();
    assert_eq!(
        (step.location.span.byte_start, step.location.span.byte_end),
        (6114, 7909)
    );
    assert_eq!(step.location.last_line, 60);
    assert!(
        matches!(field(step,"heading"), FieldValue::Text { value, .. } if value == "3. Choose each observation")
    );
    assert!(
        matches!(field(step,"completion"), FieldValue::Texts { values, spans } if values.len() == 1 && spans[0].start_line == 59)
    );
    if let FieldValue::Texts { values, spans } = field(step, "completion") {
        let source_text = std::fs::read_to_string(root.join(&files[0])).unwrap();
        assert_eq!(
            values[0],
            source_text[spans[0].byte_start..spans[0].byte_end]
        );
    }
    assert!(
        matches!(field(step,"body"), FieldValue::Text { value, .. } if value.trim_start().starts_with("MUST load") && !value.contains("Completion:") && value.contains("When no Program Design seam"))
    );
    let load = catalog
        .pieces
        .iter()
        .find(|p| {
            p.kind.as_str() == "load" && p.location.file == files[0] && p.location.first_line == 51
        })
        .unwrap();
    assert!(
        matches!(field(load,"targets"), FieldValue::Targets { values } if values[0].exists && values[0].path == Path::new("skills/verification-design/references/choosing-the-observation.md") && values[0].opening.as_ref().unwrap().starts_with('#'))
    );
    let skill_load = catalog
        .pieces
        .iter()
        .find(|p| {
            p.kind.as_str() == "load" && p.location.file == files[1] && p.location.first_line == 21
        })
        .unwrap();
    assert!(
        matches!(field(skill_load,"targets"), FieldValue::Targets { values } if values.iter().any(|target| target.name == "practices-research" && target.exists && target.fields.as_ref().unwrap().contains_key(&FieldName::new("description").unwrap())))
    );
    if let FieldValue::Targets { values } = field(skill_load, "targets") {
        let target = values
            .iter()
            .find(|target| target.name == "practices-research")
            .unwrap();
        assert!(target.opening.is_none());
        assert!(target.opening_last_line.is_none());
        assert!(!target.opening_cut);
    }
    let skill = catalog
        .pieces
        .iter()
        .find(|p| p.kind.as_str() == "skill" && p.location.file == files[0])
        .unwrap();
    assert!(
        matches!(field(skill,"description"), FieldValue::Text { value, spans } if value.chars().count() == 450 && spans[0].start_line == 3)
    );
    for file in &files {
        assert_eq!(source.reads.iter().filter(|path| *path == file).count(), 1);
    }
}

#[test]
fn yaml_empty_absent_malformed_and_non_string_remain_distinct() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    // Whole-document match keeps malformed frontmatter available to the YAML decoder.
    std::fs::write(
        root.path().join("rules/skill.yml"),
        "id: skill\nlanguage: Markdown\nrule: {kind: document}\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind: EvidenceKindDefinition = serde_json::from_str(
        r#"{"rule":"skill","fields":{"description":{"from":"yaml","key":"description"}}}"#,
    )
    .unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("skill").unwrap(), kind)]);
    for (input, state) in [
        ("---\ndescription: \"\"\n---\n", "empty"),
        ("---\nother: x\n---\n", "absent"),
        ("---\ndescription: [\n---\n", "malformed"),
        ("---\ndescription: {key: value}\n---\n", "non_string"),
        ("---\ndescription: >\n  folded\n  value\n---\n", "folded"),
    ] {
        std::fs::write(root.path().join("SKILL.md"), input).unwrap();
        let catalog = build_pieces(
            &project,
            &kinds,
            &["SKILL.md".into()],
            &mut RecordingSource::new(root.path()),
            limits(),
        );
        let value = field(&catalog.pieces[0], "description");
        match state {
            "empty" => assert!(
                matches!(value, FieldValue::Text { value, spans } if value.is_empty() && spans[0].start_line == 2)
            ),
            "absent" => assert_eq!(*value, FieldValue::Absent),
            "malformed" => assert_eq!(
                *value,
                FieldValue::Unavailable {
                    reason: FieldUnavailable::FrontmatterMalformed
                }
            ),
            "non_string" => assert_eq!(
                *value,
                FieldValue::Unavailable {
                    reason: FieldUnavailable::NotAString
                }
            ),
            "folded" => assert!(
                matches!(value, FieldValue::Text { value, .. } if value == "folded value\n")
            ),
            _ => unreachable!(),
        }
    }
}

#[test]
fn duplicate_skills_and_outside_targets_are_not_opened() {
    let workspace = tempfile::tempdir().unwrap();
    let root_path = workspace.path().join("repository");
    std::fs::create_dir(&root_path).unwrap();
    let root = root_path.as_path();
    std::fs::write(
        workspace.path().join("outside.md"),
        "Outside the allowed repository.\n",
    )
    .unwrap();
    std::fs::create_dir(root.join("rules")).unwrap();
    std::fs::write(root.join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.join("rules/paragraph.yml"),
        "id: load\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    for directory in ["a/shared", "b/shared"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
        std::fs::write(root.join(directory).join("SKILL.md"), "Shared skill.\n").unwrap();
    }
    std::fs::write(
        root.join("SKILL.md"),
        "load `shared` and `../outside.md`.\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.join("sgconfig.yml")).unwrap();
    let kind: EvidenceKindDefinition = serde_json::from_str(r#"{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path_or_skill"}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("load").unwrap(), kind)]);
    let mut source = RecordingSource::new(root);
    let catalog = build_pieces(
        &project,
        &kinds,
        &[
            "SKILL.md".into(),
            "a/shared/SKILL.md".into(),
            "b/shared/SKILL.md".into(),
        ],
        &mut source,
        limits(),
    );
    let piece = catalog
        .pieces
        .iter()
        .find(|p| p.location.file == Path::new("SKILL.md"))
        .unwrap();
    let FieldValue::Targets { values } = field(piece, "targets") else {
        panic!()
    };
    assert!(!values[0].exists);
    assert_eq!(
        values[0].candidates,
        [
            PathBuf::from("a/shared/SKILL.md"),
            PathBuf::from("b/shared/SKILL.md")
        ]
    );
    assert!(!values[1].exists);
    assert_eq!(source.reads.len(), 3);
}

#[test]
fn non_ascii_terminal_character_without_newline_keeps_valid_ranges_and_character_columns() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/paragraph.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind:EvidenceKindDefinition=serde_json::from_str(r#"{"rule":"paragraph","fields":{"token":{"from":"capture","regex":"(token)"},"body":{"from":"remainder"}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("paragraph").unwrap(), kind)]);
    std::fs::write(root.path().join("SKILL.md"), "📚 token尾").unwrap();
    let catalog = build_pieces(
        &project,
        &kinds,
        &["SKILL.md".into()],
        &mut RecordingSource::new(root.path()),
        limits(),
    );
    let piece = &catalog.pieces[0];
    assert_eq!(piece.location.last_line, 1);
    assert_eq!(piece.location.span.end_column, 9);
    let span = field(piece, "token").spans().first().unwrap();
    assert_eq!(span.start_column, 3);
    assert_eq!(span.end_column, 8);
}
