#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_rules::test_support::rule_fixture;
use sunstack_rules::*;
#[path = "../support/contract_schemas.rs"]
mod schema_support;
fn check_input(root: &std::path::Path) -> CheckRulesInput {
    let mut input = serde_json::to_value(rule_fixture::input(root, "not-a-url")).unwrap();
    for key in [
        "origin",
        "cursors",
        "execution",
        "backend",
        "attempts",
        "request_budget",
        "package_bindings",
    ] {
        input.as_object_mut().unwrap().remove(key);
    }
    input["output"] = json!("assembled_rules");
    serde_json::from_value(input).unwrap()
}
fn mechanical(check: Value) -> Value {
    let mut rule = rule_fixture::rule("mechanical.one");
    rule["evidence"][0]["select"] =
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    rule["verdict"] =
        json!({"kind":"mechanical","explanation":"Keep this explanation verbatim.","check":check});
    rule
}
#[tokio::test]
async fn native_ast_refusals_are_located_and_vendor_relations_remain_admitted() {
    for ast in [
        json!({"kind":"paragraph","inside":{"kind":"list_item","typo":true}}),
        json!({"kind":"paragraph","inside":{"kind":"list_item","stop_by":"end"}}),
        json!({"all":[{"kind":"paragraph"},{"inside":{"kind":"list_item","inside":{"kind":"list","typo":true}}}]}),
        json!({"kind":"paragraph","inside":{"kind":"list_item","stopBy":{"inside":{"kind":"document","typo":true}}}}),
        json!({"kind":"paragraph","regex":"["}),
    ] {
        let root = tempfile::tempdir().unwrap();
        rule_fixture::write_set(
            root.path(),
            &[mechanical(json!({"kind":"ast_grep","ast_grep":ast}))],
            vec![],
        );
        let outcome = check_rules(check_input(root.path()), CancellationToken::new()).await;
        assert_eq!(outcome.exit_code(), 2, "{outcome:?}");
        let value = serde_json::to_value(outcome).unwrap();
        assert!(
            value["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["location"]["file"]
                    .as_str()
                    .is_some_and(|file| file.ends_with("rules.jsonl"))
                    && item["location"]["line"] == 1)
        );
    }
    let oracle: Value = serde_json::from_str(include_str!(
        "../fixtures/mechanical-ast/relation-options-oracle.json"
    ))
    .unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let root = tempfile::tempdir().unwrap();
        rule_fixture::write_set(
            root.path(),
            &[mechanical(
                json!({"kind":"ast_grep","ast_grep":case["rule"]}),
            )],
            vec![],
        );
        let outcome = check_rules(check_input(root.path()), CancellationToken::new()).await;
        assert_eq!(outcome.exit_code(), 0, "{case}: {outcome:?}");
    }
}
#[tokio::test]
async fn native_mechanical_admission_keeps_expect_forms_and_refuses_invalid_shapes_without_questions()
 {
    for (check, valid) in [
        (json!({"kind":"expect","expect":{"matches":"MUST"}}), true),
        (
            json!({"kind":"expect","expect":{"not_matches":"bad"}}),
            true,
        ),
        (
            json!({"kind":"expect","expect":{"starts_with":"Use"}}),
            true,
        ),
        (json!({"kind":"expect","expect":{"matches":"["}}), false),
        (
            json!({"kind":"expect","expect":{"matches":"ok","present":true}}),
            false,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        rule_fixture::write_set(root.path(), &[mechanical(check.clone())], vec![]);
        let outcome = check_rules(check_input(root.path()), CancellationToken::new()).await;
        assert_eq!(outcome.exit_code() == 0, valid, "{check}: {outcome:?}");
        if let CheckRulesOutcome::Checked { printed_rules, .. } = outcome {
            let wire = serde_json::to_value(&printed_rules[0].definition).unwrap();
            assert_eq!(wire["verdict"]["kind"], "mechanical");
            assert_eq!(wire["questions"], json!({}));
            assert!(wire["verdict"].get("question").is_none());
            assert!(schema_support::reviewed("rule-file.schema.json").is_valid(&wire));
        }
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
