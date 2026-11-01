#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;

#[test]
fn neighbour_sources_decode_rule_only_with_bounded_default_count() {
    for from in ["next", "previous"] {
        let source: FieldSource =
            serde_json::from_value(serde_json::json!({"from":from,"rule":"paragraph"})).unwrap();
        assert_eq!(source.shape(), FieldShape::Texts);
        assert_eq!(serde_json::to_value(source).unwrap()["count"], 1);
        for value in [
            serde_json::json!(0),
            serde_json::json!(6),
            serde_json::json!(null),
        ] {
            assert!(
                serde_json::from_value::<FieldSource>(
                    serde_json::json!({"from":from,"rule":"paragraph","count":value})
                )
                .is_err()
            );
        }
        for keys in [
            serde_json::json!({"kind":"existing"}),
            serde_json::json!({"rule":"paragraph","kind":"existing"}),
        ] {
            let mut value = keys;
            value["from"] = from.into();
            assert!(serde_json::from_value::<FieldSource>(value).is_err());
        }
    }
}
