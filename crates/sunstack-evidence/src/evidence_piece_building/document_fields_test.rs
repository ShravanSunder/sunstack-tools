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
            FieldValue::Texts { values, spans } | FieldValue::Document { values, spans, .. } => {
                (values, spans)
            }
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
                FieldValue::Texts { values, spans }
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
