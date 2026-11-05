use super::*;

const INLINE_RULE: &str = "{language: Markdown, rule: {matches: frontmatter}}";

#[tokio::test]
async fn inline_rule_is_loaded_through_check_and_the_real_evidence_listing() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    let named = {
        let mut request = fixture::request(root.path());
        request.evidence_listing = Some(None);
        run(request, CancellationToken::new(), None).await
    };
    std::fs::remove_file(root.path().join("evidence/rules/skill.yml")).unwrap();
    config["evidence"]["kinds"]["skill"]
        .as_object_mut()
        .unwrap()
        .remove("rule");
    config["evidence"]["kinds"]["skill"]["inline_rule"] = json!(INLINE_RULE);
    fixture::config_file(root.path(), &config);

    let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
    assert!(matches!(
        check(&loaded, CancellationToken::new()).await,
        CheckResult::Valid { .. }
    ));
    let mut request = fixture::request(root.path());
    request.evidence_listing = Some(None);
    let inline = run(request, CancellationToken::new(), None).await;

    assert_ne!(named.status, LintStatus::Invalid, "{named:?}");
    assert_ne!(inline.status, LintStatus::Invalid, "{inline:?}");
    let named = serde_json::to_value(named.evidence_listing.unwrap()).unwrap();
    let inline = serde_json::to_value(inline.evidence_listing.unwrap()).unwrap();
    assert!(named.to_string().contains("Wrong opening"));
    assert_eq!(inline, named);
}

#[tokio::test]
async fn inline_rule_refusals_keep_the_second_kinds_config_key_and_location() {
    for bad_rule in [
        "{id: own, language: Markdown, rule: {matches: frontmatter}}",
        "{id: '', language: Markdown, rule: {matches: frontmatter}}",
        "{language: Markdown, rule: [",
        "{language: Markdown, rule: {kind: nonexistent_node}}",
        "language: Markdown\nrule: {matches: frontmatter}\n---\nlanguage: Markdown\nrule: {kind: paragraph}",
        "{language: Markdown, rule: {matches: frontmatter}, utils: {frontmatter: {kind: paragraph}}}",
        "{language: Rust, rule: {kind: source_file}}",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut value = fixture::write(root.path());
        value["evidence"]["kinds"]["a_valid"] = json!({"inline_rule":INLINE_RULE,"fields":{}});
        value["evidence"]["kinds"]["b_bad"] = json!({"inline_rule":bad_rule,"fields":{}});
        fixture::config_file(root.path(), &value);
        let location = discover_config(root.path(), None).unwrap();
        let text = std::fs::read_to_string(&location.file).unwrap();
        let owner = text.find("\"b_bad\":").unwrap();
        let position = owner + text[owner..].find("\"inline_rule\":").unwrap();
        let prefix = &text[..position];
        let expected_line =
            u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let expected_column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();

        let loaded = load_config(&location).unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!("accepted {bad_rule}")
        };

        assert_eq!(diagnostics.len(), 1, "{bad_rule}: {diagnostics:?}");
        assert_eq!(diagnostics[0].file.as_ref(), Some(&location.file));
        assert_eq!(
            (diagnostics[0].line, diagnostics[0].column),
            (Some(expected_line), Some(expected_column)),
            "{bad_rule}: {diagnostics:?}"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("evidence.kinds.b_bad.inline_rule"),
            "{diagnostics:?}"
        );
    }
}

#[tokio::test]
async fn matcher_wire_refuses_both_neither_null_and_unknown_keys() {
    for definition in [
        json!({"rule":"skill","inline_rule":INLINE_RULE,"fields":{}}),
        json!({"fields":{}}),
        json!({"inline_rule":null,"fields":{}}),
        json!({"rule":null,"fields":{}}),
        json!({"inline_rule":INLINE_RULE,"fields":{},"unexpected":true}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut value = fixture::write(root.path());
        value["evidence"]["kinds"]["b_bad"] = definition;
        fixture::config_file(root.path(), &value);
        let location = discover_config(root.path(), None).unwrap();
        let refusal = load_config(&location).unwrap_err();
        assert_eq!(refusal.file, location.file);
        assert!(
            refusal
                .json_path
                .join(".")
                .starts_with("evidence.kinds.b_bad"),
            "{refusal:?}"
        );
        assert!(refusal.line.is_some_and(|line| line > 1));
    }
}

#[tokio::test]
async fn inline_kind_cannot_reuse_a_named_rule_id_and_named_errors_keep_yaml_provenance() {
    let root = tempfile::tempdir().unwrap();
    let mut value = fixture::write(root.path());
    value["evidence"]["kinds"]["skill"]
        .as_object_mut()
        .unwrap()
        .remove("rule");
    value["evidence"]["kinds"]["skill"]["inline_rule"] = json!(INLINE_RULE);
    fixture::config_file(root.path(), &value);
    let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
    let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
    else {
        panic!()
    };
    assert!(
        diagnostics[0]
            .message
            .contains("duplicate evidence rule ID: skill")
    );
    assert!(
        diagnostics[0]
            .message
            .contains("evidence.kinds.skill.inline_rule")
    );

    let path = root.path().join("evidence/rules/skill.yml");
    std::fs::write(&path, "id: skill\nlanguage: Markdown\nrule: [\n").unwrap();
    let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
    else {
        panic!()
    };
    assert_eq!(
        diagnostics[0].file.as_ref(),
        Some(&path.canonicalize().unwrap())
    );
    assert!(diagnostics[0].line.is_some_and(|line| line >= 3));
    assert!(diagnostics[0].column.is_some());
}

#[test]
fn inline_rule_schema_and_wire_agree_on_exclusive_matchers_and_round_trip() {
    let schema = serde_json::to_value(config_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    for definition in [
        json!({"rule":"skill","fields":{}}),
        json!({"inline_rule":INLINE_RULE,"fields":{}}),
    ] {
        let kind: sunstack_evidence::EvidenceKindDefinition =
            serde_json::from_value(definition.clone()).unwrap();
        assert_eq!(serde_json::to_value(kind).unwrap(), definition);
        let mut config = baseline.clone();
        config["evidence"]["kinds"]["skill"] = definition;
        assert!(validator.is_valid(&config));
    }
    for definition in [
        json!({"fields":{}}),
        json!({"rule":"skill","inline_rule":INLINE_RULE,"fields":{}}),
        json!({"inline_rule":null,"fields":{}}),
        json!({"rule":null,"fields":{}}),
        json!({"inline_rule":INLINE_RULE,"fields":{},"unexpected":true}),
    ] {
        let mut config = baseline.clone();
        config["evidence"]["kinds"]["skill"] = definition;
        assert!(!validator.is_valid(&config), "{config}");
    }
}
