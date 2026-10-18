#![allow(dead_code, clippy::unwrap_used)]
use schemars::JsonSchema;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
fn examples() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/2026-10-12-rust-rule-engine/examples")
}
pub fn generated<T: JsonSchema>() -> jsonschema::Validator {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .offline()
        .build(&schema)
        .unwrap()
}
pub fn reviewed(name: &str) -> jsonschema::Validator {
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
