use super::*;

const SECTION_RULE: &str = "{language: Markdown, rule: {all: [{kind: section}, {has: {matches: selected_heading, stopBy: neighbor}}]}, utils: {selected_heading: {kind: atx_heading, regex: '^## Chosen'}}}";

fn inline_inputs(kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>) -> Vec<InlineRule> {
    kinds
        .iter()
        .filter_map(|(kind, definition)| match &definition.matcher {
            KindMatcher::Named(_) | KindMatcher::HtmlUnits(_) => None,
            KindMatcher::Inline { text } => Some(InlineRule {
                kind: kind.clone(),
                text: text.clone(),
                origin: ConfigPath::keys(["evidence", "kinds", kind.as_str(), "inline_rule"]),
            }),
        })
        .collect()
}

fn catalog(
    root: &std::path::Path,
    project: &EvidenceProject,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
) -> EvidenceCatalog {
    let mut source = FilesystemCheckedSource::new(root).unwrap();
    build_pieces(project, kinds, &["doc.md".into()], &mut source, limits())
}

#[test]
fn inline_rule_parity_preserves_effective_utilities_enclosing_within_and_input_bytes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    let project_path = root.path().join("sgconfig.yml");
    let config_path = root.path().join("sunstack.lint.json");
    std::fs::write(&project_path, "ruleDirs: [rules]\n").unwrap();
    let named_rule_path = root.path().join("rules/parent.yml");
    // Use the exact same YAML mapping for the named rule, including its local utility.
    std::fs::write(
        &named_rule_path,
        format!("{{id: parent, {}", SECTION_RULE.trim_start_matches('{')),
    )
    .unwrap();
    std::fs::write(
        root.path().join("rules/item.yml"),
        "id: item\nlanguage: Markdown\nrule: {kind: list_item}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("doc.md"),
        "# Doc\n\n## Chosen\n\n1. First.\n2. Second.\n\n## Decoy\n\n1. Outside.\n",
    )
    .unwrap();
    let mut config = serde_json::json!({
        "parent":{"rule":"parent","fields":{"heading":{"from":"first_line"}}},
        "child":{"rule":"item","within":["parent"],"fields":{"heading":{"from":"enclosing","kind":"parent"},"body":{"from":"first_line"}}}
    });
    let named_kinds = serde_json::from_value(config.clone()).unwrap();
    let named_project = load_evidence_project(&project_path, &[], None).unwrap();
    let named = catalog(root.path(), &named_project, &named_kinds);
    assert!(named.failures.is_empty(), "{:?}", named.failures);
    assert_eq!(named.pieces.len(), 3);
    assert_eq!(
        named
            .pieces
            .iter()
            .filter(|piece| piece.kind.as_str() == "parent")
            .count(),
        1
    );
    let children = named
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "child")
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 2);
    assert_eq!(
        children
            .iter()
            .map(|piece| piece.fields[&FieldName::new("body").unwrap()].clone())
            .collect::<Vec<_>>()
            .iter()
            .map(|value| match value {
                FieldValue::Text { value, .. } => value.as_str(),
                _ => panic!(),
            })
            .collect::<Vec<_>>(),
        ["1. First.", "2. Second."]
    );
    for child in children {
        let FieldValue::Texts { values, .. } = &child.fields[&FieldName::new("heading").unwrap()]
        else {
            panic!()
        };
        assert_eq!(values, &["## Chosen"]);
    }

    std::fs::remove_file(named_rule_path).unwrap();
    config["parent"].as_object_mut().unwrap().remove("rule");
    config["parent"]["inline_rule"] = serde_json::json!(SECTION_RULE);
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let inline_kinds = serde_json::from_value(config.clone()).unwrap();
    let inline_project = load_evidence_project(
        &project_path,
        &inline_inputs(&inline_kinds),
        Some(&config_path),
    )
    .unwrap();
    let inline = catalog(root.path(), &inline_project, &inline_kinds);

    assert!(inline.failures.is_empty(), "{:?}", inline.failures);
    assert_eq!(
        serde_json::to_vec(&inline.pieces).unwrap(),
        serde_json::to_vec(&named.pieces).unwrap()
    );
    assert!(inline_project.inputs.contains(&config_path));
    let before = inline_project
        .inputs
        .iter()
        .map(|path| (path.clone(), std::fs::read(path).unwrap()))
        .collect::<Vec<_>>();

    config["parent"]["inline_rule"] = serde_json::json!(SECTION_RULE.replace("Chosen", "Decoy"));
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let changed_kinds = serde_json::from_value(config).unwrap();
    let changed_project = load_evidence_project(
        &project_path,
        &inline_inputs(&changed_kinds),
        Some(&config_path),
    )
    .unwrap();
    let after = changed_project
        .inputs
        .iter()
        .map(|path| (path.clone(), std::fs::read(path).unwrap()))
        .collect::<Vec<_>>();
    let changed = catalog(root.path(), &changed_project, &changed_kinds);
    assert_ne!(before, after);
    assert!(changed.failures.is_empty());
    assert_eq!(changed.pieces.len(), 2);
    let parent = changed
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "parent")
        .unwrap();
    assert!(parent.text.starts_with("## Decoy"));
    let child = changed
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "child")
        .unwrap();
    assert!(child.text.starts_with("1. Outside."));
}
