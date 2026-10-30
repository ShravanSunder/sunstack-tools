#![allow(clippy::unwrap_used, clippy::panic)]
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_evidence::*;
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sunstack/tests/fixtures/rulebook-lint/selection")
}
#[test]
fn numbered_h2_stages_are_cut_by_the_embedded_util() {
    let root = fixture();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let text = std::fs::read_to_string(root.join("skills/spec-program-review/SKILL.md")).unwrap();
    let index = project.match_file(std::path::Path::new("SKILL.md"), &text, 10000);
    let stages = index.by_rule[&EvidenceRuleId::new("step").unwrap()]
        .iter()
        .filter(|range| text[range.start..range.end].starts_with("## "))
        .count();
    assert_eq!(stages, 9);
}
#[test]
fn forward_named_as_kind_with_a_plain_kind_field_has_its_final_fields() {
    let root = fixture();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let kinds:BTreeMap<EvidenceKindName,EvidenceKindDefinition>=serde_json::from_value(serde_json::json!({
 "a_load":{"rule":"load","fields":{"targets":{"from":"target","regex":"`([^`]+)`","resolve":"skill","as_kind":"z_skill"}}},
 "plain_heading":{"rule":"heading","fields":{"heading":{"from":"first_line"}}},
 "z_skill":{"rule":"skill","fields":{"headings":{"from":"same_file","kind":"plain_heading"}}}
 })).unwrap();
    let target_root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(target_root.path().join("target")).unwrap();
    std::fs::write(target_root.path().join("load.md"), "load `target`.\n").unwrap();
    std::fs::write(
        target_root.path().join("target/SKILL.md"),
        "---\ndescription: Target\n---\n\n## Final heading\n",
    )
    .unwrap();
    let mut source = FilesystemCheckedSource::new(target_root.path()).unwrap();
    let catalog = build_pieces(
        &project,
        &kinds,
        &["load.md".into(), "target/SKILL.md".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 2000,
            max_file_bytes: 100000,
            max_total_bytes: 1000000,
            max_matches: 10000,
            evidence_max_chars: 6000,
        },
    );
    assert!(catalog.failures.is_empty());
    let load = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "a_load")
        .unwrap();
    let FieldValue::Targets { values } = &load.fields[&FieldName::new("targets").unwrap()] else {
        panic!()
    };
    let target = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "z_skill")
        .unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].fields.as_ref(), Some(&target.fields));
    let FieldValue::Texts { values, .. } = &target.fields[&FieldName::new("headings").unwrap()]
    else {
        panic!()
    };
    assert!(values.iter().any(|line| line == "## Final heading"));
}
fn limits() -> PieceBuildLimits {
    PieceBuildLimits {
        max_files: 2000,
        max_file_bytes: 100000,
        max_total_bytes: 1000000,
        max_matches: 10000,
        evidence_max_chars: 6000,
    }
}
#[test]
fn staged_selection_filters_steps_and_preserves_shared_target_free_views_and_counts() {
    let root = fixture();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let files = [
        "skills/practices-research/SKILL.md".into(),
        "skills/presentation-tui/SKILL.md".into(),
        "skills/no-list/SKILL.md".into(),
        "skills/thirty-three/SKILL.md".into(),
    ];
    let mut source = FilesystemCheckedSource::new(&root).unwrap();
    let mut build = EvidenceBuild::start(&project, &kinds, &files, &mut source, limits()).unwrap();
    let mut chosen = ChosenCandidates::default();
    let mut observed = Vec::new();
    let catalog = loop {
        match build.advance(chosen, &mut source) {
            BuildStep::Done(catalog) => break catalog,
            BuildStep::NeedsSelection {
                kind,
                candidates,
                shared,
            } => {
                assert!(candidates.iter().all(|candidate| {
                    candidate
                        .fields
                        .keys()
                        .all(|field| !field.as_str().ends_with("targets"))
                }));
                for candidate in &candidates {
                    let ModelView::Bounded { fields, .. } = &candidate.selection_view else {
                        panic!()
                    };
                    assert!(!fields.contains_key(&FieldName::new("outline").unwrap()));
                    assert!(fields.values().map(model_value_chars).sum::<usize>() <= 6000);
                }
                assert_eq!(shared.len(), 3); // no-list has no candidates
                assert!(shared.iter().all(|shared| matches!(&shared.view,ModelView::Bounded{fields,..} if fields.contains_key(&FieldName::new("outline").unwrap()))));
                chosen = ChosenCandidates(
                    candidates
                        .iter()
                        .filter(|candidate| {
                            kind.as_str() == "workflow_section"
                                && candidate
                                    .location
                                    .file
                                    .to_string_lossy()
                                    .contains("practices-research")
                        })
                        .map(|candidate| candidate.id.clone())
                        .collect(),
                );
                observed.push(kind);
            }
        }
    };
    assert_eq!(observed.len(), 2);
    let steps = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "step")
        .collect::<Vec<_>>();
    assert_eq!(
        steps
            .iter()
            .map(|piece| piece.location.first_line)
            .collect::<Vec<_>>(),
        [21, 22, 24, 25, 26, 27]
    );
    let research = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "workflow_section")
        .unwrap();
    assert!(research.text.starts_with("## Ordered Workflow"));
    let FieldValue::Text { value, spans } = &research.fields[&FieldName::new("body").unwrap()]
    else {
        panic!()
    };
    let original =
        std::fs::read_to_string(root.join("skills/practices-research/SKILL.md")).unwrap();
    let expected: String = original.split_inclusive('\n').skip(19).take(11).collect();
    assert_eq!(value, &expected);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].start_line, 20);
    assert_eq!(
        spans[0].byte_end,
        original
            .split_inclusive('\n')
            .take(30)
            .map(str::len)
            .sum::<usize>()
    );
    assert!(!value.contains("## Ordered Workflow"));
    let empty = catalog
        .file_counts
        .iter()
        .find(|count| count.file.to_string_lossy().contains("no-list"))
        .unwrap();
    let count = &empty.kinds[&EvidenceKindName::new("workflow_section").unwrap()];
    assert_eq!(count.selection.as_ref().unwrap().candidates, 0);
    assert_eq!(catalog.file_counts.len(), 4);
    assert!(catalog.pieces.windows(2).all(|pair| (
        &pair[0].kind,
        &pair[0].location.file,
        pair[0].location.span.byte_start
    ) <= (
        &pair[1].kind,
        &pair[1].location.file,
        pair[1].location.span.byte_start
    )));
}
#[test]
fn contained_kind_fields_read_only_final_pieces_and_remainder_excludes_strict_children() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/pieces.yml"),"id: section\nlanguage: Markdown\nrule: {kind: section}\n---\nid: parent\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^## Parent', stopBy: neighbor}}\n---\nid: child\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^### Chosen', stopBy: neighbor}}\n---\nid: item\nlanguage: Markdown\nrule: {kind: list_item}\n").unwrap();
    std::fs::write(root.path().join("doc.md"),"# Title\n\n## Parent\n\n1. Outer.\n\n### Chosen\n\n1. First.\n2. Second.\n\n### Other\n\n1. Third.\n").unwrap();
    let kinds=serde_json::from_value(serde_json::json!({
 "chosen":{"rule":"child","select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3},"fields":{"heading":{"from":"first_line"}}},
 "step":{"rule":"item","within":"chosen","fields":{"body":{"from":"remainder"}}},
 "parent":{"rule":"parent","fields":{"heading":{"from":"first_line"},"body":{"from":"remainder","exclude":{"rule":"section"}},"steps":{"from":"inside","kind":"step"},"chosen":{"from":"same_file","kind":"chosen"}}}
 })).unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
    let mut build =
        EvidenceBuild::start(&project, &kinds, &["doc.md".into()], &mut source, limits()).unwrap();
    let BuildStep::NeedsSelection { candidates, .. } =
        build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!()
    };
    let chosen = ChosenCandidates(
        candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect(),
    );
    let BuildStep::Done(catalog) = build.advance(chosen, &mut source) else {
        panic!()
    };
    let parent = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "parent")
        .unwrap();
    let FieldValue::Texts { values, .. } = &parent.fields[&FieldName::new("steps").unwrap()] else {
        panic!()
    };
    assert_eq!(
        values,
        &["1. First.\n".to_string(), "2. Second.\n\n".to_string()]
    );
    let FieldValue::Text { value, .. } = &parent.fields[&FieldName::new("body").unwrap()] else {
        panic!()
    };
    assert_eq!(value, "\n1. Outer.\n\n");
    let FieldValue::Texts { values, .. } = &parent.fields[&FieldName::new("chosen").unwrap()]
    else {
        panic!()
    };
    assert_eq!(values, &["### Chosen".to_string()]);
}
#[test]
fn heading_utils_accept_h2_h3_h4_and_reject_document_titles() {
    for (id, heading) in [
        ("numbered-heading-section", "2. Inspect"),
        ("step-heading-section", "Step 2: Inspect"),
    ] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        std::fs::write(
            root.path().join("rules/test.yml"),
            format!("id: test\nlanguage: Markdown\nrule: {{matches: {id}}}\n"),
        )
        .unwrap();
        let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
        for level in ["#", "##", "###", "####", "######"] {
            let text = format!("{level} {heading}\n\nBody.\n");
            assert_eq!(
                project
                    .match_file(std::path::Path::new("doc.md"), &text, 10)
                    .by_rule["test"]
                    .len(),
                usize::from(level != "#")
            );
        }
    }
}
