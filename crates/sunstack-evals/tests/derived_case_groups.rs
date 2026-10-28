#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use sunstack_evidence::*;
use support::{case, input};

fn code(repository: &str) -> SourceInput {
    SourceInput::Code {
        repository: RepositoryId::new(repository).unwrap(),
        revision: RevisionHead::Commit {
            revision: "b".repeat(40),
        },
        paths: vec![QueryPath::Glob {
            pattern: "**/*.md".into(),
        }],
    }
}
fn diff(repository: &str, base: &str) -> SourceInput {
    SourceInput::Diff {
        repository: RepositoryId::new(repository).unwrap(),
        scope: ChangeScope::RevisionDiff {
            base_revision: base.repeat(40),
            head: RevisionHead::Commit {
                revision: "b".repeat(40),
            },
        },
    }
}
fn source(id: &str, source: SourceInput) -> CaseSource {
    CaseSource {
        source_id: CaseSourceId::new(id).unwrap(),
        source,
        evidence: Default::default(),
    }
}
fn anchor(
    source: &CaseSource,
    repository: &str,
    path: &str,
    position: usize,
    context: bool,
) -> SourceOccurrence {
    let revision = match &source.source {
        SourceInput::Code {
            revision: RevisionHead::Commit { revision },
            ..
        }
        | SourceInput::Diff {
            scope:
                ChangeScope::RevisionDiff {
                    head: RevisionHead::Commit { revision },
                    ..
                },
            ..
        } => Some(revision),
        _ => None,
    }
    .unwrap();
    let span = SourceSpan {
        source: SourceRef::RepositoryFile {
            repository: RepositoryId::new(repository).unwrap(),
            revision: SourceRevision::Commit {
                commit_id: GitObjectId::new(revision.clone()).unwrap(),
                side: SourceSide::After,
            },
            path: path.into(),
            content_identity: "captured document".into(),
        },
        byte_start: position,
        byte_end: position + 4,
        start_line: position + 1,
        start_column: 1,
        end_line: position + 1,
        end_column: 5,
    };
    let input_id = source.source.source_identity().unwrap();
    if context {
        SourceOccurrence::ContextMatch {
            input_id,
            part: ContextPart::TouchedUnit,
            unit: None,
            span,
            hunk: None,
        }
    } else {
        SourceOccurrence::CodeMatch {
            input_id,
            unit: None,
            span,
        }
    }
}
fn label(
    number: usize,
    source: &CaseSource,
    anchor: SourceOccurrence,
    split: Split,
) -> LabeledCase {
    let mut label = case(number, Label::Yes, split);
    label.source = source.source_id.clone();
    label.source_anchor = anchor;
    label.project = None;
    label
}
fn freeze_labels(
    sources: Vec<CaseSource>,
    labels: Vec<LabeledCase>,
) -> Result<FrozenCaseSet, EvalError> {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), labels);
    specification.sources = sources;
    freeze_cases(&root.path().join("cases"), &specification)
}
#[test]
fn same_document_anchors_cannot_cross_splits_even_across_code_input_ids() {
    let first = source("one", code("repository"));
    let mut second = source("two", code("repository"));
    if let SourceInput::Code { revision, .. } = &mut second.source {
        *revision = RevisionHead::Commit {
            revision: "c".repeat(40),
        };
    }
    let development = label(
        1,
        &first,
        anchor(&first, "repository", "skills/guide.md", 0, false),
        Split::Development,
    );
    let held_out = label(
        2,
        &second,
        anchor(&second, "repository", "skills/guide.md", 20, true),
        Split::HeldOut,
    );
    assert!(
        matches!(freeze_labels(vec![first,second],vec![development,held_out]),Err(EvalError::SplitLeakage {field}) if field=="group")
    );
}
#[test]
fn separate_documents_or_repositories_are_legitimate_groups() {
    for (second_repository, second_path) in [
        ("repository", "skills/other.md"),
        ("other_repository", "skills/guide.md"),
    ] {
        let first = source("one", code("repository"));
        let second = source("two", code(second_repository));
        let development = label(
            1,
            &first,
            anchor(&first, "repository", "skills/guide.md", 0, false),
            Split::Development,
        );
        let held_out = label(
            2,
            &second,
            anchor(&second, second_repository, second_path, 20, true),
            Split::HeldOut,
        );
        assert!(freeze_labels(vec![first, second], vec![development, held_out]).is_ok());
    }
}
#[test]
fn derived_code_groups_still_protect_projects() {
    let first = source("one", code("repository"));
    let second = source("two", code("repository"));
    let mut development = label(
        1,
        &first,
        anchor(&first, "repository", "skills/one.md", 0, false),
        Split::Development,
    );
    let mut held_out = label(
        2,
        &second,
        anchor(&second, "repository", "skills/two.md", 20, true),
        Split::HeldOut,
    );
    development.project = Some(ProjectKey::new("shared_project").unwrap());
    held_out.project = development.project.clone();
    assert!(
        matches!(freeze_labels(vec![first,second],vec![development,held_out]),Err(EvalError::SplitLeakage {field}) if field=="project")
    );
}
#[test]
fn diff_group_owns_the_revision_scope_rather_than_the_document_path() {
    let first = source("one", diff("repository", "a"));
    let second = source("two", diff("repository", "a"));
    let development = label(
        1,
        &first,
        anchor(&first, "repository", "one.md", 0, true),
        Split::Development,
    );
    let held_out = label(
        2,
        &second,
        anchor(&second, "repository", "two.md", 20, true),
        Split::HeldOut,
    );
    assert!(
        matches!(freeze_labels(vec![first,second],vec![development,held_out]),Err(EvalError::SplitLeakage {field}) if field=="group")
    );

    let first = source("one", diff("repository", "a"));
    let second = source("two", diff("repository", "c"));
    let development = label(
        1,
        &first,
        anchor(&first, "repository", "one.md", 0, true),
        Split::Development,
    );
    let held_out = label(
        2,
        &second,
        anchor(&second, "repository", "one.md", 20, true),
        Split::HeldOut,
    );
    assert!(freeze_labels(vec![first, second], vec![development, held_out]).is_ok());
}
#[test]
fn session_groups_come_from_provider_and_session_on_the_source() {
    for (provider, session, leaks) in [
        ("codex", "session_1", true),
        ("claude_code", "session_1", false),
        ("codex", "different_session", false),
    ] {
        let first = source(
            "one",
            SourceInput::SessionPair {
                provider: ProviderName::new("codex").unwrap(),
                session: SessionId::new("session_1").unwrap(),
            },
        );
        let second = source(
            "two",
            SourceInput::SessionPair {
                provider: ProviderName::new(provider).unwrap(),
                session: SessionId::new(session).unwrap(),
            },
        );
        let development = label(1, &first, support::occurrence(1), Split::Development);
        let held_out = label(2, &second, support::occurrence(2), Split::HeldOut);
        let result = freeze_labels(vec![first, second], vec![development, held_out]);
        if leaks {
            assert!(matches!(result,Err(EvalError::SplitLeakage {field}) if field=="group"));
        } else {
            assert!(result.is_ok());
        }
    }
}
#[test]
fn custom_groups_use_location_even_when_the_schema_or_source_id_differs() {
    for (second_location, leaks) in [("custom.jsonl", true), ("different.jsonl", false)] {
        let custom = |location: &str, tag: &str| {
            serde_json::from_value::<SourceInput>(json!({"kind":"custom_entry","location":location,
            "schema":{"format":"jsonl","record_schema":{"type":"object","title":tag},"fields":{}}}))
            .unwrap()
        };
        let first = source("one", custom("custom.jsonl", "first schema"));
        let second = source("two", custom(second_location, "different schema"));
        let development = label(1, &first, support::occurrence(1), Split::Development);
        let held_out = label(2, &second, support::occurrence(2), Split::HeldOut);
        let result = freeze_labels(vec![first, second], vec![development, held_out]);
        if leaks {
            assert!(matches!(result,Err(EvalError::SplitLeakage {field}) if field=="group"));
        } else {
            assert!(result.is_ok());
        }
    }
}
#[test]
fn supplied_session_and_group_are_rejected_and_neither_is_serialized() {
    let mut value = serde_json::to_value(case(1, Label::Yes, Split::HeldOut)).unwrap();
    value.as_object_mut().unwrap().remove("session");
    value.as_object_mut().unwrap().remove("group");
    let label: LabeledCase = serde_json::from_value(value.clone()).unwrap();
    let roundtrip = serde_json::to_value(label).unwrap();
    assert!(roundtrip.get("session").is_none());
    assert!(roundtrip.get("group").is_none());
    for (field, forged) in [
        ("session", json!({"provider":"codex","session":"forged"})),
        ("group", json!("forged_group")),
    ] {
        let mut forged_case = value.clone();
        forged_case[field] = forged;
        assert!(serde_json::from_value::<LabeledCase>(forged_case).is_err());
    }
}
#[test]
fn code_anchor_requires_the_sources_namespace_repository_and_document_path() {
    let source = source("one", code("repository"));
    let valid = anchor(&source, "repository", "guide.md", 0, false);
    let mut wrong_namespace = valid.clone();
    if let SourceOccurrence::CodeMatch { input_id, .. } = &mut wrong_namespace {
        *input_id = SourceInputId::new("forged").unwrap();
    }
    let wrong_repository = anchor(&source, "other_repository", "guide.md", 0, false);
    let missing_path = anchor(&source, "repository", "", 0, false);
    for invalid_anchor in [
        wrong_namespace,
        wrong_repository,
        missing_path,
        support::occurrence(1),
    ] {
        let label = label(1, &source, invalid_anchor, Split::HeldOut);
        assert!(
            matches!(freeze_labels(vec![source.clone()],vec![label]),Err(EvalError::Invalid {field}) if field.starts_with("labels.source_anchor"))
        );
    }
}
#[test]
fn case_and_anchor_question_uniqueness_survive_group_derivation() {
    let source = source("one", code("repository"));
    let one = label(
        1,
        &source,
        anchor(&source, "repository", "guide.md", 0, false),
        Split::HeldOut,
    );
    let mut two = one.clone();
    two.case_id = CaseId::new("different_case").unwrap();
    assert!(
        matches!(freeze_labels(vec![source.clone()],vec![one.clone(),two.clone()]),Err(EvalError::Invalid {field}) if field=="labels.source_anchor")
    );
    two.question = sunstack_rules::QuestionRef::RuleGate { position: 0 };
    assert!(freeze_labels(vec![source.clone()], vec![one.clone(), two.clone()]).is_ok());
    two.case_id = one.case_id.clone();
    assert!(
        matches!(freeze_labels(vec![source],vec![one,two]),Err(EvalError::Invalid {field}) if field=="labels.case_id")
    );
}
