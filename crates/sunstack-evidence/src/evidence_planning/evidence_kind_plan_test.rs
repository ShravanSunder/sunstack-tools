#![allow(clippy::unwrap_used)]
use super::*;
fn kinds(value: serde_json::Value) -> BTreeMap<EvidenceKindName, EvidenceKindDefinition> {
    serde_json::from_value(value).unwrap()
}
#[test]
fn plan_orders_forward_dependencies_and_refuses_cycles_and_selection_dependent_targets() {
    let mut value = serde_json::json!({"a":{"rule":"r","fields":{"target":{"from":"target","regex":"(x)","resolve":"skill","as_kind":"z"}}},"z":{"rule":"r","fields":{"headings":{"from":"same_file","kind":"plain"}}},"plain":{"rule":"r","fields":{}}});
    let plan = EvidenceKindPlan::derive(&kinds(value.clone())).unwrap();
    assert_eq!(
        plan.order
            .iter()
            .map(EvidenceKindName::as_str)
            .collect::<Vec<_>>(),
        ["plain", "z", "a"]
    );
    value["plain"]["within"] = serde_json::json!("a");
    assert!(EvidenceKindPlan::derive(&kinds(value.clone())).is_err());
    value["plain"].as_object_mut().unwrap().remove("within");
    value["plain"]["select"] =
        serde_json::json!({"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3});
    assert!(
        EvidenceKindPlan::derive(&kinds(value))
            .unwrap_err()
            .iter()
            .any(|error| error.message.contains("selection-dependent"))
    );
}
