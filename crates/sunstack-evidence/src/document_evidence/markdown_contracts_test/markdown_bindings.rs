#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;

#[test]
fn marker_ids_and_prefixes_validate_at_construction_and_deserialization() {
    for id in ["K000", "Z999"] {
        assert_eq!(MarkerId::new(id).unwrap().as_str(), id);
    }
    for id in ["", "K00", "K0000", "k014", "KK014", "K０１４"] {
        assert!(MarkerId::new(id).is_err(), "{id}");
        assert!(serde_json::from_value::<MarkerId>(serde_json::json!(id)).is_err());
    }
    for prefix in ["", "KK", "k", "1", "É"] {
        assert!(MarkerPrefix::new(prefix).is_err(), "{prefix}");
    }
}
