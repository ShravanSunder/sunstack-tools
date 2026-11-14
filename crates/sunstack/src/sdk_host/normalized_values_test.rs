#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

#[test]
fn normalization_sorts_keys_and_keeps_explicit_nulls_and_exact_integers() {
    let value = serde_json::json!({
        "z": 1,
        "a": {"y": null, "b": [3, null, {"d": 18446744073709551615u64, "c": null}]},
        "m": 0.5
    });
    assert_eq!(
        normalized_text(&value),
        r#"{"a":{"b":[3,null,{"c":null,"d":18446744073709551615}],"y":null},"m":0.5,"z":1}"#
    );
    // An explicit null member and an omitted member are different data.
    assert_ne!(
        normalized_text(&serde_json::json!({"setting": null})),
        normalized_text(&serde_json::json!({}))
    );
}

#[test]
fn source_ids_escape_pointer_tokens() {
    assert_eq!(
        source_id("standards", &["evidence", "matchers", "a/b~c"]),
        "standards#/evidence/matchers/a~1b~0c"
    );
}

#[test]
fn transported_values_match_across_integral_float_spelling_only() {
    use serde_json::json;
    let same = |supplied: serde_json::Value, original: serde_json::Value| {
        same_after_transport(&supplied, &original)
    };
    // An unchanged integral float returns from JavaScript as an integer.
    assert!(same(json!({"weight": 2}), json!({"weight": 2.0})));
    assert!(same(
        json!({"nested": {"list": [3, 1.5, -4]}}),
        json!({"nested": {"list": [3.0, 1.5, -4.0]}})
    ));
    assert!(same(json!(1.5), json!(1.5)));
    assert!(same(json!(null), json!(null)));
    // Real changes stay changes.
    assert!(!same(json!(3), json!(2.0)));
    assert!(!same(json!(2), json!(1.5)));
    assert!(!same(json!("2"), json!(2)));
    assert!(!same(
        json!({"weight": 2}),
        json!({"weight": 2, "extra": null})
    ));
    assert!(!same(json!([2]), json!([2, 2])));
    // Integers compare exactly; nothing is rounded through f64.
    assert!(!same(json!(u64::MAX), json!(u64::MAX - 1)));
    assert!(!same(
        json!(u64::MAX),
        json!(18_446_744_073_709_551_615.0_f64)
    ));
    assert!(!same(
        json!(9_007_199_254_740_993_u64),
        json!(9_007_199_254_740_992.0_f64)
    ));
    assert!(same(
        json!(9_007_199_254_740_992_u64),
        json!(9_007_199_254_740_992.0_f64)
    ));
    assert!(!same(json!(-1), json!(u64::MAX)));
}
