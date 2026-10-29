#![allow(clippy::panic, clippy::unwrap_used)]

#[path = "../../sunstack-rules/tests/support/checked_model_fixture.rs"]
mod checked_model_fixture;
#[path = "../../sunstack-rules/tests/support/contract_schemas.rs"]
mod schema_support;
mod support;

use checked_model_fixture::{
    assert_populated_output, invalid_required_value_variants, unknown_field_variants,
    write_checked_model_files,
};
use schema_support::{generated, reviewed};
use serde_json::Value;
use sunstack_rules::CheckRulesOutcome;

#[tokio::test]
async fn check_rules_binary_prints_populated_models_accepted_by_both_output_schemas() {
    let temporary_root = tempfile::tempdir().unwrap();
    let input = write_checked_model_files(temporary_root.path());
    let output = support::run_without_credential(temporary_root.path(), "check_rules", input).await;
    let response = support::response(&output, 0);
    assert_eq!(response["result"]["operation"], "check_rules");
    let outcome: Value = response["result"]["outcome"].clone();

    assert_populated_output(&outcome, temporary_root.path());
    let generated = generated::<CheckRulesOutcome>();
    let published = reviewed("check-rules-outcome.schema.json");
    assert!(
        generated.is_valid(&outcome),
        "generated schema rejected {outcome}"
    );
    assert!(
        published.is_valid(&outcome),
        "published schema rejected {outcome}"
    );

    for (field, invalid) in unknown_field_variants(&outcome)
        .into_iter()
        .chain(invalid_required_value_variants(&outcome))
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
