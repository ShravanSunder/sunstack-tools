#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evals::*;
use sunstack_evidence::*;
use support::{case, input};
fn source(name: &str, repository: &str, revision: &str) -> CaseSource {
    CaseSource {
        source_id: CaseSourceId::new(name).unwrap(),
        source: SourceInput::Code {
            repository: RepositoryId::new(repository).unwrap(),
            revision: RevisionHead::Commit {
                revision: revision.repeat(40),
            },
            paths: vec![QueryPath::Glob {
                pattern: "**/*.md".into(),
            }],
        },
        evidence: Default::default(),
    }
}
fn anchor(source: &CaseSource, path: &str, field: bool, position: usize) -> SourceOccurrence {
    let SourceInput::Code {
        repository,
        revision: RevisionHead::Commit { revision },
        ..
    } = &source.source
    else {
        panic!("fixture source must be code")
    };
    let span = SourceSpan {
        source: SourceRef::RepositoryFile {
            repository: repository.clone(),
            revision: SourceRevision::Commit {
                commit_id: GitObjectId::new(revision.clone()).unwrap(),
                side: SourceSide::After,
            },
            path: path.into(),
            content_identity: "captured guide".into(),
        },
        byte_start: position,
        byte_end: position + 4,
        start_line: position + 1,
        start_column: 1,
        end_line: position + 1,
        end_column: 5,
    };
    let input_id = source.source.source_identity().unwrap();
    if field {
        SourceOccurrence::DocumentField {
            input_id,
            source: span.source.clone(),
            field: format!("field_{position}"),
            state: Box::new(FrontmatterFieldLocation::Present { span }),
        }
    } else {
        SourceOccurrence::DocumentUnit {
            input_id,
            unit_kind: MarkdownUnitKind::Paragraph,
            occurrence: MarkdownOccurrence::Unit { span },
        }
    }
}
#[test]
fn native_document_units_and_fields_share_the_document_group_across_inputs_and_revisions() {
    for (first_field, second_field) in [(false, false), (false, true), (true, true)] {
        let root = tempfile::tempdir().unwrap();
        let first = source("one", "docs", "a");
        let second = source("two", "docs", "b");
        let mut development = case(1, Label::Yes, Split::Development);
        development.source = first.source_id.clone();
        development.project = None;
        development.source_anchor = anchor(&first, "skills/guide.md", first_field, 0);
        let mut held_out = case(2, Label::No, Split::HeldOut);
        held_out.source = second.source_id.clone();
        held_out.project = None;
        held_out.source_anchor = anchor(&second, "skills/guide.md", second_field, 20);
        assert_ne!(
            first.source.source_identity().unwrap(),
            second.source.source_identity().unwrap()
        );
        let mut specification = input(root.path(), vec![development, held_out]);
        specification.sources = vec![first, second];
        assert!(
            matches!(freeze_cases(&root.path().join("cases"),&specification),Err(EvalError::SplitLeakage {field}) if field=="group")
        );
    }
}
#[test]
fn native_document_group_separates_distinct_paths_or_repositories() {
    for (repository, path) in [
        ("docs", "skills/other.md"),
        ("other_docs", "skills/guide.md"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let first = source("one", "docs", "a");
        let second = source("two", repository, "b");
        let mut development = case(1, Label::Yes, Split::Development);
        development.source = first.source_id.clone();
        development.project = None;
        development.source_anchor = anchor(&first, "skills/guide.md", false, 0);
        let mut held_out = case(2, Label::No, Split::HeldOut);
        held_out.source = second.source_id.clone();
        held_out.project = None;
        held_out.source_anchor = anchor(&second, path, true, 20);
        let mut specification = input(root.path(), vec![development, held_out]);
        specification.sources = vec![first, second];
        assert!(freeze_cases(&root.path().join("cases"), &specification).is_ok());
    }
}
