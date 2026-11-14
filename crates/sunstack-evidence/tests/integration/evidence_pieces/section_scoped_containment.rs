use super::*;
use serde_json::json;

#[test]
fn section_scoped_containment_crosses_languages_and_excludes_earlier_headings() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/blocks.yml"), "id: choice\nlanguage: Html\nrule: {kind: element, regex: '^<ex-choice(?: |>)'}\n---\nid: image\nlanguage: Html\nrule: {kind: element, regex: '^<img(?: |>)'}\n---\nid: fence\nlanguage: Markdown\nrule: {kind: fenced_code_block}\n").unwrap();
    let text = "文<h2>Earlier</h2><img src='old'>\n<h2>Own</h2><img src='before'>\n<script type='text/markdown'>\n```mermaid\ngraph LR\n A --> B\n```\n</script>\n<ex-choice id='own'>Choose</ex-choice><img src='after'>\n<h2>Empty</h2><ex-choice id='empty'>Choose</ex-choice>\n<h2>Later</h2><img src='later'>";
    std::fs::write(root.path().join("page.html"), text).unwrap();
    let settings = serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
    let project =
        load_evidence_project(&root.path().join("sgconfig.yml"), &[], None, &settings).unwrap();
    let kinds = serde_json::from_value(json!({
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
        "html_picture":{"rule":"image","fields":{}},
        "markdown_picture":{"rule":"fence","fields":{}},
        "choice":{"rule":"choice","fields":{
            "html_before":{"from":"inside","kind":"html_picture","scope":{"section_prefix":"outline"}},
            "markdown_before":{"from":"inside","kind":"markdown_picture","scope":{"section_prefix":"outline"}},
            "html_section":{"from":"inside","kind":"html_picture","scope":{"section":"outline"}},
            "markdown_section":{"from":"inside","kind":"markdown_picture","scope":{"section":"outline"}},
            "body":{"from":"remainder"}
        }}
    })).unwrap();

    let catalog = build_pieces(
        &project,
        &kinds,
        &["page.html".into()],
        &mut RecordingSource::new(root.path()),
        limits(),
    );

    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let own = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "choice" && piece.text.contains("id='own'"))
        .unwrap();
    for (name, expected) in [
        ("html_before", 1),
        ("markdown_before", 1),
        ("html_section", 2),
        ("markdown_section", 1),
    ] {
        let FieldValue::Texts { values, spans } = field(own, name) else {
            panic!("{name} must be a text list")
        };
        assert_eq!(values.len(), expected, "{name}");
        for (value, span) in values.iter().zip(spans) {
            assert_eq!(value, &text[span.byte_start..span.byte_end]);
            assert_eq!(span.source, own.location.span.source);
        }
        assert!(
            spans
                .windows(2)
                .all(|pair| pair[0].byte_start <= pair[1].byte_start)
        );
    }
    assert!(matches!(field(own,"body"),FieldValue::Text{value,..} if value == &own.text));
    let ModelView::Bounded { fields, cut } = &own.model_view else {
        panic!("bounded view required")
    };
    assert!(cut.is_empty());
    assert!(
        matches!(&fields[&FieldName::new("markdown_before").unwrap()], ModelFieldValue::List(values) if values.len() == 1 && matches!(&values[0], ModelFieldValue::Text(value) if value.starts_with("```mermaid\n")))
    );
    let empty = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "choice" && piece.text.contains("id='empty'"))
        .unwrap();
    for name in [
        "html_before",
        "markdown_before",
        "html_section",
        "markdown_section",
    ] {
        assert!(
            matches!(field(empty,name),FieldValue::Texts{values,spans} if values.is_empty() && spans.is_empty()),
            "{name}"
        );
    }
}

#[test]
fn section_scoped_containment_keeps_unanswered_outline_and_picture_selection_pending() {
    for selected_kind in ["outline", "picture"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        std::fs::write(root.path().join("rules/html.yml"),"id: choice\nlanguage: Html\nrule: {kind: element, regex: '^<ex-choice'}\n---\nid: image\nlanguage: Html\nrule: {kind: element, regex: '^<img'}\n").unwrap();
        std::fs::write(
            root.path().join("page.html"),
            "<h2>Section</h2><img src='a'><ex-choice>Choose</ex-choice>",
        )
        .unwrap();
        let project = load_evidence_project(
            &root.path().join("sgconfig.yml"),
            &[],
            None,
            &Default::default(),
        )
        .unwrap();
        let mut wire = json!({
            "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{"body":{"from":"section","text":"full"}}},
            "picture":{"rule":"image","fields":{"body":{"from":"remainder"}}},
            "choice":{"rule":"choice","fields":{"pictures":{"from":"inside","kind":"picture","scope":{"section_prefix":"outline"}},"body":{"from":"remainder"}}}
        });
        wire[selected_kind]["select"] = json!({"question":{"type":"noul","instructions":{"check":"body","task":"Select this candidate."}},"yes_at_or_above":0.7,"no_at_or_below":0.3});
        let kinds = serde_json::from_value(wire).unwrap();
        let mut source = RecordingSource::new(root.path());
        let mut build = EvidenceBuild::start(
            &project,
            &kinds,
            &["page.html".into()],
            &mut source,
            limits(),
        )
        .unwrap();
        let BuildStep::NeedsSelection {
            kind, candidates, ..
        } = build.advance(ChosenCandidates::default(), &mut source)
        else {
            panic!("selection required")
        };
        assert_eq!(kind.as_str(), selected_kind);
        build.defer_selection(
            &candidates
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        );
        let BuildStep::Done(catalog) = build.advance(ChosenCandidates::default(), &mut source)
        else {
            panic!("done required")
        };
        let choice = catalog
            .pieces
            .iter()
            .find(|piece| piece.kind.as_str() == "choice")
            .unwrap();
        assert!(
            matches!(
                field(choice, "pictures"),
                FieldValue::Unavailable {
                    reason: FieldUnavailable::SelectionPending
                }
            ),
            "{selected_kind}"
        );
        assert!(matches!(field(choice,"body"),FieldValue::Text{value,..} if value == &choice.text));
    }
}
