#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use sunstack_evidence::*;
use sunstack_rules::*;
use support::*;
#[test]
fn code_occurrence_namespace_joins_and_complete_repository_discovery_alone_establishes_a_miss() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    std::fs::write(root.path().join("source.rs"), "fn x() {}\n").unwrap();
    specification.source_files = vec![root.path().join("source.rs")];
    specification.sources[0].source = SourceInput::Code {
        repository: RepositoryId::new("code").unwrap(),
        revision: RevisionHead::Commit {
            revision: "a".repeat(40),
        },
        paths: vec![QueryPath::Exact {
            path: "source.rs".into(),
        }],
    };
    let identity = specification.sources[0].source.source_identity().unwrap();
    let occurrence:SourceOccurrence=serde_json::from_value(json!({"kind":"code_match","input_id":identity,"unit":null,"span":{"source":{"kind":"repository_file","repository":"code","revision":{"kind":"commit","commit_id":"a".repeat(40),"side":"after"},"path":"source.rs","content_identity":"source-content"},"byte_start":0,"byte_end":9,"start_line":1,"start_column":1,"end_line":1,"end_column":10}})).unwrap();
    specification.labels[0].source_anchor = occurrence.clone();
    let frozen = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let mut input_report = located_input(&frozen);
    input_report.location = InputLocation::Repository {
        repository: RepositoryId::new("code").unwrap(),
    };
    let mut observed = entry(
        1,
        json!({"kind":"finding","path":["answer_1"]}),
        Some("passed"),
        Some(1),
    );
    observed.occurrence = occurrence.clone();
    observed.candidate_id.occurrence = occurrence.occurrence_identity().unwrap();
    let mut found = bundle(&frozen, vec![observed], Some(0.001), 1);
    found.inputs = vec![input_report.clone()];
    let grade = score(&[found], &frozen).unwrap();
    assert_eq!(grade.filter.kept, 1);
    for complete in [true, false] {
        let mut report = bundle(&frozen, vec![], Some(0.0), 1);
        let mut input = input_report.clone();
        if !complete {
            input.capture = InputCaptureStatus::Incomplete;
        }
        report.inputs = vec![input];
        let grade = score(&[report], &frozen).unwrap();
        assert_eq!(grade.filter.missed.len(), usize::from(complete));
        assert_eq!(grade.filter.unknown.len(), usize::from(!complete));
    }
}
