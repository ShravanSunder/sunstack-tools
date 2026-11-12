#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

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
    let project =
        crate::load_evidence_project(&root.path().join("sgconfig.yml"), &[], None).unwrap();
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
    let project =
        crate::load_evidence_project(&root.path().join("sgconfig.yml"), &[], None).unwrap();
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
