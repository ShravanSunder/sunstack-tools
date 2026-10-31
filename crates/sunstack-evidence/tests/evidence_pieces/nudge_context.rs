use super::*;
use serde_json::json;

fn context_fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/pieces.yml"), "id: section\nlanguage: Markdown\nrule: {kind: section}\n---\nid: child\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^### Child', stopBy: neighbor}}\n---\nid: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n---\nid: document\nlanguage: Markdown\nrule: {kind: document}\n").unwrap();
    root
}

#[test]
fn target_exclusions_apply_to_both_scans_and_only_the_child_owns_its_load() {
    let root = context_fixture();
    std::fs::write(
        root.path().join("doc.md"),
        "# Title\n\n## Parent\n\nParent body.\n\n### Child\n\nRead `child.md`.\n",
    )
    .unwrap();
    std::fs::write(root.path().join("child.md"), "Loaded child.\n").unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
    // Every document piece excludes the only paragraph containing a load.
    let excluded = serde_json::from_value(json!({"document":{"rule":"document","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","exclude":{"rule":"paragraph"}}}}})).unwrap();
    let build = EvidenceBuild::start(
        &project,
        &excluded,
        &["doc.md".into()],
        &mut source,
        limits(),
    )
    .unwrap();
    assert_eq!(build.target_paths(&source, root.path()), BTreeMap::new());
    let kinds = serde_json::from_value(json!({"section":{"rule":"section","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","exclude":{"rule":"section"}}}}})).unwrap();
    let build =
        EvidenceBuild::start(&project, &kinds, &["doc.md".into()], &mut source, limits()).unwrap();
    assert_eq!(
        build.target_paths(&source, root.path()),
        BTreeMap::from([("doc.md".into(), vec!["child.md".into()])])
    );
    let catalog = build_pieces(&project, &kinds, &["doc.md".into()], &mut source, limits());
    assert!(catalog.failures.is_empty());
    assert_eq!(catalog.pieces.len(), 3);
    let changes = [CapturedChange {
        path: "child.md".into(),
        before: CapturedFileSide::Absent,
        after: CapturedFileSide::Absent,
        before_spans: vec![],
        after_spans: vec![],
    }];
    let (picked, outside) = pick_pieces(&catalog.pieces, &changes, false, root.path());
    assert_eq!(outside, 2);
    assert_eq!(
        picked
            .iter()
            .map(|index| catalog.pieces[*index].location.first_line)
            .collect::<Vec<_>>(),
        [7]
    );
    for piece in &catalog.pieces {
        let FieldValue::Targets { values } = field(piece, "targets") else {
            panic!()
        };
        if piece.location.first_line == 7 {
            assert_eq!(
                values
                    .iter()
                    .map(|target| target.path.as_path())
                    .collect::<Vec<_>>(),
                [Path::new("child.md")]
            );
        } else {
            assert!(values.is_empty(), "ancestor inherited the child's load");
        }
    }
}

#[test]
fn full_enclosing_and_same_file_text_preserve_the_whole_matched_span() {
    let root = context_fixture();
    let text = "# Title\n\n## Parent\n\nParent body.\n\n### Child\n\nChild body.\n";
    std::fs::write(root.path().join("doc.md"), text).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let kinds = serde_json::from_value(json!({"child":{"rule":"child","fields":{"enclosing":{"from":"enclosing","rule":"document","text":"full"},"same":{"from":"same_file","rule":"document","text":"full"},"heading":{"from":"enclosing","rule":"document"}}}})).unwrap();
    let catalog = build_pieces(
        &project,
        &kinds,
        &["doc.md".into()],
        &mut FilesystemCheckedSource::new(root.path()).unwrap(),
        limits(),
    );
    for name in ["enclosing", "same"] {
        let FieldValue::Texts { values, spans } = field(&catalog.pieces[0], name) else {
            panic!()
        };
        assert_eq!(values, &[text]);
        assert_eq!(
            (spans[0].byte_start, spans[0].byte_end, spans[0].start_line),
            (0, text.len(), 1)
        );
    }
    let FieldValue::Texts { values, .. } = field(&catalog.pieces[0], "heading") else {
        panic!()
    };
    assert_eq!(values, &["# Title"]);
}

#[test]
fn path_target_fields_and_opening_metadata_match_what_was_read() {
    let root = context_fixture();
    std::fs::write(root.path().join("doc.md"), "Read `target.md`.\n").unwrap();
    let target_text = "# Target\n\nDetails.\n";
    std::fs::write(root.path().join("target.md"), target_text).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    for (cap, opening, cut, last) in [
        (100, target_text, false, json!(3)),
        (10, "# Target\n\n", true, json!(2)),
        (0, "", true, json!(null)),
    ] {
        let kinds = serde_json::from_value(json!({"load":{"rule":"paragraph","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":cap,"as_kind":"document"}}},"document":{"rule":"document","fields":{"heading":{"from":"first_line"}}}})).unwrap();
        let mut source = RecordingSource::new(root.path());
        let catalog = build_pieces(&project, &kinds, &["doc.md".into()], &mut source, limits());
        let piece = catalog
            .pieces
            .iter()
            .find(|piece| piece.kind.as_str() == "load")
            .unwrap();
        let FieldValue::Targets { values } = field(piece, "targets") else {
            panic!()
        };
        let target = serde_json::to_value(&values[0]).unwrap();
        assert_eq!(target["opening"], opening);
        assert_eq!(target["opening_cut"], cut);
        assert_eq!(target["opening_last_line"], last);
        assert_eq!(target["fields"]["heading"]["value"], "# Target");
        let span = &target["fields"]["heading"]["spans"][0];
        assert_eq!(span["source"]["path"], "target.md");
        assert_eq!(
            (
                span["byte_start"].as_u64(),
                span["byte_end"].as_u64(),
                span["start_line"].as_u64()
            ),
            (Some(0), Some(9), Some(1))
        );
        assert_eq!(
            source.reads,
            [PathBuf::from("doc.md"), PathBuf::from("target.md")]
        );
        let model = serde_json::to_value(&piece.model_view).unwrap();
        assert_eq!(
            model["fields"]["targets"][0].get("opening_cut"),
            cut.then_some(&json!(true))
        );
    }
    std::fs::write(root.path().join("target.md"), "").unwrap();
    let kinds = serde_json::from_value(json!({"load":{"rule":"paragraph","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path"}}}})).unwrap();
    let catalog = build_pieces(
        &project,
        &kinds,
        &["doc.md".into()],
        &mut FilesystemCheckedSource::new(root.path()).unwrap(),
        limits(),
    );
    let FieldValue::Targets { values } = field(&catalog.pieces[0], "targets") else {
        panic!()
    };
    assert_eq!(
        serde_json::to_value(&values[0]).unwrap()["opening_last_line"],
        json!(null)
    );
}
