use super::*;

#[test]
fn added_context_keeps_checked_pieces_cache_and_cumulative_budgets() {
    for budget in ["sufficient", "files", "bytes", "matches"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        std::fs::write(
            root.path().join("rules/paragraph.yml"),
            "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
        )
        .unwrap();
        std::fs::write(root.path().join("checked.md"), "One.\n").unwrap();
        std::fs::write(root.path().join("context.md"), "Two.\n").unwrap();
        let project = load_evidence_project(&root.path().join("sgconfig.yml"), &[], None).unwrap();
        let kinds=serde_json::from_value(serde_json::json!({"paragraph":{"rule":"paragraph","fields":{"body":{"from":"first_line"}}}})).unwrap();
        let mut limits = limits();
        match budget {
            "files" => limits.max_files = 1,
            "bytes" => limits.max_total_bytes = 9,
            "matches" => limits.max_matches = 1,
            _ => {}
        }
        let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
        let mut build = EvidenceBuild::start(
            &project,
            &kinds,
            &["checked.md".into()],
            &mut source,
            limits,
        )
        .unwrap();
        let BuildStep::Done(checked) = build.advance(ChosenCandidates::default(), &mut source)
        else {
            panic!()
        };
        assert_eq!(checked.pieces.len(), 1);
        let catalog = build.add_context_files(&["context.md".into()], &mut source);
        assert_eq!(
            catalog
                .pieces
                .iter()
                .filter(|piece| piece.location.file == Path::new("checked.md"))
                .count(),
            1
        );
        if budget == "sufficient" {
            assert_eq!(catalog.pieces.len(), 2);
            assert!(catalog.failures.is_empty());
            assert_eq!(source.checked_files_acquired(), 2);
            let repeated = build.add_context_files(&["context.md".into()], &mut source);
            assert_eq!(repeated.pieces.len(), 2);
            assert_eq!(source.checked_files_acquired(), 2);
            assert!(
                repeated
                    .file_counts
                    .iter()
                    .all(|file| file.kinds.values().all(|count| count.pieces == 1))
            );
        } else {
            let reason = match budget {
                "files" => "max_files",
                "bytes" => "max_total_bytes",
                "matches" => "max_matches",
                _ => unreachable!(),
            };
            assert!(
                catalog
                    .failures
                    .iter()
                    .any(|failure| failure.file == Path::new("context.md")
                        && failure.reason.contains(reason)),
                "{budget}: {:?}",
                catalog.failures
            );
            assert_eq!(catalog.pieces.len(), 1);
        }
    }
}

#[test]
fn context_reuses_an_already_acquired_target_at_the_exact_file_and_byte_limits() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/paragraph.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    let checked = "Use `context.md`.\n";
    let context = "Cached context.\n";
    std::fs::write(root.path().join("checked.md"), checked).unwrap();
    std::fs::write(root.path().join("context.md"), context).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml"), &[], None).unwrap();
    let kinds=serde_json::from_value(serde_json::json!({"paragraph":{"rule":"paragraph","fields":{"body":{"from":"first_line"},"targets":{"from":"target","regex":"`([^`]+)`","resolve":"path"}}}})).unwrap();
    let mut limits = limits();
    limits.max_files = 2;
    limits.max_total_bytes = checked.len() + context.len();
    let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
    let mut build = EvidenceBuild::start(
        &project,
        &kinds,
        &["checked.md".into()],
        &mut source,
        limits,
    )
    .unwrap();
    let BuildStep::Done(first) = build.advance(ChosenCandidates::default(), &mut source) else {
        panic!()
    };
    assert!(first.failures.is_empty());
    assert_eq!(source.checked_files_acquired(), 2);
    let catalog = build.add_context_files(&["context.md".into()], &mut source);
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    assert_eq!(catalog.pieces.len(), 2);
    assert_eq!(source.checked_files_acquired(), 2);
}
