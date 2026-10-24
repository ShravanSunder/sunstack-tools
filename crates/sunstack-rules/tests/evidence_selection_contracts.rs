#![allow(clippy::unwrap_used)]
#[path = "support/contract_schemas.rs"]
mod schema_support;
use serde_json::{Value, json};
use sunstack_evidence::EvidenceDeclaration;

#[test]
fn input_pattern_optional_string_shape_agrees_between_reviewed_generated_and_native_contracts() {
    let reviewed = schema_support::reviewed("evidence-declaration.schema.json");
    let generated = schema_support::generated::<EvidenceDeclaration>();
    for (pattern, admitted) in [
        (None, true),
        (Some(json!("quoted\\ninput")), true),
        (Some(Value::Null), true),
        (Some(json!(42)), false),
        (Some(json!(false)), false),
    ] {
        let mut declaration = json!({
            "id":"target","input":"session","select":{
                "kind":"conversation","turns":{"kind":"all"},
                "user_messages":"excluded","assistant_messages":"excluded",
                "tool_use":{"kind":"included","tools":{"kind":"all"},"results":"excluded"},
                "hook_injected":"excluded"
            }
        });
        if let Some(pattern) = pattern {
            declaration["select"]["tool_use"]["input_pattern"] = pattern;
        }
        assert_eq!(
            reviewed.is_valid(&declaration),
            admitted,
            "reviewed: {declaration}"
        );
        assert_eq!(
            generated.is_valid(&declaration),
            admitted,
            "generated: {declaration}"
        );
        assert_eq!(
            serde_json::from_value::<EvidenceDeclaration>(declaration).is_ok(),
            admitted
        );
    }
}
