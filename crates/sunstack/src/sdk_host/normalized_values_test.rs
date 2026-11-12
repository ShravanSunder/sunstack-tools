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
