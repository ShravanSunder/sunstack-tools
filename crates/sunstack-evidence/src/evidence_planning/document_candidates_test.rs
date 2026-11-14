#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
#[test]
fn candidates_preserve_scalar_or_ordered_wire_and_refuse_empty_inputs() {
    for wire in [
        json!("verification.md"),
        json!(["verification.md", "verification-design.md"]),
    ] {
        let parsed: DocumentCandidates = serde_json::from_value(wire.clone()).unwrap();
        assert!(!parsed.paths().is_empty());
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
    for wire in [json!([]), json!([""]), json!(null), json!({"glob":"*.md"})] {
        assert!(serde_json::from_value::<DocumentCandidates>(wire).is_err());
    }
}
