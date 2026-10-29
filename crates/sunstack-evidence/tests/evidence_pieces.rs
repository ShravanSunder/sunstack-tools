#![allow(clippy::unwrap_used, clippy::panic)]
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use sunstack_evidence::*;

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
}
impl RecordingSource {
    fn new(root: &Path) -> Self {
        Self {
            source: FilesystemCheckedSource::new(root).unwrap(),
            reads: vec![],
        }
    }
}
impl CheckedFileSource for RecordingSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf> {
        self.source.resolve_file(path)
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        self.reads.push(path.into());
        self.source.read_file(path, max_bytes)
    }
    fn checked_files_read(&self) -> usize {
        self.reads.len()
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
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/paragraph.yml"),
        "id: load\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    for directory in ["a/shared", "b/shared"] {
        std::fs::create_dir_all(root.path().join(directory)).unwrap();
        std::fs::write(
            root.path().join(directory).join("SKILL.md"),
            "Shared skill.\n",
        )
        .unwrap();
    }
    std::fs::write(
        root.path().join("SKILL.md"),
        "load `shared` and `../outside.md`.\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kind: EvidenceKindDefinition = serde_json::from_str(r#"{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path_or_skill"}}}"#).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("load").unwrap(), kind)]);
    let mut source = RecordingSource::new(root.path());
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
    assert_eq!(values[0].candidates.len(), 2);
    assert!(!values[1].exists);
    assert_eq!(source.reads.len(), 3);
}
