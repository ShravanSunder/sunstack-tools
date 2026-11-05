#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::{InvocationOrigin, OriginAgent};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

fn assert_closed_unit<TInput: DeserializeOwned + Serialize>(canonical_wire: &str) {
    let valid: TInput = serde_json::from_str(canonical_wire).unwrap();
    assert_eq!(serde_json::to_string(&valid).unwrap(), canonical_wire);
    let mut extra: serde_json::Value = serde_json::from_str(canonical_wire).unwrap();
    extra["unexpected"] = json!(1);
    assert!(
        serde_json::from_value::<TInput>(extra).is_err(),
        "{} silently accepted unexpected",
        std::any::type_name::<TInput>()
    );
}
macro_rules! closed_unit {
    ($name:ident, $input:ty, $wire:literal) => {
        #[test]
        fn $name() {
            assert_closed_unit::<$input>($wire);
        }
    };
}
closed_unit!(
    direct_origin_refuses_unknown_fields,
    InvocationOrigin,
    r#"{"kind":"direct"}"#
);
closed_unit!(
    main_agent_refuses_unknown_fields,
    OriginAgent,
    r#"{"kind":"main"}"#
);
closed_unit!(
    unknown_agent_refuses_unknown_fields,
    OriginAgent,
    r#"{"kind":"unknown"}"#
);
