#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sunstack_decisions as decisions;
use sunstack_evidence as evidence;
use sunstack_rules as rules;
fn assert_closed_fielded<TInput: DeserializeOwned + Serialize>(valid: Value) {
    let decoded: TInput = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);
    let mut extra = valid;
    extra["unexpected"] = json!(1);
    assert!(
        serde_json::from_value::<TInput>(extra).is_err(),
        "{} accepted unexpected",
        std::any::type_name::<TInput>()
    );
}

#[test]
fn fielded_alternatives_of_affected_enums_stay_closed() {
    for value in [
        json!({"kind":"paragraph","target_id":"U001"}),
        json!({"kind":"literal_text","text":"text"}),
        json!({"kind":"regex_text","pattern":"text"}),
        json!({"kind":"ast_pattern","language":"rust","pattern":"fn $NAME() {}"}),
        json!({"kind":"payload_field","pointer":"/enabled"}),
        json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"excluded"},"hook_injected":"excluded"}),
    ] {
        assert_closed_fielded::<evidence::EvidenceSelection>(value);
    }
    assert_closed_fielded::<evidence::TurnWindow>(json!({"kind":"last","count":2}));
    assert_closed_fielded::<evidence::ToolFilter>(json!({"kind":"named","names":["Read"]}));
    assert_closed_fielded::<evidence::ToolFilter>(
        json!({"kind":"categories","categories":["read_file"]}),
    );
    assert_closed_fielded::<evidence::ToolUseSelection>(
        json!({"kind":"included","tools":{"kind":"all"},"results":"included"}),
    );
    assert_closed_fielded::<evidence::EvidenceDiscovery>(
        json!({"kind":"bounded","limits":{"max_files":1,"max_file_bytes":1024,"max_total_bytes":1024,"max_matches":1,"include_ignored":false,"max_unit_visits":1,"max_relationship_candidates":1,"max_fan_out":1,"max_history_commits":1,"max_local_time_ms":100}}),
    );
    assert_closed_fielded::<rules::InputDeclaration>(json!({"kind":"payload","schema":"flags"}));
    assert_closed_fielded::<rules::InvocationOrigin>(
        json!({"kind":"hook","host":"codex","event":"Stop"}),
    );
    assert_closed_fielded::<rules::OriginAgent>(json!({"kind":"subagent","id":"reviewer"}));
}
#[test]
fn plain_string_enums_and_author_json_keep_their_existing_wire() {
    let authored =
        json!({"kind":"full","unexpected":1,"nested":{"kind":"direct","unexpected":true}});
    let source: evidence::SourceSpec =
        serde_json::from_value(json!({"kind":"payload","schema":"flags","value":authored}))
            .unwrap();
    assert_eq!(serde_json::to_value(source).unwrap()["value"], authored);
    let question: decisions::JudgmentQuestion =
        serde_json::from_value(json!({"type":"noul","instructions":authored})).unwrap();
    assert_eq!(
        serde_json::to_value(question).unwrap()["instructions"],
        authored
    );
    macro_rules! string_wire {
        ($input:ty, $wire:literal) => {{
            let parsed: $input = serde_json::from_str($wire).unwrap();
            assert_eq!(serde_json::to_string(&parsed).unwrap(), $wire);
        }};
    }
    string_wire!(evidence::Inclusion, r#""included""#);
    string_wire!(evidence::AssistantSelection, r#""final_only""#);
    string_wire!(decisions::AccountingPolicy, r#""allow_estimated""#);
    string_wire!(rules::Severity, r#""warn""#);
    string_wire!(rules::Speculation, r#""off""#);
}
