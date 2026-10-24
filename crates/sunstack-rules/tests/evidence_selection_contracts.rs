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

#[test]
fn change_match_sides_are_required_and_nonempty_in_reviewed_and_generated_contracts() {
    let reviewed = schema_support::reviewed("evidence-declaration.schema.json");
    let generated = schema_support::generated::<EvidenceDeclaration>();
    for selector in [
        json!({"kind":"change_set_literal_text","text":"call"}),
        json!({"kind":"change_set_regex_text","pattern":"call"}),
        json!({"kind":"change_set_ast_pattern","language":"typescript","pattern":"call()"}),
    ] {
        for (sides, valid) in [
            (None, false),
            (Some(json!([])), false),
            (Some(json!(["added"])), true),
            (Some(json!(["removed"])), true),
            (Some(json!(["after_file"])), true),
            (Some(json!(["before_file"])), true),
            (Some(json!(["future_side"])), false),
        ] {
            let mut declaration = json!({"id":"target","input":"changes","select":selector});
            declaration["select"]["paths"] = json!([{"kind":"exact","path":"source.ts"}]);
            if let Some(sides) = sides {
                declaration["select"]["sides"] = sides;
            }
            assert_eq!(
                reviewed.is_valid(&declaration),
                valid,
                "reviewed: {declaration}"
            );
            assert_eq!(
                generated.is_valid(&declaration),
                valid,
                "generated: {declaration}"
            );
        }
    }
}
