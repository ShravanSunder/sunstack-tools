#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

#[test]
fn backend_selection_allows_an_absent_ceiling_but_declared_ceilings_are_positive() {
    let missing = json!({
        "kind": "jev_openrouter",
        "endpoint": "http://127.0.0.1:1/",
        "model": "jev-latest"
    });
    assert!(
        serde_json::from_value::<BackendSelection>(missing.clone()).is_ok(),
        "an unknown provider ceiling must not reject a caller-bounded request"
    );
    for invalid in [json!(null), json!(0), json!(-1), json!(1.5)] {
        let mut selection = missing.clone();
        selection["context_ceiling_tokens"] = invalid;
        assert!(serde_json::from_value::<BackendSelection>(selection).is_err());
    }
    let mut declared = missing;
    declared["context_ceiling_tokens"] = json!(262144);
    assert!(serde_json::from_value::<BackendSelection>(declared).is_ok());
}
