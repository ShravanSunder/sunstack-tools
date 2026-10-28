#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use sunstack_evidence::*;
use sunstack_rules::*;
use support::*;

#[test]
fn a_payload_schema_requires_an_explicit_frozen_name_path_binding_and_pinned_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let path = root.path().join("set.json");
    let mut set: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["owner"] = json!({"kind":"payload","schema":"urn:unrelated-schema-name"});
    std::fs::write(path, set.to_string()).unwrap();
    let error = freeze_cases(&root.path().join("missing-map"), &specification).unwrap_err();
    let EvalError::Definitions { diagnostics } = error else {
        panic!("{error:?}")
    };
    let wire = serde_json::to_value(diagnostics).unwrap();
    assert!(
        wire["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "unknown_payload_schema"
                && item["location"]["field_path"] == json!(["inputs", "owner", "schema"]))
    );
    assert!(!root.path().join("missing-map").exists());
    let schema=json!({"type":"object","required":["approved"],"properties":{"approved":{"type":"boolean"}}}).to_string();
    let schema_path = root.path().join("arbitrary-file-name.json");
    std::fs::write(&schema_path, &schema).unwrap();
    specification.payload_schemas = vec![PayloadSchemaPath {
        name: SchemaName::new("urn:unrelated-schema-name").unwrap(),
        path: schema_path.clone(),
    }];
    let frozen = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let mapping = &frozen.manifest().payload_schemas[0];
    assert_eq!(mapping.name.as_str(), "urn:unrelated-schema-name");
    assert_eq!(
        mapping.path,
        std::path::PathBuf::from("arbitrary-file-name.json")
    );
    assert_eq!(
        mapping.content_identity.as_str(),
        content_digest(schema.as_bytes())
    );
    std::fs::write(&schema_path, "{\"not\":{}}").unwrap();
    let loaded = load_case_set(&root.path().join("cases/manifest.json")).unwrap();
    assert_eq!(loaded.rule_version(), frozen.rule_version());
    let copied = frozen.definition_root().join("arbitrary-file-name.json");
    assert_eq!(std::fs::read_to_string(&copied).unwrap(), schema);
    std::fs::write(copied, "{}").unwrap();
    assert!(matches!(
        load_case_set(&root.path().join("cases/manifest.json")),
        Err(EvalError::Integrity { .. })
    ));
}
#[test]
fn entry_paths_must_be_frozen_and_the_entry_must_reach_the_named_rule() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let unlisted_router = json!({"schema_version":1,"name":"unlisted-entry","description":"Existing valid entry reaching the frozen scored set","inputs":{"subject":{"kind":"document"}},"evidence":[{"id":"context","input":"subject","select":{"kind":"whole_document"}}],"question":{"type":"choice","instructions":"Route","criteria":{"a":"A","b":"B"}},"routes":{"a":[{"rule_set":"set.json","inputs":{"subject":"subject"}}],"b":[{"rule_set":"set.json","inputs":{"subject":"subject"}}]},"confidence_floor":0.6});
    let unlisted_path = root.path().join("unlisted-router.json");
    std::fs::write(&unlisted_path, unlisted_router.to_string()).unwrap();
    sunstack_rules::decode_router(&std::fs::read(&unlisted_path).unwrap(), &unlisted_path).unwrap();
    specification.entry = CandidateDefinitions::Router {
        path: "unlisted-router.json".into(),
    };
    let error = freeze_cases(&root.path().join("missing"), &specification).unwrap_err();
    let EvalError::Definitions { diagnostics } = error else {
        panic!("{error:?}")
    };
    let wire = serde_json::to_value(diagnostics).unwrap();
    assert!(
        wire["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "missing_rule_entry"
                && item["location"]["file"] == "unlisted-router.json"
                && item["location"]["field_path"] == json!(["entry"]))
    );
    assert!(!root.path().join("missing").exists());
    let mut other: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("set.json")).unwrap()).unwrap();
    other["rule_paths"] = json!(["other.rules.jsonl"]);
    std::fs::write(root.path().join("other.json"), other.to_string()).unwrap();
    std::fs::copy(
        root.path().join("rule.jsonl"),
        root.path().join("other.rules.jsonl"),
    )
    .unwrap();
    specification.definition_files.extend([
        root.path().join("other.json"),
        root.path().join("other.rules.jsonl"),
    ]);
    for router in [false, true] {
        if router {
            let value = json!({"schema_version":1,"name":"wrong-entry","description":"Cannot reach scored rule","inputs":{"subject":{"kind":"document"}},"evidence":[{"id":"context","input":"subject","select":{"kind":"whole_document"}}],"question":{"type":"choice","instructions":"Route","criteria":{"a":"A","b":"B"}},"routes":{"a":[{"rule_set":"other.json","inputs":{"subject":"subject"}}],"b":[{"rule_set":"other.json","inputs":{"subject":"subject"}}]},"confidence_floor":0.6});
            std::fs::write(root.path().join("router.json"), value.to_string()).unwrap();
            specification
                .definition_files
                .push(root.path().join("router.json"));
            specification.entry = CandidateDefinitions::Router {
                path: "router.json".into(),
            };
        } else {
            specification.entry = CandidateDefinitions::RuleSets {
                rule_sets: vec!["other.json".into()],
            };
        }
        let error = freeze_cases(
            &root.path().join(if router {
                "unreachable-router"
            } else {
                "unreachable-direct"
            }),
            &specification,
        )
        .unwrap_err();
        let EvalError::Definitions { diagnostics } = error else {
            panic!("{error:?}")
        };
        let value = serde_json::to_value(diagnostics).unwrap();
        assert!(
            value["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["code"] == "missing_rule_entry"
                    && item["location"]["file"] == "set.json"
                    && item["location"]["field_path"] == json!(["rule_id", "test_rule"]))
        );
    }
}
