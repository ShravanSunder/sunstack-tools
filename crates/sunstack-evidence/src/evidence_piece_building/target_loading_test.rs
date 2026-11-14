#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn anchored_targets_collect_the_matching_kind_instead_of_the_first_piece() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/rows.yml"), "id: rows\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n---\nid: links\nlanguage: Html\nrule: {kind: element, regex: '^<a'}\n").unwrap();
    let target = "<p id='U1'>First requirement</p>\n<p><span id='U2'>Second requirement</span></p>\n<div id='U3'>No requirement here</div>";
    std::fs::write(root.path().join("requirements.html"), target).unwrap();
    std::fs::write(
        root.path().join("owner.html"),
        "<a href='requirements.html#U2'>R2</a>",
    )
    .unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({
        "requirement":{"rule":"rows","fields":{"body":{"from":"remainder"}}},
        "obligation":{"rule":"links","fields":{"needs":{"from":"target","regex":"href='([^']+)'","resolve":"path","as_kind":"requirement"}}}
    })).unwrap();
    for files in [
        vec!["owner.html".into()],
        vec!["owner.html".into(), "requirements.html".into()],
    ] {
        let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
        let catalog = crate::build_pieces(
            &project,
            &kinds,
            &files,
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
        let owner = catalog
            .pieces
            .iter()
            .find(|piece| piece.kind.as_str() == "obligation")
            .unwrap();
        let FieldValue::Targets { values } = &owner.fields[&FieldName::new("needs").unwrap()]
        else {
            panic!("expected target")
        };
        let body = &values[0].fields.as_ref().unwrap()[&FieldName::new("body").unwrap()];
        assert!(
            matches!(body, FieldValue::Text {value,..} if value=="<p><span id='U2'>Second requirement</span></p>"),
            "{body:?}"
        );
    }
    for fragment in ["U3", "missing"] {
        std::fs::write(
            root.path().join("owner.html"),
            format!("<a href='requirements.html#{fragment}'>R3</a>"),
        )
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
        let owner = catalog
            .pieces
            .iter()
            .find(|piece| piece.kind.as_str() == "obligation")
            .unwrap();
        assert!(matches!(
            owner.fields[&FieldName::new("needs").unwrap()],
            FieldValue::Unavailable {
                reason: FieldUnavailable::TargetKindMissing
            }
        ));
    }
}

#[test]
fn markdown_target_kinds_follow_heading_anchors_and_preserve_file_openings() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/links.yml"),
        "id: links\nlanguage: Markdown\nrule: {kind: inline, regex: '^\\[read\\]'}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("owner.md"), "[read](target.md#second)\n").unwrap();
    std::fs::write(
        root.path().join("target.md"),
        "# First\n\nFirst body\n\n# Second\n\nSecond body\n",
    )
    .unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &serde_json::from_value(json!({"markdown":{"heading_anchors":"github"}})).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({
        "sections":{"markdown_units":{"mode":"unit","units":["section"]},"fields":{"body":{"from":"section","text":"full"}}},
        "links":{"rule":"links","fields":{"targets":{"from":"target","regex":"\\]\\(([^)]+)\\)","resolve":"path","as_kind":"sections"}}}
    })).unwrap();
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
    let owner = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "links")
        .unwrap();
    let FieldValue::Targets { values } = &owner.fields[&FieldName::new("targets").unwrap()] else {
        panic!("expected target")
    };
    assert!(values[0].opening.as_ref().unwrap().starts_with("# First"));
    assert!(
        matches!(&values[0].fields.as_ref().unwrap()[&FieldName::new("body").unwrap()],FieldValue::Text{value,..} if value=="# Second\n\nSecond body\n")
    );
}

#[test]
fn markdown_targets_resolve_heading_fragments_and_percent_encoded_files() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/links.yml"),
        "id: links\nlanguage: Markdown\nrule: {kind: inline}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("owner.md"),
        "[found](target.md#hello-code) [missing](target.md#absent) [space](a%20b.md#spaced)\n",
    )
    .unwrap();
    std::fs::write(root.path().join("target.md"), "# Hello `code`\n").unwrap();
    std::fs::write(root.path().join("a b.md"), "# Spaced\n").unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &serde_json::from_value(json!({"markdown":{"heading_anchors":"github"}})).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({"links":{
        "rule":"links","fields":{"targets":{"from":"target","regex":"\\]\\(([^)]+)\\)","resolve":"path"}}
    }})).unwrap();
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
    assert_eq!(catalog.pieces.len(), 1);
    let field = &catalog.pieces[0].fields[&FieldName::new("targets").unwrap()];
    let values = match field {
        FieldValue::Targets { values } => values.as_slice(),
        _ => &[],
    };
    assert_eq!(values.len(), 3);
    assert!(
        matches!(&values[0].anchor, Some(crate::TargetAnchor::Found { fragment }) if fragment=="hello-code"),
        "{:?}",
        values[0]
    );
    assert!(
        matches!(&values[1].anchor, Some(crate::TargetAnchor::Missing { fragment }) if fragment=="absent"),
        "{:?}",
        values[1]
    );
    assert!(
        values[2].exists,
        "encoded Markdown path must resolve: {:?}",
        values[2]
    );
    assert!(
        matches!(&values[2].anchor, Some(crate::TargetAnchor::Found { fragment }) if fragment=="spaced")
    );
}

#[test]
fn unconfigured_markdown_targets_keep_explicit_ids_and_name_unsupported_headings() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/links.yml"),
        "id: links\nlanguage: Markdown\nrule: {kind: inline}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("owner.md"),
        "[heading](target.md#heading) [explicit](target.md#explicit) [other](target.txt#anchor)\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("target.md"),
        "# Heading\n\n<span id='explicit'></span>\n",
    )
    .unwrap();
    std::fs::write(root.path().join("target.txt"), "plain text\n").unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({"links":{
        "rule":"links","fields":{"targets":{"from":"target","regex":"\\]\\(([^)]+)\\)","resolve":"path"}}
    }})).unwrap();
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
    let FieldValue::Targets { values } =
        &catalog.pieces[0].fields[&FieldName::new("targets").unwrap()]
    else {
        panic!("target field must retain typed anchor outcomes");
    };
    assert_eq!(values.len(), 3);
    assert_eq!(
        values[0].anchor,
        Some(crate::TargetAnchor::Unsupported {
            fragment: "heading".into(),
            reason: crate::TargetAnchorUnsupportedReason::HeadingSchemeNotConfigured,
        })
    );
    assert_eq!(
        values[1].anchor,
        Some(crate::TargetAnchor::Found {
            fragment: "explicit".into()
        })
    );
    assert_eq!(
        values[2].anchor,
        Some(crate::TargetAnchor::Unsupported {
            fragment: "anchor".into(),
            reason: crate::TargetAnchorUnsupportedReason::UnsupportedDocument,
        })
    );
    assert!(values.iter().all(|target| target.exists));
}

#[test]
fn target_outline_and_ast_matches_share_the_local_match_allowance() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/whole.yml"),
        "id: whole\nlanguage: Html\nrule: {kind: document}\n",
    )
    .unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({"outline":{
        "html_units":{"mode":"unit","units":["section"]},
        "fields":{"body":{"from":"section","text":"full"}}
    }}))
    .unwrap();
    let html = "<h2>Target</h2><p>Original body</p>";
    for (max_matches, expected_available) in [(1, false), (2, true)] {
        let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
        let build = crate::EvidenceBuild::start(
            &project,
            &kinds,
            &[],
            &mut source,
            PieceBuildLimits {
                max_files: 10,
                max_file_bytes: 10000,
                max_total_bytes: 100000,
                max_matches,
                evidence_max_chars: 6000,
            },
        )
        .unwrap();

        let fields = build.path_target_fields(
            Path::new("target.html"),
            html,
            &EvidenceKindName::new("outline").unwrap(),
            None,
        );

        assert_eq!(
            fields.is_ok(),
            expected_available,
            "AST document plus outline section need two match slots"
        );
        if let Ok(Some(fields)) = fields {
            assert!(
                matches!(&fields[&FieldName::new("body").unwrap()], FieldValue::Text { value, .. } if value==html)
            );
        }
    }
}

#[test]
fn target_outline_budget_failure_is_unavailable_rather_than_normal_missing_fields() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/whole.yml"),
        "id: whole\nlanguage: Html\nrule: {kind: document}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("owner.html"),
        "<a href='target.html'>Read target</a>",
    )
    .unwrap();
    std::fs::write(
        root.path().join("target.html"),
        "<h2>Target</h2><p>Original body</p>",
    )
    .unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({
        "owner":{"rule":"whole","fields":{"targets":{"from":"target","regex":"href='([^']+)'","resolve":"path","as_kind":"outline"}}},
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{"body":{"from":"section","text":"full"}}}
    })).unwrap();
    for max_matches in [1, 2] {
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
                max_matches,
                evidence_max_chars: 6000,
            },
        );

        let owner = catalog
            .pieces
            .iter()
            .find(|piece| piece.kind.as_str() == "owner")
            .unwrap();
        let field = &owner.fields[&FieldName::new("targets").unwrap()];
        if max_matches == 1 {
            assert!(
                matches!(
                    field,
                    FieldValue::Unavailable {
                        reason: FieldUnavailable::Unreadable
                    }
                ),
                "uncollected target metadata must not look normally absent: {field:?}"
            );
        } else {
            assert!(
                matches!(field, FieldValue::Targets { values } if values.first().is_some_and(|target| target.fields.as_ref().is_some_and(|fields| matches!(&fields[&FieldName::new("body").unwrap()], FieldValue::Text { value, .. } if value.starts_with("<h2>Target"))))),
                "{field:?}"
            );
        }
    }
}
