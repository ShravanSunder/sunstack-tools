#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn regex_groups_empty_matches_literal_projection_and_unavailable_sources() {
    // Arrange a raw piece with duplicates and alternative capture groups.
    let text = "é U2 R3 U2 <tag>";
    let file = MatchedFile {
        text: text.into(),
        index: Default::default(),
        native_units: Default::default(),
        source: SourceRef::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "page.html".into(),
            content_identity: content_digest(text.as_bytes()),
        },
    };
    let range = MatchRange {
        start: 0,
        end: text.len(),
    };
    // Act: whole-match and grouped patterns use the same ordered deduplication.
    for pattern in ["[UR][0-9]+", "(U[0-9]+)|(R[0-9]+)|()"] {
        let field = capture_list(pattern, None, false, &file, &range, None);
        // Assert: zero-length alternatives do not become empty needles.
        assert!(matches!(field,FieldValue::Texts{values,cut:false,..} if values==["U2","R3"]));
    }
    assert!(
        matches!(capture_list(".*?",None,false,&file,&range,None),FieldValue::Texts {values,..} if values.is_empty())
    );
    for input in [
        FieldValue::Absent,
        FieldValue::Text {
            value: "".into(),
            spans: vec![],
        },
        FieldValue::Texts {
            values: vec![],
            spans: vec![],
            cut: false,
        },
    ] {
        assert!(
            matches!(capture_list("U",Some(&input),true,&file,&range,None),FieldValue::Texts {values,..} if values.is_empty())
        );
    }
    let pending = FieldValue::Unavailable {
        reason: FieldUnavailable::SelectionPending,
    };
    assert_eq!(
        capture_list("U", Some(&pending), true, &file, &range, None),
        pending
    );
    let input = FieldValue::Text {
        value: "<tag>".into(),
        spans: vec![source_span(&file, &range)],
    };
    let captured = capture_list("(<tag>)", Some(&input), true, &file, &range, None);
    assert_eq!(captured.spans(), input.spans()); // transformed string keeps its context
    let definition:EvidenceKindDefinition=serde_json::from_value(json!({"html_units":{"mode":"unit","units":["section"]},"fields":{"ids":{"from":"captures","regex":"(<tag>)"}}})).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let fields = BTreeMap::from([(FieldName::new("ids").unwrap(), captured)]);
    let view = crate::project_kind_model_view(
        &project,
        &EvidenceKindName::new("subject").unwrap(),
        &definition,
        &fields,
        1000,
    );
    assert_eq!(
        serde_json::to_value(view).unwrap()["fields"]["ids"],
        json!(["<tag>"])
    );
}
