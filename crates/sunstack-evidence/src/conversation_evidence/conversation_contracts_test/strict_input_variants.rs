#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::{ToolFilter, ToolUseSelection, TurnWindow};
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
    turn_window_all_refuses_unknown_fields,
    TurnWindow,
    r#"{"kind":"all"}"#
);
closed_unit!(
    turn_window_since_cursor_refuses_unknown_fields,
    TurnWindow,
    r#"{"kind":"since_cursor"}"#
);
closed_unit!(
    tool_filter_all_refuses_unknown_fields,
    ToolFilter,
    r#"{"kind":"all"}"#
);
closed_unit!(
    tool_use_excluded_refuses_unknown_fields,
    ToolUseSelection,
    r#"{"kind":"excluded"}"#
);
