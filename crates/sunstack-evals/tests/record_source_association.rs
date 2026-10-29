#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use std::path::{Path, PathBuf};
use sunstack_evals::*;
use sunstack_evidence::*;
use sunstack_rules::QuestionRef;

struct NativeFixture {
    root: tempfile::TempDir,
    specification: FreezeInput,
}
fn native_anchor(source: &SourceInput, bytes: &[u8]) -> SourceOccurrence {
    let identity = source.source_identity().unwrap();
    let registry = AdapterRegistry::builtins().unwrap();
    let (registry, format) = match source {
        SourceInput::CustomEntry { schema, .. } => registry.with_custom_schema(schema).unwrap(),
        SourceInput::SessionPair { .. } => (
            registry,
            TranscriptFormatName::new("claude_code_jsonl").unwrap(),
        ),
        _ => unreachable!(),
    };
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes,
            source_identity: identity.as_str(),
        });
    let positioned = project_positioned(&identity, &lines).unwrap();
    assert_eq!(positioned.records.len(), 1);
    let discovered = positioned.records.as_slice().discover(None, 0).unwrap();
    assert_eq!(discovered.matches.len(), 1);
    discovered.matches[0].occurrence.clone()
}
fn fixture(custom: bool) -> NativeFixture {
    let root = tempfile::tempdir().unwrap();
    let mut specification = support::input(root.path(), vec![]);
    let mut source_files = Vec::<PathBuf>::new();
    let mut sources = Vec::new();
    let mut labels = Vec::new();
    for (position, name) in ["source_a", "source_b"].into_iter().enumerate() {
        let path = root.path().join(format!("{name}.jsonl"));
        let (source, text) = if custom {
            let schema=serde_json::from_value(json!({"format":"jsonl","record_schema":{"type":"object","required":["text"],"properties":{"text":{"type":"string"}}},"fields":{"text":"/text"}})).unwrap();
            (
                SourceInput::CustomEntry {
                    location: path.clone(),
                    schema,
                },
                json!({"text":"MATCH identical public occurrence"}).to_string() + "\n",
            )
        } else {
            (
                SourceInput::SessionPair {
                    provider: ProviderName::new("claude_code").unwrap(),
                    session: SessionId::new(name).unwrap(),
                },
                json!({"type":"assistant","message":{"id":"identical_message","content":"MATCH identical public occurrence"}})
                    .to_string()
                    + "\n",
            )
        };
        std::fs::write(&path, &text).unwrap();
        let anchor = native_anchor(&source, &std::fs::read(&path).unwrap());
        let source_id = CaseSourceId::new(name).unwrap();
        labels.push(LabeledCase {
            case_id: CaseId::new(format!("case_{position}")).unwrap(),
            source: source_id.clone(),
            project: None,
            source_anchor: anchor,
            question: QuestionRef::Verdict {},
            label: Label::Yes,
            split: Split::HeldOut,
        });
        sources.push(CaseSource {
            source_id,
            source,
            evidence: Default::default(),
        });
        source_files.push(path);
    }
    specification.sources = sources;
    specification.labels = labels;
    specification.source_files = source_files;
    NativeFixture {
        root,
        specification,
    }
}
fn assert_association_refused(result: Result<FrozenCaseSet, EvalError>) {
    assert!(
        matches!(result,Err(EvalError::Invalid {field}) if field=="labels.source_anchor.record")
    );
}
fn replace_labels(manifest_path: &Path, labels: &[LabeledCase]) {
    let mut manifest: CaseSetManifest =
        serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
    let bytes = labels
        .iter()
        .map(|label| serde_json::to_string(label).unwrap() + "\n")
        .collect::<String>()
        .into_bytes();
    std::fs::write(&manifest.labels.path, &bytes).unwrap();
    manifest.labels.content_identity = ContentIdentity::new(content_digest(&bytes)).unwrap();
    std::fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}
#[test]
fn native_session_and_custom_anchors_freeze_load_and_keep_genuine_groups_separate() {
    for custom in [false, true] {
        let mut fixture = fixture(custom);
        fixture.specification.labels[0].split = Split::Development;
        let path = fixture.root.path().join("valid");
        let frozen = freeze_cases(&path, &fixture.specification).unwrap();
        let loaded = load_case_set(&path.join("manifest.json")).unwrap();
        assert_eq!(loaded.identity(), frozen.identity());
        assert_eq!(loaded.labels().len(), 2);
        assert_eq!(
            loaded.labels()[0]
                .source_anchor
                .occurrence_identity()
                .unwrap(),
            fixture.specification.labels[0]
                .source_anchor
                .occurrence_identity()
                .unwrap()
        );
    }
}
fn rejects_foreign_record_freeze(custom: bool) {
    let mut fixture = fixture(custom);
    fixture.specification.labels.truncate(1);
    fixture.specification.labels[0].source = fixture.specification.sources[1].source_id.clone();
    assert_association_refused(freeze_cases(
        &fixture.root.path().join("foreign"),
        &fixture.specification,
    ));
}
fn rejects_foreign_record_load(custom: bool) {
    let mut fixture = fixture(custom);
    fixture.specification.labels.truncate(1);
    let directory = fixture.root.path().join("valid");
    freeze_cases(&directory, &fixture.specification).unwrap();
    fixture.specification.labels[0].source = fixture.specification.sources[1].source_id.clone();
    replace_labels(
        &directory.join("manifest.json"),
        &fixture.specification.labels,
    );
    assert_association_refused(load_case_set(&directory.join("manifest.json")));
}
fn rejects_record_split_redirection(custom: bool) {
    let mut fixture = fixture(custom);
    fixture.specification.labels[0].split = Split::Development;
    fixture.specification.labels[1].source_anchor =
        fixture.specification.labels[0].source_anchor.clone();
    assert!(
        fixture
            .specification
            .labels
            .iter()
            .all(|label| label.project.is_none())
    );
    assert_association_refused(freeze_cases(
        &fixture.root.path().join("cross_split"),
        &fixture.specification,
    ));
}
#[test]
fn session_foreign_record_anchor_refuses_freezing() {
    rejects_foreign_record_freeze(false);
}
#[test]
fn custom_foreign_record_anchor_refuses_freezing() {
    rejects_foreign_record_freeze(true);
}
#[test]
fn session_integrity_valid_foreign_record_refuses_loading() {
    rejects_foreign_record_load(false);
}
#[test]
fn custom_integrity_valid_foreign_record_refuses_loading() {
    rejects_foreign_record_load(true);
}
#[test]
fn session_record_cannot_redirect_into_another_split_without_shared_project() {
    rejects_record_split_redirection(false);
}
#[test]
fn custom_record_cannot_redirect_into_another_split_without_shared_project() {
    rejects_record_split_redirection(true);
}
