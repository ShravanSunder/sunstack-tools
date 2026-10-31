#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_evidence::*;

#[test]
fn neighbour_sources_decode_rule_only_with_bounded_default_count() {
    for from in ["next", "previous"] {
        let source: FieldSource =
            serde_json::from_value(serde_json::json!({"from":from,"rule":"paragraph"})).unwrap();
        assert_eq!(source.shape(), FieldShape::Texts);
        assert_eq!(serde_json::to_value(source).unwrap()["count"], 1);
        for value in [
            serde_json::json!(0),
            serde_json::json!(6),
            serde_json::json!(null),
        ] {
            assert!(
                serde_json::from_value::<FieldSource>(
                    serde_json::json!({"from":from,"rule":"paragraph","count":value})
                )
                .is_err()
            );
        }
        for keys in [
            serde_json::json!({"kind":"existing"}),
            serde_json::json!({"rule":"paragraph","kind":"existing"}),
        ] {
            let mut value = keys;
            value["from"] = from.into();
            assert!(serde_json::from_value::<FieldSource>(value).is_err());
        }
    }
}

use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/neighbours")
}
fn limits() -> PieceBuildLimits {
    PieceBuildLimits {
        max_files: 100,
        max_file_bytes: 100_000,
        max_total_bytes: 1_000_000,
        max_matches: 10_000,
        evidence_max_chars: 20_000,
    }
}
fn project(reverse: bool) -> (tempfile::TempDir, EvidenceProject, String, String) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("rules")).unwrap();
    std::fs::write(directory.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    let (list, item) = if reverse {
        ("z-list", "a-item")
    } else {
        ("a-list", "z-item")
    };
    for (id, kind) in [
        (list, "list"),
        (item, "list_item"),
        ("paragraph", "paragraph"),
        ("paragraph-copy", "paragraph"),
        ("fence", "fenced_code_block"),
        ("continuation", "block_continuation"),
    ] {
        std::fs::write(
            directory.path().join(format!("rules/{id}.yml")),
            format!("id: {id}\nlanguage: Markdown\nrule: {{kind: {kind}}}\n"),
        )
        .unwrap();
    }
    let project = load_evidence_project(&directory.path().join("sgconfig.yml")).unwrap();
    (directory, project, list.into(), item.into())
}
fn build(
    project: &EvidenceProject,
    file: &str,
    rule: &str,
    neighbour_rule: &str,
    direction: &str,
    count: u8,
    build_limits: PieceBuildLimits,
) -> EvidenceCatalog {
    let definition:EvidenceKindDefinition=serde_json::from_value(json!({"rule":rule,"fields":{"neighbours":{"from":direction,"rule":neighbour_rule,"count":count},"remainder":{"from":"remainder"}}})).unwrap();
    let kinds = BTreeMap::from([(EvidenceKindName::new("subject").unwrap(), definition)]);
    let mut source = FilesystemCheckedSource::new(&fixtures()).unwrap();
    let catalog = build_pieces(project, &kinds, &[file.into()], &mut source, build_limits);
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    catalog
}
fn at_line(catalog: &EvidenceCatalog, line: usize) -> &EvidencePiece {
    catalog
        .pieces
        .iter()
        .find(|piece| piece.location.first_line == line)
        .unwrap()
}
fn neighbours(piece: &EvidencePiece) -> (&[String], &[SourceSpan]) {
    match &piece.fields[&FieldName::new("neighbours").unwrap()] {
        FieldValue::Texts { values, spans } => (values, spans),
        other => panic!("{other:?}"),
    }
}
fn oracle(file: &str, kind: &str, line: usize) -> serde_json::Value {
    let data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("scan-oracle.json")).unwrap(),
    )
    .unwrap();
    let text = std::fs::read_to_string(fixtures().join(file)).unwrap();
    data["matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| {
            m["file"] == file
                && m["kind"] == kind
                && text[..m["start"].as_u64().unwrap() as usize]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1
                    == line
        })
        .unwrap()
        .clone()
}
fn assert_oracles(piece: &EvidencePiece, expected: &[serde_json::Value]) {
    let (values, spans) = neighbours(piece);
    assert_eq!(
        values,
        expected
            .iter()
            .map(|m| m["text"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>(),
        "{piece:?}"
    );
    assert_eq!(
        spans
            .iter()
            .map(|s| (s.byte_start, s.byte_end))
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|m| (
                m["start"].as_u64().unwrap() as usize,
                m["end"].as_u64().unwrap() as usize
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn copied_skills_have_exact_neighbours_and_previous_two_stay_in_source_order() {
    let (_directory, project, list, item) = project(false);
    let catalog = build(
        &project,
        "scaffold-project.md",
        "paragraph",
        &list,
        "next",
        1,
        limits(),
    );
    assert_oracles(
        at_line(&catalog, 161),
        &[oracle("scaffold-project.md", "list", 163)],
    );
    let catalog = build(
        &project,
        "implementation-pr-wrapup.md",
        "paragraph",
        "fence",
        "next",
        1,
        limits(),
    );
    assert_oracles(
        at_line(&catalog, 43),
        &[oracle(
            "implementation-pr-wrapup.md",
            "fenced_code_block",
            44,
        )],
    );
    let catalog = build(
        &project,
        "descriptions-and-invocation.md",
        &item,
        &item,
        "previous",
        2,
        limits(),
    );
    assert_oracles(
        at_line(&catalog, 15),
        &[
            oracle("descriptions-and-invocation.md", "list_item", 13),
            oracle("descriptions-and-invocation.md", "list_item", 14),
        ],
    );
    assert!(neighbours(at_line(&catalog, 15)).0[0].contains("aims for 100 to 200 characters"));
}

#[test]
fn neighbour_walk_never_crosses_parent_or_nonmatching_blocks() {
    let (_directory, project, _list, item) = project(false);
    for (file, line) in [
        ("last-item.md", 2),
        ("nested-space.md", 3),
        ("nested-tab.md", 3),
        ("separate-lists.md", 1),
        ("intervening-paragraph.md", 1),
    ] {
        let catalog = build(&project, file, &item, &item, "next", 5, limits());
        assert_oracles(at_line(&catalog, line), &[]);
    }
    let catalog = build(
        &project,
        "intervening-paragraph.md",
        &_list,
        &_list,
        "next",
        5,
        limits(),
    );
    assert_oracles(&catalog.pieces[0], &[]);
    // Section-level blank lines do not form a barrier between the two list items.
    let catalog = build(&project, "last-item.md", &item, &item, "next", 1, limits());
    assert_oracles(
        at_line(&catalog, 1),
        &[oracle("last-item.md", "list_item", 2)],
    );
}

#[test]
fn continuation_piece_retains_its_position_but_continuations_are_not_values_or_hops() {
    let (_directory, project, _, _) = project(false);
    let file = "continuation.md";
    for rule in ["paragraph", "continuation"] {
        let catalog = build(&project, file, rule, "fence", "next", 1, limits());
        let piece = if rule == "continuation" {
            catalog
                .pieces
                .iter()
                .find(|p| p.location.span.byte_start == 43 && p.location.span.byte_end == 46)
                .unwrap()
        } else {
            at_line(&catalog, 1)
        };
        let index = &catalog.matches[Path::new(file)];
        let position = index.by_rule[rule]
            .iter()
            .position(|r| {
                r.start == piece.location.span.byte_start && r.end == piece.location.span.byte_end
            })
            .unwrap();
        let node = index.match_nodes[rule][position];
        let parent = &index.parents[&node];
        assert_eq!(parent.children[parent.position].node, node, "{index:#?}");
        assert_eq!(
            parent.children[parent.position].continuation,
            rule == "continuation",
            "{index:#?}"
        );
        assert_oracles(piece, &[oracle(file, "fenced_code_block", 3)]);
    }
}

#[test]
fn same_range_nodes_keep_distinct_parents_in_both_actual_rule_orders() {
    for reverse in [false, true] {
        let (_directory, project, list, item) = project(reverse);
        let order = project
            .rules
            .for_path("same-range.md")
            .into_iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            order.iter().position(|r| r == &list).unwrap()
                < order.iter().position(|r| r == &item).unwrap(),
            !reverse,
            "{order:?}"
        );
        let list_catalog = build(
            &project,
            "same-range.md",
            &list,
            "paragraph",
            "next",
            1,
            limits(),
        );
        let item_catalog = build(
            &project,
            "same-range.md",
            &item,
            "paragraph",
            "next",
            1,
            limits(),
        );
        let index = &list_catalog.matches[Path::new("same-range.md")];
        assert_eq!(
            index.by_rule[list.as_str()][0],
            MatchRange { start: 0, end: 37 }
        );
        assert_eq!(
            index.by_rule[item.as_str()][0],
            MatchRange { start: 0, end: 37 }
        );
        let list_node = index.match_nodes[list.as_str()][0];
        let item_node = index.match_nodes[item.as_str()][0];
        assert_ne!(list_node, item_node, "{order:?} {index:#?}");
        assert_ne!(
            index.parents[&list_node].parent, index.parents[&item_node].parent,
            "{order:?} {index:#?}"
        );
        assert_eq!(
            index.match_nodes["paragraph"], index.match_nodes["paragraph-copy"],
            "node identity unstable across rules: {index:#?}"
        );
        assert_oracles(
            &list_catalog.pieces[0],
            &[oracle("same-range.md", "paragraph", 6)],
        );
        assert_oracles(&item_catalog.pieces[0], &[]);
    }
}

#[test]
fn neighbour_lists_use_existing_model_size_cut_without_changing_raw_fields_or_remainder() {
    let (_directory, project, list, _) = project(false);
    let catalog = build(
        &project,
        "oversize.md",
        "paragraph",
        &list,
        "next",
        1,
        PieceBuildLimits {
            evidence_max_chars: 300,
            ..limits()
        },
    );
    let piece = &catalog.pieces[0];
    assert_oracles(piece, &[oracle("oversize.md", "list", 3)]);
    assert_eq!(
        piece.fields[&FieldName::new("remainder").unwrap()],
        FieldValue::Text {
            value: "Before.\n".into(),
            spans: vec![piece.location.span.clone()]
        }
    );
    match &piece.model_view {
        ModelView::Bounded { fields, cut } => {
            assert!(cut.contains(&FieldName::new("neighbours").unwrap()));
            assert!(fields.values().map(model_value_chars).sum::<usize>() <= 300);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn new_structural_index_preserves_native_scan_ranges_and_match_limits() {
    let (_directory, project, list, item) = project(false);
    let data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("scan-oracle.json")).unwrap(),
    )
    .unwrap();
    for file in std::fs::read_dir(fixtures())
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
    {
        let text = std::fs::read_to_string(&file).unwrap();
        let name = file.file_name().unwrap().to_str().unwrap();
        let index = project.match_file(Path::new(name), &text, usize::MAX);
        for (id, kind) in [
            (list.as_str(), "list"),
            (item.as_str(), "list_item"),
            ("paragraph", "paragraph"),
            ("fence", "fenced_code_block"),
            ("continuation", "block_continuation"),
        ] {
            let mut expected = data["matches"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["file"] == name && m["kind"] == kind)
                .map(|m| MatchRange {
                    start: m["start"].as_u64().unwrap() as usize,
                    end: m["end"].as_u64().unwrap() as usize,
                })
                .collect::<Vec<_>>();
            expected.sort_by_key(|m| (m.start, m.end));
            assert_eq!(index.by_rule[id], expected, "{name}:{id}");
            assert_eq!(index.by_rule[id].len(), index.match_nodes[id].len());
        }
        let limited = project.match_file(Path::new(name), &text, 2);
        assert!(limited.limited);
        assert_eq!(limited.by_rule.values().map(Vec::len).sum::<usize>(), 2);
    }
}
