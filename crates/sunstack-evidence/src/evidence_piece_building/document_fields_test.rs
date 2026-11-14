#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn containing_fields_filter_same_file_and_document_matches_by_subject_token() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/rows.yml"), "id: subjects\nlanguage: Html\nrule: {kind: element, regex: '^<h2'}\n---\nid: rows\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n").unwrap();
    let rows = "<p>R1 → U1</p>\n<p>R10 → U10</p>\n<p>XR1 → XU1</p>\n<p>R2 → (U1)</p>\n<p>R3 → U1_extra</p>";
    std::fs::write(
        root.path().join("owner.html"),
        format!("<h2 id='U1'>Requirement</h2>\n{rows}"),
    )
    .unwrap();
    std::fs::write(root.path().join("specification.html"), rows).unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({"requirement":{"rule":"subjects","fields":{
        "zid":{"from":"capture","regex":"id='([^']+)'"},
        "local":{"from":"same_file","rule":"rows","text":"full","containing":"zid"},
        "remote":{"from":"document","path":["missing.html","specification.html"],"rule":"rows","containing":"zid"},
        "missing":{"from":"capture","regex":"NEVER:([^ ]+)"},
        "unavailable":{"from":"same_file","rule":"rows","containing":"missing"},
        "remote_unavailable":{"from":"document","path":"specification.html","rule":"rows","containing":"missing"}
    }}})).unwrap();
    let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();

    let catalog = crate::build_pieces(
        &project,
        &kinds,
        &["owner.html".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10000,
            max_total_bytes: 100000,
            max_matches: 100,
            evidence_max_chars: 6000,
        },
    );

    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let fields = &catalog.pieces[0].fields;
    for name in ["unavailable", "remote_unavailable"] {
        assert!(matches!(
            fields[&FieldName::new(name).unwrap()],
            FieldValue::Unavailable {
                reason: FieldUnavailable::ContainingFieldUnavailable
            }
        ));
    }
    let expected = ["<p>R1 → U1</p>", "<p>R2 → (U1)</p>"];
    for name in ["local", "remote"] {
        let value = &fields[&FieldName::new(name).unwrap()];
        let (values, spans) = match value {
            FieldValue::Texts { values, spans, .. }
            | FieldValue::Document { values, spans, .. } => (values, spans),
            _ => panic!("{value:?}"),
        };
        assert_eq!(values, &expected);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].byte_start < spans[1].byte_start);
    }
}

#[test]
fn containing_accepts_list_needles_and_empty_lists_across_document_languages() {
    for surface in ["html", "markdown", "island"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        let (extension, language, subject_kind, row_kind, id_kind, subject, rows) =
            if surface == "html" {
                (
                    "html",
                    "Html",
                    "element",
                    "element",
                    "text",
                    "<h2>Subject</h2><i>U4</i><i>U7</i>",
                    "<p>U4 (R1–R3, R5, R7)</p>\n<p>U40 unrelated</p>\n<p>U7 proof</p>",
                )
            } else {
                (
                    if surface == "island" { "html" } else { "md" },
                    "Markdown",
                    "atx_heading",
                    "paragraph",
                    "inline",
                    "# Subject\n\nU4\n\nU7\n",
                    "U4 (R1–R3, R5, R7)\n\nU40 unrelated\n\nU7 proof\n",
                )
            };
        std::fs::write(root.path().join("rules/rows.yml"), format!("id: subjects\nlanguage: {language}\nrule: {{kind: {subject_kind}, regex: '^{}'}}\n---\nid: rows\nlanguage: {language}\nrule: {{kind: {row_kind}, regex: '{}'}}\n---\nid: ids\nlanguage: {language}\nrule: {{kind: {id_kind}, regex: '^U[47]$'}}\n---\nid: absent\nlanguage: {language}\nrule: {{kind: {id_kind}, regex: '^NEVER$'}}\n", if surface=="html" {"<h2"} else {"# Subject"}, if surface=="html" {"^<p"} else {"^U[0-9]+ "})).unwrap();
        let wrap = |text: &str| {
            if surface == "island" {
                format!("<script type='text/markdown'>\n{text}\n</script>")
            } else {
                text.to_owned()
            }
        };
        let owner = format!("owner.{extension}");
        let sibling = format!("sibling.{extension}");
        std::fs::write(
            root.path().join(&owner),
            wrap(&format!("{subject}\n{rows}")),
        )
        .unwrap();
        std::fs::write(root.path().join(&sibling), wrap(rows)).unwrap();
        let settings = serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
        let project =
            crate::load_evidence_project(&root.path().join("sgconfig.yml"), &[], None, &settings)
                .unwrap();
        let kinds = serde_json::from_value(json!({"subject":{"rule":"subjects","fields":{
            "ids":{"from":"same_file","rule":"ids"},
            "empty":{"from":"same_file","rule":"absent"},
            "own":{"from":"same_file","rule":"rows","text":"full","containing":"ids"},
            "remote":{"from":"document","path":["missing",sibling],"rule":"rows","containing":"ids"},
            "none":{"from":"same_file","rule":"rows","containing":"empty"},
            "remote_none":{"from":"document","path":sibling,"rule":"rows","containing":"empty"}
        }}})).unwrap();
        let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();

        let catalog = crate::build_pieces(
            &project,
            &kinds,
            &[owner.into()],
            &mut source,
            PieceBuildLimits {
                max_files: 10,
                max_file_bytes: 10000,
                max_total_bytes: 100000,
                max_matches: 100,
                evidence_max_chars: 6000,
            },
        );

        assert!(
            catalog.failures.is_empty(),
            "{surface}: {:?}",
            catalog.failures
        );
        assert_eq!(catalog.pieces.len(), 1, "{surface}");
        for name in ["own", "remote", "none", "remote_none"] {
            let field = &catalog.pieces[0].fields[&FieldName::new(name).unwrap()];
            let (values, spans) = match field {
                FieldValue::Texts { values, spans, .. }
                | FieldValue::Document { values, spans, .. } => (values, spans),
                _ => panic!("{surface} {name}: {field:?}"),
            };
            assert_eq!(
                values.len(),
                if name.contains("none") { 0 } else { 2 },
                "{surface} {name}: {values:?}"
            );
            assert_eq!(spans.len(), values.len());
            assert!(values.iter().all(|value| !value.contains("U40")));
        }
    }
}

#[test]
fn captures_feed_document_filters_in_dependency_order_without_changing_scalar_capture() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/rows.yml"), "id: subjects\nlanguage: Html\nrule: {kind: element, regex: '^<h2'}\n---\nid: rows\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n").unwrap();
    std::fs::write(root.path().join("owner.html"), "<h2>é U2, U3, U2</h2>").unwrap();
    std::fs::write(
        root.path().join("spec.html"),
        "<p>R2 → U2, U3</p>\r\n<p>R10 → U3</p>\r\n<p>R99 → U30</p>",
    )
    .unwrap();
    std::fs::write(
        root.path().join("program.html"),
        "<p>é R2 realization</p>\r\n<p>R10 realization</p>\r\n<p>R20 unrelated</p>",
    )
    .unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({"subject":{"rule":"subjects","fields":{
        "z_ids":{"from":"captures","regex":"\\bU[0-9]+\\b"},
        "b_obligations":{"from":"document","path":"spec.html","rule":"rows","containing":"z_ids"},
        "a_rids":{"from":"captures","of":"b_obligations","regex":"\\bR[0-9]+\\b"},
        "trace":{"from":"document","path":["program.html"],"rule":"rows","containing":"a_rids"},
        "scalar":{"from":"capture","regex":"(U[0-9]+)"},
        "scalar_ids":{"from":"captures","of":"scalar","regex":"U[0-9]+"},
        "rest":{"from":"remainder"},
        "empty":{"from":"captures","regex":"NEVER"},
        "none":{"from":"document","path":["program.html"],"rule":"rows","containing":"empty"},
        "missing":{"from":"document","path":"absent.html","rule":"rows"},
        "unavailable":{"from":"captures","of":"missing","regex":"R[0-9]+"}
    }}}))
    .unwrap();
    let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();

    let catalog = crate::build_pieces(
        &project,
        &kinds,
        &["owner.html".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10000,
            max_total_bytes: 100000,
            max_matches: 100,
            evidence_max_chars: 6000,
        },
    );

    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let fields = &catalog.pieces[0].fields;
    let field = |name| &fields[&FieldName::new(name).unwrap()];
    for (name, expected) in [
        ("z_ids", vec!["U2", "U3"]),
        ("a_rids", vec!["R2", "R10"]),
        ("scalar_ids", vec!["U2"]),
        ("empty", vec![]),
    ] {
        let FieldValue::Texts { values, spans, .. } = field(name) else {
            panic!("{name}: {:?}", field(name))
        };
        assert_eq!(values, &expected);
        assert_eq!(spans.len(), values.len());
        for (value, span) in values.iter().zip(spans) {
            let SourceRef::Document { path, .. } = &span.source else {
                panic!("document span")
            };
            let original = std::fs::read_to_string(root.path().join(path)).unwrap();
            assert_eq!(&original[span.byte_start..span.byte_end], value);
            assert_eq!(
                *span,
                crate::source_acquisition::source_capture::source_span(
                    &span.source,
                    &original,
                    span.byte_start..span.byte_end
                )
            );
        }
    }
    let FieldValue::Document { values, .. } = field("trace") else {
        panic!("trace")
    };
    assert_eq!(
        values,
        &["<p>é R2 realization</p>", "<p>R10 realization</p>"]
    );
    assert!(matches!(field("none"),FieldValue::Document {values,..} if values.is_empty()));
    assert!(matches!(
        field("unavailable"),
        FieldValue::Unavailable { .. }
    ));
    assert!(matches!(field("scalar"),FieldValue::Text {value,..} if value=="U2"));
    assert!(matches!(field("rest"),FieldValue::Text {value,..} if value=="<h2>é , U3, U2</h2>"));
}

#[test]
fn attached_siblings_keep_whole_blocks_stop_at_boundaries_and_report_caps() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/blocks.yml"), "id: subjects\nlanguage: Markdown\nrule: {kind: atx_heading, regex: '^# Subject'}\n---\nid: leads\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^\\*\\*D10b'}\n---\nid: boundary\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^\\*\\*D[0-9]'}\n").unwrap();
    let block = "**D10b U1.** classifier\n\n```rust\nenum AttemptOutcome { NoResponse }\n```\n\n- exhaustion rule\n- credential rule\n\nClosing paragraph.\n";
    let page = format!(
        "# Subject\n\n{block}\n**D11.** next\n\nMust not attach.\n\n## Heading\n\nHeading body.\n"
    );
    std::fs::write(root.path().join("owner.md"), &page).unwrap();
    std::fs::write(root.path().join("program.md"), &page).unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds=serde_json::from_value(json!({"subject":{"rule":"subjects","fields":{
        "ids":{"from":"captures","regex":"(Subject)"},
        "local":{"from":"same_file","rule":"leads","text":"full","attach":{"until":"boundary","max_siblings":12}},
        "remote":{"from":"document","path":["program.md"],"rule":"leads","attach":{"until":"boundary","max_siblings":12}},
        "capped":{"from":"document","path":"program.md","rule":"leads","attach":{"until":"boundary","max_siblings":1}},
        "names":{"from":"captures","of":"capped","regex":"AttemptOutcome"},
        "heading_stop":{"from":"document","path":"program.md","rule":"leads","attach":{"until":"leads","max_siblings":12}}
    }}})).unwrap();
    let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();

    let catalog = crate::build_pieces(
        &project,
        &kinds,
        &["owner.md".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10000,
            max_total_bytes: 100000,
            max_matches: 100,
            evidence_max_chars: 6000,
        },
    );

    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let piece = &catalog.pieces[0];
    for name in ["local", "remote"] {
        let serialized =
            serde_json::to_value(&piece.fields[&FieldName::new(name).unwrap()]).unwrap();
        let value = serialized["values"][0].as_str().unwrap();
        assert_eq!(value.trim_end(), block.trim_end());
        assert!(serialized.get("cut").is_none());
        let span = &piece.fields[&FieldName::new(name).unwrap()].spans()[0];
        assert_eq!(&page[span.byte_start..span.byte_end], value);
    }
    let fields = serde_json::to_value(&piece.fields).unwrap();
    assert_eq!(fields["capped"]["cut"], true);
    assert_eq!(fields["names"]["cut"], true);
    assert!(
        fields["capped"]["values"][0]
            .as_str()
            .unwrap()
            .contains("enum AttemptOutcome")
    );
    assert!(
        !fields["capped"]["values"][0]
            .as_str()
            .unwrap()
            .contains("exhaustion rule")
    );
    assert!(
        fields["heading_stop"]["values"][0]
            .as_str()
            .unwrap()
            .contains("Must not attach")
    );
    assert!(
        !fields["heading_stop"]["values"][0]
            .as_str()
            .unwrap()
            .contains("## Heading")
    );
    let crate::ModelView::Bounded { cut, .. } = &piece.model_view else {
        panic!("bounded")
    };
    assert!(cut.contains(&FieldName::new("capped").unwrap()));
    assert!(cut.contains(&FieldName::new("names").unwrap()));
}

#[test]
fn attachment_filters_leads_before_expansion_and_keeps_html_and_island_boundaries() {
    for surface in ["html", "island"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        let (language, node, leads, boundary, content) = if surface == "html" {
            (
                "Html",
                "element",
                "^<p",
                "^<aside",
                "<h1>Subject U1</h1>\n<p>Lead U1</p>\n<pre>enum Outcome</pre>\n<ul><li>rules</li></ul>\n<p>Tail U2</p>\n<aside>Boundary</aside>\n<p>Other U20</p>\n<h2>Stop</h2>\n<p>Later U1</p>",
            )
        } else {
            (
                "Markdown",
                "paragraph",
                "^Lead|^Tail|^Other|^Later",
                "^Boundary",
                "# Subject U1\n\nLead U1\n\n```rust\nenum Outcome {}\n```\n\n- rules\n\nTail U2\n\nBoundary\n\nOther U20\n\n## Stop\n\nLater U1\n",
            )
        };
        std::fs::write(root.path().join("rules/blocks.yml"), format!("id: subject\nlanguage: {language}\nrule: {{kind: {}, regex: '^{}'}}\n---\nid: leads\nlanguage: {language}\nrule: {{kind: {node}, regex: '{leads}'}}\n---\nid: boundary\nlanguage: {language}\nrule: {{kind: {node}, regex: '{boundary}'}}\n",if surface=="html" {"element"} else {"atx_heading"},if surface=="html" {"<h1"} else {"# Subject"})).unwrap();
        let page = if surface == "island" {
            format!("<script type='text/markdown'>\n{content}</script>\n<p>Outside U1</p>")
        } else {
            content.to_owned()
        };
        std::fs::write(root.path().join("owner.html"), &page).unwrap();
        std::fs::write(root.path().join("sibling.html"), &page).unwrap();
        let settings=serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
        let project =
            crate::load_evidence_project(&root.path().join("sgconfig.yml"), &[], None, &settings)
                .unwrap();
        let kinds=serde_json::from_value(json!({"subject":{"rule":"subject","fields":{
            "ids":{"from":"captures","regex":"U1"},
            "local":{"from":"same_file","rule":"leads","containing":"ids","attach":{"until":"boundary","max_siblings":3}},
            "remote":{"from":"document","path":"sibling.html","rule":"leads","containing":"ids","attach":{"until":"boundary","max_siblings":3}}
        }}})).unwrap();
        let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
        let catalog = crate::build_pieces(
            &project,
            &kinds,
            &["owner.html".into()],
            &mut source,
            PieceBuildLimits {
                max_files: 10,
                max_file_bytes: 10000,
                max_total_bytes: 100000,
                max_matches: 100,
                evidence_max_chars: 6000,
            },
        );
        assert!(
            catalog.failures.is_empty(),
            "{surface}: {:?}",
            catalog.failures
        );
        let fields = &catalog.pieces[0].fields;
        for name in ["local", "remote"] {
            let FieldValue::Texts { values, cut, spans } = &fields[&FieldName::new(name).unwrap()]
            else {
                panic!("{surface} {name}: {:?}", fields)
            };
            assert!(!cut, "{surface} {name}: {values:?}"); // exactly 3 following nodes fits
            assert_eq!(values.len(), 2, "{surface} {name}: {values:?}");
            assert!(values[0].contains("enum Outcome") && values[0].contains("Tail U2"));
            assert!(!values[0].contains("Boundary"));
            assert!(
                !values
                    .iter()
                    .any(|value| value.contains("Outside U1") || value.contains("Other U20"))
            );
            for (value, span) in values.iter().zip(spans) {
                assert_eq!(value, &page[span.byte_start..span.byte_end]);
            }
        }
    }
}
