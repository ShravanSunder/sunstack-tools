#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;

#[test]
fn native_comparison_results_reject_extra_fields_on_empty_outcomes() {
    assert!(
        serde_json::from_value::<ChangeComparison>(
            serde_json::json!({"kind":"unchanged","members":[0]})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ChangeComparison>(serde_json::json!({"kind":"unchanged"})).is_ok()
    );
}
