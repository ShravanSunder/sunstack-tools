#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_rules::*;
#[path = "support/change_fixture.rs"]
mod changes;
#[path = "support/contract_schemas.rs"]
mod schema_support;

fn check_input(root: &std::path::Path) -> CheckRulesInput {
    serde_json::from_value(json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"skill":{"kind":"document","repository":"repo","path":"unread.md"}}}]}]},"repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"limits":{"max_files":10,"max_file_bytes":65536,"max_total_bytes":131072,"max_matches":20,"max_transcript_bytes":65536,"include_ignored":false},"output":"assembled_rules"})).unwrap()
}
fn write_source(root: &std::path::Path, entry: &str) {
    std::fs::write(root.join("set.json"),serde_json::to_vec(&json!({"schema_version":1,"name":"mechanical","description":"native check","inputs":{"skill":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_sources":[{"markdown":"standards.md","rules":"standards.rules.json"}],"marker_prefix":"K"})).unwrap()).unwrap();
    std::fs::write(
        root.join("standards.md"),
        "Keep this explanation `verbatim`.<!-- K014 -->\n",
    )
    .unwrap();
    std::fs::write(root.join("standards.rules.json"), entry).unwrap();
}
fn entry(check: Value) -> Value {
    let mut entry = json!({"name":"mechanical.one","target":{"input":"skill","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},"severity":"error"});
    entry
        .as_object_mut()
        .unwrap()
        .extend(check.as_object().unwrap().clone());
    entry
}

#[tokio::test]
async fn nested_ast_grep_unknown_keys_refuse_with_the_block_location_and_full_field_path() {
    for (rule, path) in [
        (
            json!({"kind":"paragraph","inside":{"kind":"list_item","typo":true}}),
            json!(["ast_grep", "inside", "typo"]),
        ),
        (
            json!({"kind":"paragraph","inside":{"kind":"list_item","stop_by":"end"}}),
            json!(["ast_grep", "inside", "stop_by"]),
        ),
        (
            json!({"all":[{"kind":"paragraph"},{"inside":{"kind":"list_item","inside":{"kind":"list","typo":true}}}]}),
            json!(["ast_grep", "all", "1", "inside", "inside", "typo"]),
        ),
        (
            json!({"kind":"paragraph","inside":{"kind":"list_item","stopBy":{"inside":{"kind":"document","typo":true}}}}),
            json!(["ast_grep", "inside", "stopBy", "inside", "typo"]),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut source = entry(json!({"ast_grep":rule.clone()}));
        source.as_object_mut().unwrap().remove("ast_grep");
        let mut text = serde_json::to_string_pretty(&json!({"K014":source})).unwrap();
        let insertion = text.find("    \"name\"").unwrap();
        text.insert_str(
            insertion,
            &format!(
                "    \"ast_grep\": {},\n",
                serde_json::to_string(&rule).unwrap()
            ),
        );
        let expected_line = text
            .lines()
            .position(|line| line.contains("\"ast_grep\""))
            .unwrap()
            + 1;
        write_source(directory.path(), &text);
        let outcome = check_rules(check_input(directory.path()), CancellationToken::new()).await;
        assert_eq!(outcome.exit_code(), 2, "{rule}: {outcome:?}");
        let wire = serde_json::to_value(outcome).unwrap();
        assert!(
            wire["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["code"] == "unknown_field"
                    && item["location"]["line"] == expected_line
                    && item["location"]["column"]
                        .as_u64()
                        .is_some_and(|column| column > 0)
                    && item["location"]["file"]
                        .as_str()
                        .unwrap()
                        .ends_with("standards.rules.json")
                    && item["location"]["field_path"] == path),
            "{wire}"
        );
        assert!(serde_json::from_value::<AstGrepRule>(rule).is_err());
    }
}

#[tokio::test]
async fn vendor_relation_options_remain_admitted_without_a_local_key_list() {
    let oracle: Value = serde_json::from_str(include_str!(
        "fixtures/mechanical-ast/relation-options-oracle.json"
    ))
    .unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let directory = tempfile::tempdir().unwrap();
        write_source(
            directory.path(),
            &serde_json::to_string(&json!({"K014":entry(json!({"ast_grep":case["rule"]}))}))
                .unwrap(),
        );
        let outcome = check_rules(check_input(directory.path()), CancellationToken::new()).await;
        let CheckRulesOutcome::Checked {
            checked_rule_count,
            printed_rules,
        } = outcome
        else {
            panic!("{case}: {outcome:?}")
        };
        assert_eq!(checked_rule_count, 1);
        let wire = serde_json::to_value(&printed_rules[0].definition).unwrap();
        assert!(schema_support::reviewed("rule-file.schema.json").is_valid(&wire));
        assert!(serde_json::from_value::<RuleDefinition>(wire).is_ok());
        assert!(!directory.path().join("unread.md").exists());
    }
}

#[tokio::test]
async fn check_lowers_each_mechanical_form_without_reading_a_target_or_making_a_question() {
    for check in [
        json!({"expect":{"matches":"MUST"}}),
        json!({"expect":{"not_matches":"bad"}}),
        json!({"expect":{"starts_with":"Use"}}),
        json!({"expect":{"present":true}}),
        json!({"ast_grep":{"kind":"paragraph","regex":"bad","not":{"regex":"good"}}}),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut value = entry(check.clone());
        if check["expect"].get("present").is_some() {
            value["target"]["select"] = json!({"kind":"frontmatter_field","field":"description"});
        }
        write_source(
            directory.path(),
            &serde_json::to_string(&json!({"K014":value})).unwrap(),
        );
        let outcome = check_rules(check_input(directory.path()), CancellationToken::new()).await;
        let CheckRulesOutcome::Checked {
            checked_rule_count,
            printed_rules,
        } = outcome
        else {
            panic!("{check}: {outcome:?}")
        };
        assert_eq!(checked_rule_count, 1);
        let wire = serde_json::to_value(&printed_rules[0].definition).unwrap();
        assert_eq!(wire["verdict"]["kind"], "mechanical");
        assert_eq!(
            wire["verdict"]["explanation"],
            "Keep this explanation verbatim."
        );
        assert_eq!(wire["questions"], json!({}));
        assert!(wire["verdict"].get("question").is_none());
        assert!(wire["verdict"].get("threshold").is_none());
        assert!(!directory.path().join("unread.md").exists());
    }
}

#[tokio::test]
async fn mechanical_source_alternatives_are_exclusive_and_ast_grep_errors_name_the_block_line() {
    for check in [
        json!({}),
        json!({"expect":{"matches":"ok"},"threshold":0.8}),
        json!({"expect":{"matches":"ok"},"ast_grep":{"kind":"paragraph"}}),
        json!({"expect":{"matches":"ok","present":true}}),
        json!({"expect":{"present":false}}),
    ] {
        let directory = tempfile::tempdir().unwrap();
        write_source(
            directory.path(),
            &serde_json::to_string(&json!({"K014":entry(check.clone())})).unwrap(),
        );
        let outcome = check_rules(check_input(directory.path()), CancellationToken::new()).await;
        assert_eq!(outcome.exit_code(), 2, "{check}: {outcome:?}");
    }
    let directory = tempfile::tempdir().unwrap();
    let rules = "{\n  \"K014\": {\n    \"name\": \"mechanical.one\",\n    \"target\": {\"input\":\"skill\",\"select\":{\"kind\":\"markdown_units\",\"mode\":\"unit\",\"units\":[\"paragraph\"]}},\n    \"ast_grep\": {\"regex\": \"[\"}\n  }\n}\n";
    write_source(directory.path(), rules);
    let outcome = check_rules(check_input(directory.path()), CancellationToken::new()).await;
    let CheckRulesOutcome::Refused {
        diagnostics,
        checked_rule_count,
        ..
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(checked_rule_count, 0);
    let wire = serde_json::to_value(diagnostics).unwrap();
    assert!(
        wire["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["location"]["file"]
                .as_str()
                .is_some_and(|path| path.ends_with("standards.rules.json"))
                && item["location"]["line"] == 5),
        "{wire}"
    );
}

#[tokio::test]
async fn markdown_rule_sources_scope_absence_checks_to_conversation_and_change_selections() {
    for (selector, kind, binding, expected_exit) in [
        (
            json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"excluded","assistant_messages":"excluded","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"excluded","input_pattern":"needle"},"hook_injected":"excluded"}),
            "conversation",
            json!({"kind":"conversation","transcript_path":"not-created.jsonl","format":"codex_rollout_jsonl"}),
            0,
        ),
        (
            json!({"kind":"change_set_literal_text","paths":[{"kind":"exact","path":"not-created.ts"}],"text":"needle","sides":["added"]}),
            "change_set",
            json!({"kind":"change_set","repository":"repo"}),
            0,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            "document",
            json!({"kind":"document","repository":"repo","path":"unread.md"}),
            2,
        ),
        (
            json!({"kind":"frontmatter_field","field":"description"}),
            "document",
            json!({"kind":"document","repository":"repo","path":"unread.md"}),
            2,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut source = entry(json!({"expect":{"present":false}}));
        source["target"]["select"] = selector;
        write_source(
            directory.path(),
            &serde_json::to_string(&json!({"K014":source})).unwrap(),
        );
        let path = directory.path().join("set.json");
        let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        set["inputs"]["skill"] = json!({"kind":kind});
        std::fs::write(path, serde_json::to_vec(&set).unwrap()).unwrap();
        let mut input = serde_json::to_value(check_input(directory.path())).unwrap();
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["skill"] = binding;
        if kind == "change_set" {
            input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        }
        let outcome = check_rules(
            serde_json::from_value(input).unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert_eq!(outcome.exit_code(), expected_exit, "{kind}: {outcome:?}");
        let wire = serde_json::to_value(outcome).unwrap();
        assert!(
            schema_support::reviewed("check-rules-outcome.schema.json").is_valid(&wire),
            "{wire}"
        );
        assert!(schema_support::generated::<CheckRulesOutcome>().is_valid(&wire));
        assert!(!directory.path().join("not-created.jsonl").exists());
    }
}

#[test]
fn judgment_rules_cannot_target_frontmatter_and_mechanical_rules_cannot_hide_model_questions() {
    let inputs = serde_json::from_value(json!({"skill":{"kind":"document"}})).unwrap();
    let base = json!({"rule_id":"mechanical.one","evidence":[{"id":"target","input":"skill","select":{"kind":"frontmatter_field","field":"description"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"mechanical","explanation":"Require a description","check":{"kind":"expect","expect":{"present":true}}}});
    let valid: RuleDefinition = serde_json::from_value(base.clone()).unwrap();
    assert!(schema_support::reviewed("rule-file.schema.json").is_valid(&base));
    assert!(validate_rule_definition(&valid, &inputs).is_ok());
    let mut invalid = base.clone();
    invalid["questions"] = json!({"extra":{"type":"noul","instructions":"Judge something"}});
    let invalid: RuleDefinition = serde_json::from_value(invalid).unwrap();
    assert!(validate_rule_definition(&invalid, &inputs).is_err());
    let mut judgment = base;
    judgment["verdict"] = json!({"kind":"noul","threshold":0.8,"question":{"type":"noul","instructions":"Judge the field"}});
    let judgment: RuleDefinition = serde_json::from_value(judgment).unwrap();
    assert!(validate_rule_definition(&judgment, &inputs).is_err());
}

#[test]
fn mechanical_source_shape_is_exclusive_even_when_an_extra_verdict_field_is_null() {
    let schema = serde_json::to_value(schemars::schema_for!(RuleSourceEntry)).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    let reviewed = schema_support::reviewed("rule-source.schema.json");
    for (check, shape) in [
        (json!({"expect":{"matches":"ok"}}), true),
        (json!({"ast_grep":{"kind":"paragraph"}}), true),
        (json!({"threshold":0.8,"error_at_or_below":null}), true),
        (json!({"expect":{"matches":"ok"},"threshold":null}), false),
        (
            json!({"expect":{"matches":"ok"},"error_at_or_below":null}),
            false,
        ),
        (json!({"expect":null}), false),
        (json!({"ast_grep":null}), false),
        (json!({"expect":{"matches":"ok","present":true}}), false),
    ] {
        let value = entry(check);
        assert_eq!(validator.is_valid(&value), shape, "schema: {value}");
        assert_eq!(
            reviewed.is_valid(&json!({"K014":value})),
            shape,
            "reviewed: {value}"
        );
        assert_eq!(
            serde_json::from_value::<RuleSourceEntry>(value.clone()).is_ok(),
            shape,
            "native: {value}"
        );
    }
}
