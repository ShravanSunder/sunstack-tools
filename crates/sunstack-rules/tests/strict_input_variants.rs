#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use sunstack_evidence::{
    ChangeScope, EvidenceDiscovery, EvidenceSelection, ToolFilter, ToolUseSelection, TurnWindow,
};
use sunstack_rules::{InputDeclaration, InvocationOrigin, OriginAgent};

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
    change_scope_full_refuses_unknown_fields,
    ChangeScope,
    r#"{"kind":"full"}"#
);
closed_unit!(
    whole_document_refuses_unknown_fields,
    EvidenceSelection,
    r#"{"kind":"whole_document"}"#
);
closed_unit!(
    whole_payload_refuses_unknown_fields,
    EvidenceSelection,
    r#"{"kind":"whole_payload"}"#
);
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
closed_unit!(
    discovery_none_refuses_unknown_fields,
    EvidenceDiscovery,
    r#"{"kind":"none"}"#
);
closed_unit!(
    document_declaration_refuses_unknown_fields,
    InputDeclaration,
    r#"{"kind":"document"}"#
);
closed_unit!(
    conversation_declaration_refuses_unknown_fields,
    InputDeclaration,
    r#"{"kind":"conversation"}"#
);
closed_unit!(
    change_set_declaration_refuses_unknown_fields,
    InputDeclaration,
    r#"{"kind":"change_set"}"#
);
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
