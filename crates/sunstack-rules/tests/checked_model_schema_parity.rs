#![allow(clippy::panic, clippy::unwrap_used)]

#[path = "support/checked_model_fixture.rs"]
mod checked_model_fixture;
#[path = "support/contract_schemas.rs"]
mod schema_support;

use checked_model_fixture::{
    assert_populated_output, invalid_required_value_variants, unknown_field_variants,
    write_checked_model_files,
};
use schema_support::{generated, reviewed};
use serde_json::{Value, json};
use std::path::Path;
use sunstack_decisions::CancellationToken;
use sunstack_rules::{CheckRulesOutcome, check_rules, decode_check_rules};

fn assert_schema_parity_and_closed_fields(outcome: &Value) {
    let generated = generated::<CheckRulesOutcome>();
    let published = reviewed("check-rules-outcome.schema.json");
    assert!(
        generated.is_valid(outcome),
        "generated schema rejected {outcome}"
    );
    assert!(
        published.is_valid(outcome),
        "published schema rejected {outcome}"
    );

    for (field, invalid) in unknown_field_variants(outcome)
        .into_iter()
        .chain(invalid_required_value_variants(outcome))
    {
        assert!(
            !generated.is_valid(&invalid),
            "generated schema accepted extra field in {field}: {invalid}"
        );
        assert!(
            !published.is_valid(&invalid),
            "published schema accepted extra field in {field}: {invalid}"
        );
        assert!(
            serde_json::from_value::<CheckRulesOutcome>(invalid).is_err(),
            "native outcome accepted extra field in {field}"
        );
    }
}

#[tokio::test]
async fn checked_marked_and_full_models_match_generated_and_published_output_schemas() {
    let temporary_root = tempfile::tempdir().unwrap();
    let input_value = write_checked_model_files(temporary_root.path());
    let input = decode_check_rules(&serde_json::to_vec(&input_value).unwrap()).unwrap();
    let outcome = check_rules(input, CancellationToken::new()).await;
    let wire = serde_json::to_value(&outcome).unwrap();

    assert_populated_output(&wire, temporary_root.path());
    assert_schema_parity_and_closed_fields(&wire);
}

#[tokio::test]
async fn flat_legacy_marked_rule_remains_valid_under_both_output_schemas() {
    let example_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-12-rust-rule-engine/examples");
    let mut request: Value = serde_json::from_str(include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/marked-standards.check.request.json"
    ))
    .unwrap();
    request["input"]["definition_root"] = json!(example_root);
    let input = decode_check_rules(&serde_json::to_vec(&request["input"]).unwrap()).unwrap();
    let outcome = check_rules(input, CancellationToken::new()).await;
    let CheckRulesOutcome::Checked {
        checked_rule_count,
        printed_rules,
    } = &outcome
    else {
        panic!("expected flat legacy rule to check, got {outcome:?}");
    };
    assert_eq!(*checked_rule_count, 1);
    assert_eq!(printed_rules.len(), 1);
    assert!(printed_rules[0].definition.mechanical_pass.is_none());
    assert!(printed_rules[0].definition.narrowing.is_none());

    let wire = serde_json::to_value(&outcome).unwrap();
    let generated = generated::<CheckRulesOutcome>();
    let published = reviewed("check-rules-outcome.schema.json");
    assert!(
        generated.is_valid(&wire),
        "generated schema rejected {wire}"
    );
    assert!(
        published.is_valid(&wire),
        "published schema rejected {wire}"
    );
}
