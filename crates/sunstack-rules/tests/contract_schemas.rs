#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
use sunstack_rules::*;
fn examples() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-12-rust-rule-engine/examples")
}
fn generated<T: JsonSchema>() -> jsonschema::Validator {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .offline()
        .build(&schema)
        .unwrap()
}
fn reviewed(name: &str) -> jsonschema::Validator {
    let directory = examples().parent().unwrap().join("schemas");
    let mut registry = jsonschema::Registry::new();
    let mut resources = BTreeMap::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            value["$id"] = json!(format!("https://schemas.test/{name}"));
            registry = registry
                .add(format!("https://schemas.test/{name}"), value.clone())
                .unwrap();
            resources.insert(name, value);
        }
    }
    let registry = registry.prepare().unwrap();
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_registry(&registry)
        .offline()
        .build(&resources[name])
        .unwrap()
}
fn admit_shape<T: JsonSchema + DeserializeOwned>(path: &Path) {
    let raw = std::fs::read(path).unwrap();
    let _: T = serde_json::from_slice(&raw).unwrap();
    let value: Value = serde_json::from_slice(&raw).unwrap();
    assert!(generated::<T>().is_valid(&value), "{}", path.display());
}
#[test]
fn every_reviewed_milestone_one_definition_and_invocation_deserializes_and_validates_generated_schema()
 {
    for entry in std::fs::read_dir(examples()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if name.ends_with(".ruleset.json") {
            admit_shape::<RuleSetDefinition>(&path);
            let set = decode_rule_set(&std::fs::read(&path).unwrap(), &path).unwrap();
            for rule_path in &set.rule_paths {
                for (line, raw) in std::fs::read_to_string(path.parent().unwrap().join(rule_path))
                    .unwrap()
                    .lines()
                    .enumerate()
                {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    let rule = decode_rule_definition(raw.as_bytes(), &path, line + 1).unwrap();
                    validate_rule_definition(&rule, &set.inputs).unwrap();
                }
            }
        } else if name.ends_with(".router.json") {
            admit_shape::<RouterDefinition>(&path);
            decode_router(&std::fs::read(&path).unwrap(), &path).unwrap();
        } else if name.ends_with(".invocation.json") {
            admit_shape::<RuleInvocationInput>(&path);
            decode_rule_invocation(&std::fs::read(&path).unwrap()).unwrap();
        } else if name.ends_with(".jsonl") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                let _: RuleDefinition = serde_json::from_str(line).unwrap();
                assert!(
                    generated::<RuleDefinition>().is_valid(&serde_json::from_str(line).unwrap())
                );
            }
        } else {
            assert_eq!(name, "SKILL.md");
            assert!(std::fs::read_to_string(path).unwrap().contains("U001:"));
        }
    }
}
#[test]
fn generated_and_reviewed_rule_shapes_agree_on_shape_refusal_controls() {
    let baseline: Value = serde_json::from_str(
        include_str!("../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl")
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let generated = generated::<RuleDefinition>();
    let reviewed = reviewed("rule-file.schema.json");
    let mut cases = vec![(baseline.clone(), true)];
    for (pointer, value) in [
        ("/rule_id", json!("BAD ID")),
        ("/evidence", json!([])),
        ("/verdict/threshold", json!(0)),
        ("/verdict/question/instructions", json!(true)),
        ("/verdict/question/criteria", Value::Null),
        ("/evidence/0/id", json!("INVALID")),
        ("/evidence/0/select/target_id", json!("U01")),
    ] {
        let mut case = baseline.clone();
        *case.pointer_mut(pointer).unwrap() = value;
        cases.push((case, false));
    }
    for (case, valid) in cases {
        assert_eq!(reviewed.is_valid(&case), valid, "reviewed {case}");
        assert_eq!(generated.is_valid(&case), valid, "generated {case}");
    }
}
#[test]
fn generated_invocation_shape_refuses_zero_budgets_and_empty_binding_groups() {
    let baseline: Value = serde_json::from_str(include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-lint.invocation.json"
    ))
    .unwrap();
    let generated = generated::<RuleInvocationInput>();
    let reviewed = reviewed("rule-invocation.schema.json");
    for (pointer, value) in [
        ("/request_budget/max_input_tokens", json!(0)),
        ("/attempts/attempt_timeout_ms", json!(0)),
        ("/definitions/rule_sets/0/binding_groups", json!([])),
    ] {
        let mut case = baseline.clone();
        *case.pointer_mut(pointer).unwrap() = value;
        assert!(!reviewed.is_valid(&case));
        assert!(!generated.is_valid(&case), "generated accepts {pointer}");
    }
}
