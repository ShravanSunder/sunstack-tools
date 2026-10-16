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

#[test]
fn generated_backend_ceiling_schema_matches_native_optional_positive_only_contract() {
    let backend_schema = generated::<sunstack_decisions::BackendSelection>();
    let invocation_schema = generated::<RuleInvocationInput>();
    let reviewed_schema = reviewed("rule-invocation.schema.json");
    let invocation_base: Value = serde_json::from_str(include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-lint.invocation.json"
    ))
    .unwrap();
    let backend_base = json!({"kind":"jev_openrouter","endpoint":"https://openrouter.ai/api/v1/systemone","model":"jev-latest"});
    for (ceiling, accepted) in [
        (None, true),
        (Some(json!(1)), true),
        (Some(json!(262144)), true),
        (Some(json!(0)), false),
        (Some(Value::Null), false),
        (Some(json!(-1)), false),
        (Some(json!(1.5)), false),
    ] {
        let mut backend = backend_base.clone();
        if let Some(value) = ceiling {
            backend["context_ceiling_tokens"] = value;
        }
        assert_eq!(
            serde_json::from_value::<sunstack_decisions::BackendSelection>(backend.clone()).is_ok(),
            accepted
        );
        assert_eq!(
            backend_schema.is_valid(&backend),
            accepted,
            "backend {backend}"
        );
        let mut invocation = invocation_base.clone();
        invocation["backend"] = backend;
        assert_eq!(
            serde_json::from_value::<RuleInvocationInput>(invocation.clone()).is_ok(),
            accepted
        );
        assert_eq!(
            invocation_schema.is_valid(&invocation),
            accepted,
            "generated invocation {invocation}"
        );
        assert_eq!(
            reviewed_schema.is_valid(&invocation),
            accepted,
            "reviewed invocation {invocation}"
        );
    }
}

#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/decision_http.rs"]
mod transport;
#[tokio::test]
async fn reviewed_invocations_pass_native_cross_file_admission_and_real_evidence_preparation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let transcript = tempfile::tempdir().unwrap();
    let transcript_path = transcript.path().join("reviewer.jsonl");
    // A public Codex call/result literal retained verbatim, followed by a synthetic
    // shell pair in that same format so the example's read/shell selector is nonempty.
    let mut content = std::fs::read_to_string(
        root.join("crates/sunstack-evidence/tests/fixtures/codex-composed.jsonl"),
    )
    .unwrap();
    content.push_str("{\"timestamp\":\"2026-10-15T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"call_id\":\"read\",\"name\":\"exec_command\",\"arguments\":\"{\\\"cmd\\\":\\\"cat docs/specs/2026-10-12-rust-rule-engine/program-design.md\\\"}\"}}\n{\"timestamp\":\"2026-10-15T00:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"read\",\"output\":\"Program Design read.\"}}\n");
    std::fs::write(&transcript_path, content).unwrap();
    for name in [
        "skill-lint.invocation.json",
        "handoff-monitor.invocation.json",
    ] {
        let server = controlled::ControlledServer::start().await;
        let mut raw: Value =
            serde_json::from_slice(&std::fs::read(examples().join(name)).unwrap()).unwrap();
        raw["definition_root"] = json!(examples().canonicalize().unwrap());
        raw["backend"] = serde_json::to_value(transport::selection(&server.endpoint)).unwrap();
        if name.starts_with("skill") {
            raw["repositories"][0]["root"] = json!(root);
        } else {
            raw["definitions"]["binding_groups"][0]["bindings"]["session"]["transcript_path"] =
                json!(transcript_path);
            raw["payload_schemas"][0]["path"] = json!(
                root.join("crates/sunstack-evidence/tests/fixtures/agent-job-packet.schema.json")
            );
        }
        let input = decode_rule_invocation(&serde_json::to_vec(&raw).unwrap()).unwrap();
        let result = prepare_rules(
            input,
            RuleExecutionContext {
                backend: std::sync::Arc::new(transport::backend(&server.endpoint)),
                adapters: sunstack_evidence::AdapterRegistry::builtins().unwrap(),
                cancellation: sunstack_decisions::CancellationToken::new(),
            },
        )
        .await;
        let RulePreparationOutcome::Prepared { preparation } = result else {
            panic!("{name}: {result:?}")
        };
        assert_eq!(preparation.measurements.model_requests, 0);
        assert!(!preparation.planned_requests.is_empty());
        assert!(!preparation.occurrences.is_empty());
        assert_eq!(server.finish().await, 0);
    }
}
