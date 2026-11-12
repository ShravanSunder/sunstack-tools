#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

#[test]
fn normalization_sorts_keys_drops_nulls_and_keeps_exact_integers() {
    let value = serde_json::json!({
        "z": 1,
        "a": {"y": null, "b": [3, {"d": 18446744073709551615u64, "c": null}]},
        "m": 0.5
    });
    assert_eq!(
        normalized_text(&value),
        r#"{"a":{"b":[3,{"d":18446744073709551615}]},"m":0.5,"z":1}"#
    );
}

#[test]
fn source_ids_escape_pointer_tokens() {
    assert_eq!(
        source_id("standards", &["evidence", "matchers", "a/b~c"]),
        "standards#/evidence/matchers/a~1b~0c"
    );
}
