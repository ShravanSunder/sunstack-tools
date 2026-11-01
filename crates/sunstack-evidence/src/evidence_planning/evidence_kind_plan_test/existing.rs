#![allow(clippy::unwrap_used)]
use super::super::*;
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
    value["plain"]["within"] = serde_json::json!(["a"]);
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

#[test]
fn within_plan_waits_for_every_parent_and_refuses_a_second_entry_cycle() {
    let mut value = serde_json::json!({
        "a_child":{"rule":"r","fields":{},"within":["b_parent","z_parent"]},
        "b_parent":{"rule":"r","fields":{}},
        "z_parent":{"rule":"r","fields":{}}
    });
    let plan = EvidenceKindPlan::derive(&kinds(value.clone())).unwrap();
    assert_eq!(
        plan.order
            .iter()
            .map(EvidenceKindName::as_str)
            .collect::<Vec<_>>(),
        ["b_parent", "z_parent", "a_child"]
    );
    value["z_parent"]["within"] = serde_json::json!(["a_child"]);
    let errors = EvidenceKindPlan::derive(&kinds(value)).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("a_child -> z_parent -> a_child")),
        "{errors:?}"
    );
}

#[test]
fn within_plan_refuses_an_undefined_second_entry() {
    let value = serde_json::json!({
        "child":{"rule":"r","fields":{},"within":["parent","missing"]},
        "parent":{"rule":"r","fields":{}}
    });
    let errors = EvidenceKindPlan::derive(&kinds(value)).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("undefined kind missing"));
    assert!(errors[0].field.to_string().contains("child"));
}

#[test]
fn cycle_field_is_the_deterministic_first_kind_in_its_reported_path() {
    for text in [
        r#"{"child":{"rule":"r","fields":{},"within":["plain","z"]},"plain":{"rule":"r","fields":{}},"z":{"rule":"r","fields":{},"within":["child"]}}"#,
        r#"{"z":{"rule":"r","fields":{},"within":["child"]},"plain":{"rule":"r","fields":{}},"child":{"rule":"r","fields":{},"within":["plain","z"]}}"#,
    ] {
        let definitions = serde_json::from_str(text).unwrap();
        let errors = EvidenceKindPlan::derive(&definitions).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field.to_string(), "evidence.kinds.child");
        assert_eq!(
            errors[0].message,
            "kind dependency cycle: child -> z -> child"
        );
    }
}
