#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;
use serde_json::json;

#[test]
fn selected_reasons_are_closed_nonempty_unique_and_in_the_canonical_order() {
    for (value, valid) in [
        (json!([]), false),
        (json!(["full_scope"]), true),
        (
            json!([
                "target_changed",
                "context_changed",
                "before_analysis_unknown"
            ]),
            true,
        ),
        (json!(["full_scope", "target_changed"]), true),
        (json!(["context_changed", "target_changed"]), false),
        (json!(["new_rule", "new_rule"]), false),
        (json!(["unknown"]), false),
    ] {
        assert_eq!(
            serde_json::from_value::<SelectedReasons>(value).is_ok(),
            valid
        );
    }
    assert!(
        serde_json::from_value::<OccurrenceSelection>(
            json!({"kind":"comparison_pending","selected_by":["full_scope"]})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<OccurrenceSelection>(json!({"kind":"selected","selected_by":[]}))
            .is_err()
    );
}
