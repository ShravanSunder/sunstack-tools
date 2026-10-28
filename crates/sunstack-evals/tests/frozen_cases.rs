#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use support::*;

#[test]
fn same_size_valid_label_change_refuses_before_scoring() {
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let manifest = root.path().join("cases/manifest.json");
    let labels = root.path().join("cases/labels.jsonl");
    let original = std::fs::read(&labels).unwrap();
    // yes -> no is padded as valid JSON: equal length; neither metadata nor parse failure decides this refusal.
    let changed = String::from_utf8(original.clone())
        .unwrap()
        .replace("\"label\":\"yes\"", "\"label\":\"no\" ");
    assert_eq!(changed.len(), original.len());
    assert_ne!(changed.as_bytes(), original);
    let _: LabeledCase = serde_json::from_str(changed.trim()).unwrap();
    std::fs::write(&labels, changed).unwrap();
    assert!(matches!(
        load_case_set(&manifest),
        Err(EvalError::Integrity { .. })
    ));
    // The old in-memory snapshot still represents the original content version.
    assert_eq!(score(&[], &set).unwrap().case_set, set.identity().clone());
    std::fs::write(&labels, original).unwrap();
    let restored = load_case_set(&manifest).unwrap();
    assert_eq!(restored.identity(), set.identity());
    assert!(score(&[], &restored).is_ok());
}

#[test]
fn sources_definitions_and_label_blind_sources_are_frozen_separately() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Unknown, Split::HeldOut)]);
    specification.sources[0].evidence.insert(
        sunstack_evidence::SourceName::new("owner").unwrap(),
        sunstack_rules::EvidenceBinding::Unavailable {
            reason: sunstack_rules::UnavailableReason::Withheld,
        },
    );
    let set = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let sources = std::fs::read_to_string(root.path().join("cases/sources.jsonl")).unwrap();
    let source: CaseSource = serde_json::from_str(sources.trim()).unwrap();
    assert_eq!(
        serde_json::to_value(&source.evidence).unwrap(),
        json!({"owner":{"kind":"unavailable","reason":"withheld"}})
    );
    assert!(!sources.contains("label"));
    assert_eq!(set.sources().len(), 1);
    for path in [
        root.path().join("transcript.jsonl"),
        root.path().join("cases/definitions/rule.jsonl"),
        root.path().join("cases/sources.jsonl"),
    ] {
        let original = std::fs::read(&path).unwrap();
        let mut changed = original.clone();
        changed[0] ^= 1;
        std::fs::write(&path, changed).unwrap();
        assert!(matches!(
            load_case_set(&root.path().join("cases/manifest.json")),
            Err(EvalError::Integrity { .. })
        ));
        std::fs::write(path, original).unwrap();
    }
    assert!(freeze_cases(&root.path().join("cases"), &specification).is_err());
}

#[test]
fn sessions_and_projects_cannot_cross_splits() {
    let root = tempfile::tempdir().unwrap();
    let one = case(1, Label::Yes, Split::Development);
    let mut two = case(2, Label::No, Split::HeldOut);
    let mut specification = input(root.path(), vec![one.clone(), two.clone()]);
    assert!(matches!(
        freeze_cases(&root.path().join("same-session"), &specification),
        Err(EvalError::SplitLeakage { .. })
    ));
    two.source = CaseSourceId::new("source_2").unwrap();
    let mut second_source = specification.sources[0].clone();
    second_source.source_id = two.source.clone();
    second_source.source = serde_json::from_value(
        json!({"kind":"session_pair","provider":"codex","session":"different_session"}),
    )
    .unwrap();
    specification.sources.push(second_source);
    specification.labels = vec![one, two];
    assert!(matches!(
        freeze_cases(&root.path().join("same-project"), &specification),
        Err(EvalError::SplitLeakage { .. })
    ));
    specification.labels[1].project = Some(ProjectKey::new("project_2").unwrap());
    assert!(freeze_cases(&root.path().join("disjoint"), &specification).is_ok());
}

#[test]
fn duplicate_or_unbound_cases_and_invalid_identities_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let one = case(1, Label::Yes, Split::HeldOut);
    let mut specification = input(root.path(), vec![one.clone(), one]);
    assert!(freeze_cases(&root.path().join("duplicate"), &specification).is_err());
    specification.labels.truncate(1);
    specification.labels[0].source = CaseSourceId::new("not_frozen").unwrap();
    assert!(freeze_cases(&root.path().join("unbound"), &specification).is_err());
    assert!(CaseId::new("").is_err());
    assert!(serde_json::from_str::<CaseId>("\"\"").is_err());
}

#[test]
fn frozen_manifest_and_jsonl_decoders_refuse_duplicate_and_unknown_fields() {
    let root = tempfile::tempdir().unwrap();
    let _set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let manifest_path = root.path().join("cases/manifest.json");
    let original = std::fs::read(&manifest_path).unwrap();
    let mut text = String::from_utf8(original.clone()).unwrap();
    text.insert_str(1, "\"split\":\"held_out\",");
    std::fs::write(&manifest_path, text).unwrap();
    assert!(matches!(
        load_case_set(&manifest_path),
        Err(EvalError::Invalid { .. })
    ));
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["unexpected"] = json!(true);
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(matches!(
        load_case_set(&manifest_path),
        Err(EvalError::Invalid { .. })
    ));
    let labels_path = root.path().join("cases/labels.jsonl");
    let label_text = std::fs::read_to_string(&labels_path).unwrap();
    let label_text = label_text.replacen('{', "{\"label\":\"no\",", 1);
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["labels"]["content_identity"] =
        json!(sunstack_evidence::content_digest(label_text.as_bytes()));
    std::fs::write(&labels_path, label_text).unwrap();
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(matches!(
        load_case_set(&manifest_path),
        Err(EvalError::Invalid { .. })
    ));
}
