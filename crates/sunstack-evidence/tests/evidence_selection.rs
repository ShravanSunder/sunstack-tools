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
 "step":{"rule":"item","within":["chosen"],"fields":{"body":{"from":"remainder"}}},
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

#[test]
fn remainder_exclusions_are_local_to_each_field_and_its_spans() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("rules/pieces.yml"), "id: section\nlanguage: Markdown\nrule: {kind: section}\n---\nid: child\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^### Child', stopBy: neighbor}}\n---\nid: other\nlanguage: Markdown\nrule: {kind: section, has: {kind: atx_heading, regex: '^### Other', stopBy: neighbor}}\n").unwrap();
    let text = "# Doc\n\n## Parent\n\nParent body.\n\n### Child\n\nChild body.\n\n### Other\n\nOther body.\n";
    std::fs::write(root.path().join("doc.md"), text).unwrap();
    let kinds = serde_json::from_value(serde_json::json!({
        "section":{"rule":"section","fields":{
            "heading":{"from":"first_line"},
            "body":{"from":"remainder"},
            "without_child":{"from":"remainder","exclude":{"rule":"child"}},
            "without_other":{"from":"remainder","exclude":{"rule":"other"}}
        }}
    }))
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
    let catalog = build_pieces(&project, &kinds, &["doc.md".into()], &mut source, limits());
    let parent = catalog
        .pieces
        .iter()
        .find(|piece| piece.text.starts_with("## Parent"))
        .unwrap();
    let body_start = text.find("\nParent body.").unwrap();
    let child_start = text.find("### Child").unwrap();
    let other_start = text.find("### Other").unwrap();
    for (name, expected, ranges) in [
        (
            "body",
            "\nParent body.\n\n### Child\n\nChild body.\n\n### Other\n\nOther body.\n",
            vec![(body_start, text.len())],
        ),
        (
            "without_child",
            "\nParent body.\n\n### Other\n\nOther body.\n",
            vec![(body_start, child_start), (other_start, text.len())],
        ),
        (
            "without_other",
            "\nParent body.\n\n### Child\n\nChild body.\n\n",
            vec![(body_start, other_start)],
        ),
    ] {
        let FieldValue::Text { value, spans } = &parent.fields[&FieldName::new(name).unwrap()]
        else {
            panic!()
        };
        assert_eq!(value, expected, "{name}");
        assert_eq!(
            spans
                .iter()
                .map(|span| (span.byte_start, span.byte_end))
                .collect::<Vec<_>>(),
            ranges,
            "{name}"
        );
        assert_eq!(
            spans
                .iter()
                .map(|span| &text[span.byte_start..span.byte_end])
                .collect::<String>(),
            expected
        );
    }
}

#[test]
fn selected_candidates_coalesce_equal_ranges_without_changing_plain_kinds() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/pieces.yml"),
        "id: duplicate\nlanguage: Markdown\nrule: {any: [{kind: document}, {kind: paragraph}]}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("doc.md"), "Hello\n").unwrap();
    let kinds = serde_json::from_value(serde_json::json!({
        "plain":{"rule":"duplicate","fields":{"body":{"from":"remainder"}}},
        "selected":{"rule":"duplicate","fields":{"body":{"from":"remainder"}},"select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3}}
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
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].key.as_str(), "c1");
    let BuildStep::Done(catalog) = build.advance(
        ChosenCandidates(
            candidates
                .iter()
                .map(|candidate| candidate.id.clone())
                .collect(),
        ),
        &mut source,
    ) else {
        panic!()
    };
    assert_eq!(
        catalog
            .pieces
            .iter()
            .filter(|piece| piece.kind.as_str() == "plain")
            .count(),
        2
    );
    assert_eq!(
        catalog
            .pieces
            .iter()
            .filter(|piece| piece.kind.as_str() == "selected")
            .count(),
        1
    );
    assert_eq!(
        catalog.file_counts[0].kinds[&EvidenceKindName::new("selected").unwrap()]
            .selection
            .as_ref()
            .unwrap()
            .candidates,
        1
    );
}

#[test]
fn containing_kinds_union_keeps_each_match_once_and_only_chosen_steps() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/union.yml"),
        r#"id: workflow
language: Markdown
rule: {kind: section, has: {kind: atx_heading, regex: '^## Workflow', stopBy: neighbor}}
---
id: standards
language: Markdown
rule: {kind: section, has: {kind: atx_heading, regex: '^###? Standards', stopBy: neighbor}}
---
id: item
language: Markdown
rule: {kind: list_item}
---
id: sentence
language: Markdown
rule: {kind: paragraph}
"#,
    )
    .unwrap();
    std::fs::write(
        root.path().join("skill.md"),
        "# Skill\n\nOutside skill.\n\n## Workflow\n\n1. Step only.\n\n### Standards\n\n1. Both.\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("reference.md"),
        "# Reference\n\n## Standards\n\nStandards only.\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    for select_sentences in [false, true] {
        for choose_workflow in [false, true] {
            let mut definitions = serde_json::json!({
                "workflow_section":{"rule":"workflow","select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3},"fields":{}},
                "step":{"rule":"item","within":["workflow_section"],"fields":{}},
                "standards_section":{"rule":"standards","fields":{}},
                "sentence":{"rule":"sentence","within":["step","standards_section"],"fields":{}}
            });
            if select_sentences {
                definitions["sentence"]["select"] =
                    serde_json::json!({"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3});
            }
            let kinds = serde_json::from_value(definitions).unwrap();
            let mut source = FilesystemCheckedSource::new(root.path()).unwrap();
            let mut build = EvidenceBuild::start(
                &project,
                &kinds,
                &["skill.md".into(), "reference.md".into()],
                &mut source,
                limits(),
            )
            .unwrap();
            let mut chosen = ChosenCandidates::default();
            let mut sentence_candidates = Vec::new();
            let catalog = loop {
                match build.advance(chosen, &mut source) {
                    BuildStep::Done(catalog) => break catalog,
                    BuildStep::NeedsSelection {
                        kind, candidates, ..
                    } => {
                        if kind.as_str() == "sentence" {
                            sentence_candidates = candidates
                                .iter()
                                .map(|candidate| candidate.first_line.clone())
                                .collect();
                        }
                        chosen = ChosenCandidates(
                            candidates
                                .iter()
                                .filter(|_| kind.as_str() == "sentence" || choose_workflow)
                                .map(|candidate| candidate.id.clone())
                                .collect(),
                        );
                    }
                }
            };
            assert!(catalog.failures.is_empty());
            let mut sentences = catalog
                .pieces
                .iter()
                .filter(|piece| piece.kind.as_str() == "sentence")
                .map(|piece| piece.text.trim().to_owned())
                .collect::<Vec<_>>();
            sentences.sort();
            let expected = if choose_workflow {
                vec!["Both.", "Standards only.", "Step only."]
            } else {
                vec!["Both.", "Standards only."]
            };
            assert_eq!(
                sentences, expected,
                "selected sentence={select_sentences}, chosen workflow={choose_workflow}"
            );
            require_union_file_counts(&catalog.file_counts, choose_workflow);
            let (picked, _) = pick_pieces(&catalog.pieces, &[], true, root.path());
            assert_eq!(
                picked
                    .iter()
                    .filter(|index| catalog.pieces[**index].kind.as_str() == "sentence")
                    .count(),
                if choose_workflow { 3 } else { 2 }
            );
            let (unpicked, _) = pick_pieces(&catalog.pieces, &[], false, root.path());
            assert!(unpicked.is_empty());
            let mut corrupted_counts = catalog.file_counts.clone();
            corrupted_counts[0]
                .kinds
                .get_mut(&EvidenceKindName::new("sentence").unwrap())
                .unwrap()
                .matches += 1;
            assert!(
                std::panic::catch_unwind(|| require_union_file_counts(
                    &corrupted_counts,
                    choose_workflow
                ))
                .is_err(),
                "counter-only corruption must fail while catalog pieces remain untouched"
            );
            if select_sentences {
                sentence_candidates.sort();
                assert_eq!(sentence_candidates, expected);
            }
        }
    }
}

fn require_union_file_counts(counts: &[FileCount], choose_workflow: bool) {
    let sentence = EvidenceKindName::new("sentence").unwrap();
    assert_eq!(counts.len(), 2);
    let actual = counts
        .iter()
        .map(|entry| {
            let count = &entry.kinds[&sentence];
            (
                entry.file.to_string_lossy().into_owned(),
                count.matches,
                count.pieces,
                count.picked,
                count.limited,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            ("reference.md".into(), 1, 1, 0, false),
            (
                "skill.md".into(),
                3,
                if choose_workflow { 2 } else { 1 },
                0,
                false
            )
        ]
    );
}
