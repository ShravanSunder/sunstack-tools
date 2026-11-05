#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::*;
#[test]
fn pointer_preserves_rfc6901_escaping_and_rejects_noncanonical_indices() {
    let payload: StructuredJson =
        serde_json::from_str(r#"{"a/b":{"~key":[null,"value"]}}"#).unwrap();
    assert_eq!(
        payload.pointer("/a~1b/~0key/1"),
        Some(&StructuredJson::String("value".into()))
    );
    assert!(payload.pointer("/a~1b/~0key/01").is_none());
    assert!(payload.pointer("/a~2b").is_none());
    assert!(payload.pointer("/a~1b/~0key/+1").is_none());
}
