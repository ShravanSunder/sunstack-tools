#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_rules::test_support::rule_fixture;
use sunstack_rules::*;
#[path = "../support/contract_schemas.rs"]
mod schema_support;
use schema_support::{generated, reviewed};
fn check_input(root: &std::path::Path) -> Value {
    let mut value = serde_json::to_value(rule_fixture::input(root, "not-a-url")).unwrap();
    for key in [
        "origin",
        "cursors",
        "execution",
        "package_bindings",
        "backend",
        "attempts",
        "request_budget",
    ] {
        value.as_object_mut().unwrap().remove(key);
    }
    value["output"] = json!("assembled_rules");
    value
}
#[tokio::test]
async fn check_schema_refuses_backend_fields_and_empty_root_at_full_admission() {
    let reviewed = reviewed("check-rules-input.schema.json");
    let generated = generated::<CheckRulesInput>();
    let root = tempfile::tempdir().unwrap();
    let baseline = check_input(root.path());
    assert!(reviewed.is_valid(&baseline));
    assert!(generated.is_valid(&baseline));
    let mut empty = baseline.clone();
    empty["definition_root"] = json!("");
    assert!(!reviewed.is_valid(&empty));
    assert!(!generated.is_valid(&empty));
    let outcome = check_rules(
        decode_check_rules(&serde_json::to_vec(&empty).unwrap()).unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(matches!(outcome, CheckRulesOutcome::Refused { .. }));
    for field in ["backend", "attempts", "request_budget"] {
        let mut extra = baseline.clone();
        extra[field] = json!({});
        assert!(!reviewed.is_valid(&extra));
        assert!(!generated.is_valid(&extra));
        assert!(decode_check_rules(&serde_json::to_vec(&extra).unwrap()).is_err());
    }
}

#[tokio::test]
async fn markdown_selector_schemas_and_check_keep_shape_and_semantic_refusals_distinct() {
    let reviewed_declaration = reviewed("evidence-declaration.schema.json");
    let generated_declaration = generated::<sunstack_evidence::EvidenceDeclaration>();
    for (selection, shape, admitted) in [
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph","list_item"],"pattern":"Use"}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"starts_with":"Completion:"}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"]}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":[]}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"Use","starts_with":"Use"}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"["}),
            true,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"\\b"}),
            true,
            false,
        ),
        (
            json!({"kind":"frontmatter_field","field":"name"}),
            true,
            false,
        ),
    ] {
        let declaration = json!({"id":"target","input":"subject","select":selection});
        assert_eq!(
            reviewed_declaration.is_valid(&declaration),
            shape,
            "reviewed {declaration}"
        );
        assert_eq!(
            generated_declaration.is_valid(&declaration),
            shape,
            "generated {declaration}"
        );
        let root = tempfile::tempdir().unwrap();
        let mut authored = rule_fixture::rule("selector");
        authored["evidence"][0]["select"] = selection;
        rule_fixture::write_set(root.path(), &[authored], vec![]);
        let input =
            decode_check_rules(&serde_json::to_vec(&check_input(root.path())).unwrap()).unwrap();
        let outcome = check_rules(input, CancellationToken::new()).await;
        assert_eq!(
            matches!(outcome, CheckRulesOutcome::Checked { .. }),
            admitted,
            "{declaration}: {outcome:?}"
        );
        assert!(!root.path().join("missing.md").exists());
    }
}

#[test]
fn versioned_reports_require_coverage_and_reject_unknown_skip_shapes() {
    let baseline = json!({"origin":{"kind":"direct"},"routing":{"kind":"rule_sets"},"groups":[],"occurrences":[],"rule_evaluations":[],"requests":[],"captures":[],"change_selections":[],"cursors":[],"diagnostics":{"items":[],"omitted_count":0},"measurements":InvocationMeasurements::default(),"rule_coverage":"full","skipped_rules":[]});
    let reviewed = reviewed("rule-invocation-report.schema.json");
    let generated = generated::<RuleInvocationReport>();
    for coverage in ["full", "partial", "none"] {
        let mut value = baseline.clone();
        value["rule_coverage"] = json!(coverage);
        assert!(reviewed.is_valid(&value));
        assert!(generated.is_valid(&value));
        assert!(serde_json::from_value::<RuleInvocationReport>(value).is_ok());
    }
    let mut missing = baseline.clone();
    missing.as_object_mut().unwrap().remove("rule_coverage");
    let mut unknown = baseline.clone();
    unknown["skipped_rules"] = json!([{"kind":"invented"}]);
    let mut missing_evaluations = baseline.clone();
    missing_evaluations
        .as_object_mut()
        .unwrap()
        .remove("rule_evaluations");
    let mut unknown_evaluation = baseline.clone();
    unknown_evaluation["rule_evaluations"] = json!([{"kind":"waiting_for_expansion","key":{"rule_set":"set.json","group":0,"rule_id":"one"},"on":[]}]);
    let mut old_counts = baseline;
    old_counts["measurements"]
        .as_object_mut()
        .unwrap()
        .remove("invalid_rule_entries");
    for value in [
        missing,
        unknown,
        old_counts,
        missing_evaluations,
        unknown_evaluation,
    ] {
        assert!(!reviewed.is_valid(&value));
        assert!(!generated.is_valid(&value));
        assert!(serde_json::from_value::<RuleInvocationReport>(value).is_err());
    }
}
