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
#[path = "support/contract_schemas.rs"]
mod schema_support;
use schema_support::{generated, reviewed};
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
        } else if name.ends_with(".rules.json") {
            let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let _: BTreeMap<sunstack_evidence::MarkerId, RuleSourceEntry> =
                serde_json::from_value(value.clone()).unwrap();
            assert!(reviewed("rule-source.schema.json").is_valid(&value));
        } else if name.ends_with(".check.request.json") {
            let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(value["protocol_version"], 1);
            assert_eq!(value["operation"], "check_rules");
            assert_eq!(value.as_object().unwrap().len(), 3);
            decode_check_rules(&serde_json::to_vec(&value["input"]).unwrap()).unwrap();
            assert!(generated::<CheckRulesInput>().is_valid(&value["input"]));
            assert!(reviewed("check-rules-input.schema.json").is_valid(&value["input"]));
        } else if name == "marked-standards.md" {
            let text = std::fs::read_to_string(&path).unwrap();
            let bindings = sunstack_evidence::MarkdownDocument::parse(&text)
                .bind_markers(&sunstack_evidence::MarkerPrefix::new("K").unwrap(), 400);
            assert_eq!(bindings.len(), 1);
            assert!(
                matches!(&bindings[0],sunstack_evidence::MarkdownMarkerBinding::Bound { binding } if binding.question_text=="Clear result\n\nThe selected paragraph states a clear result.")
            );
        } else if name == "mechanical-standards.md" {
            let text = std::fs::read_to_string(path).unwrap();
            let bindings = sunstack_evidence::MarkdownDocument::parse(&text)
                .bind_markers(&sunstack_evidence::MarkerPrefix::new("K").unwrap(), 400);
            assert_eq!(bindings.len(), 2);
            assert!(bindings.iter().all(|binding| matches!(
                binding,
                sunstack_evidence::MarkdownMarkerBinding::Bound { .. }
            )));
        } else if name == "mechanical-skill.md" {
            assert!(
                std::fs::read_to_string(path)
                    .unwrap()
                    .contains("description: Use when")
            );
        } else {
            assert_eq!(name, "SKILL.md");
            assert!(std::fs::read_to_string(path).unwrap().contains("U001:"));
        }
    }
}
#[tokio::test]
async fn reviewed_marked_standard_example_checks_and_prints_through_the_native_api() {
    let request: Value = serde_json::from_slice(
        &std::fs::read(examples().join("marked-standards.check.request.json")).unwrap(),
    )
    .unwrap();
    let mut input = decode_check_rules(&serde_json::to_vec(&request["input"]).unwrap()).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    input.definition_root = workspace.join(&input.definition_root);
    for repository in &mut input.repositories {
        repository.root = workspace.join(&repository.root);
    }
    let outcome = check_rules(input, sunstack_decisions::CancellationToken::new()).await;
    let wire = serde_json::to_value(&outcome).unwrap();
    assert!(reviewed("check-rules-outcome.schema.json").is_valid(&wire));
    let CheckRulesOutcome::Checked {
        checked_rule_count,
        printed_rules,
    } = outcome
    else {
        panic!("{wire}")
    };
    assert_eq!(checked_rule_count, 1);
    assert_eq!(printed_rules.len(), 1);
    assert_eq!(
        printed_rules[0].definition.rule_id.as_str(),
        "paragraph.clear-result"
    );
    let VerdictDeclaration::Noul {
        question:
            sunstack_decisions::JudgmentQuestion::Noul {
                instructions: sunstack_decisions::StructuredEntry::Text(text),
                ..
            },
        ..
    } = &printed_rules[0].definition.verdict
    else {
        panic!("judgment shape")
    };
    assert_eq!(
        text,
        "Clear result\n\nThe selected paragraph states a clear result."
    );
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

fn milestone_two_invocation() -> Value {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-lint.invocation.json"
    ))
    .unwrap();
    input["discovery"] = json!({"kind":"bounded","limits":{"max_files":100,"max_file_bytes":100000,"max_total_bytes":1000000,"max_matches":1000,"include_ignored":false,"max_unit_visits":1000,"max_relationship_candidates":1000,"max_fan_out":100,"max_history_commits":100,"max_local_time_ms":10000}});
    input
}
async fn native_evidence_admits_invocation_fields(value: &Value) -> bool {
    let Ok(mut input) = serde_json::from_value::<RuleInvocationInput>(value.clone()) else {
        return false;
    };
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for (position, binding) in input.repositories.iter_mut().enumerate() {
        binding.root = if position == 0 {
            first.path().into()
        } else {
            second.path().into()
        };
    }
    let evidence = sunstack_evidence::EvidencePlanInput {
        repositories: input.repositories,
        sources: BTreeMap::new(),
        payload_schemas: vec![],
        cursors: vec![],
        declarations: vec![],
        native_queries: vec![],
        package_bindings: input.package_bindings,
        limits: input.limits,
        discovery: input.discovery,
    };
    // Admission is real; no selection resolves revisions or reads source content.
    sunstack_evidence::EvidenceSession::open(
        evidence,
        sunstack_evidence::AdapterRegistry::builtins().unwrap(),
        sunstack_decisions::CancellationToken::new(),
    )
    .await
    .is_ok()
}
fn native_scope_shape(value: &Value) -> (bool, bool) {
    match serde_json::from_value::<RuleInvocationInput>(value.clone()) {
        Ok(input) => (
            true,
            input
                .repositories
                .iter()
                .all(|repository| repository.change_scope.validate().is_ok()),
        ),
        Err(_) => (false, false),
    }
}
#[tokio::test]
async fn milestone_two_scope_variants_agree_across_reviewed_generated_and_native_admission() {
    let reviewed = reviewed("rule-invocation.schema.json");
    let generated = generated::<RuleInvocationInput>();
    let object_id = "a".repeat(40);
    let long_id = "B".repeat(64);
    let absolute = json!({"kind":"absolute","since_utc":"2026-10-01T00:00:00Z","until_utc":"2026-10-16T00:00:00Z"});
    let scopes = vec![
        json!({"kind":"full"}),
        json!({"kind":"uncommitted","layers":["staged","unstaged","untracked"]}),
        json!({"kind":"revision_diff","base_revision":"HEAD~1","head":{"kind":"worktree"}}),
        json!({"kind":"revision_diff","base_revision":"HEAD~1","head":{"kind":"commit","revision":"HEAD"}}),
        json!({"kind":"commit_set","commit_ids":[object_id],"parent_policy":"first_parent"}),
        json!({"kind":"commit_set","commit_ids":[long_id],"parent_policy":"each_parent"}),
        json!({"kind":"recent_commits","reference":"HEAD","window":{"kind":"lookback","seconds":3600},"parent_policy":"first_parent"}),
        json!({"kind":"recent_commits","reference":"HEAD","window":absolute,"parent_policy":"each_parent"}),
        json!({"kind":"recent_uncommitted","layers":["unstaged"],"window":{"kind":"lookback","seconds":3600},"unknown_recency":"include_with_diagnostic"}),
        json!({"kind":"recent_uncommitted","layers":["untracked"],"window":absolute,"unknown_recency":"exclude_with_omission"}),
        json!({"kind":"union","members":[{"kind":"uncommitted","layers":["staged"]},{"kind":"revision_diff","base_revision":"HEAD","head":{"kind":"worktree"}}]}),
    ];
    for scope in scopes {
        let mut input = milestone_two_invocation();
        input["repositories"][0]["change_scope"] = scope.clone();
        assert!(reviewed.is_valid(&input), "reviewed {scope}");
        assert!(generated.is_valid(&input), "generated {scope}");
        assert_eq!(native_scope_shape(&input), (true, true), "native {scope}");
        assert!(
            native_evidence_admits_invocation_fields(&input).await,
            "admission {scope}"
        );
    }
    for discovery in [
        None,
        Some(json!({"kind":"none"})),
        Some(milestone_two_invocation()["discovery"].clone()),
    ] {
        let mut input = milestone_two_invocation();
        if let Some(discovery) = discovery {
            input["discovery"] = discovery;
        } else {
            input.as_object_mut().unwrap().remove("discovery");
        }
        input.as_object_mut().unwrap().remove("package_bindings");
        assert!(reviewed.is_valid(&input));
        assert!(generated.is_valid(&input));
        assert_eq!(native_scope_shape(&input), (true, true));
        assert!(native_evidence_admits_invocation_fields(&input).await);
    }
}
#[tokio::test]
async fn scope_refusal_matrix_keeps_shape_validation_separate_from_native_semantics() {
    let reviewed = reviewed("rule-invocation.schema.json");
    let generated = generated::<RuleInvocationInput>();
    // Each tuple records reviewed, generated, native deserialize, scope validate, actual admission.
    let cases = vec![
        (
            json!({"kind":"union","members":[{"kind":"union","members":[{"kind":"full"}]}]}),
            (false, true, true, false, false),
        ),
        (
            json!({"kind":"union","members":[]}),
            (false, true, true, false, false),
        ),
        (
            json!({"kind":"uncommitted","layers":[]}),
            (false, true, true, false, false),
        ),
        (
            json!({"kind":"uncommitted","layers":["staged","staged"]}),
            (false, true, true, false, false),
        ),
        (
            json!({"kind":"revision_diff","base_revision":"  ","head":{"kind":"worktree"}}),
            (false, true, true, false, false),
        ),
        (
            json!({"kind":"commit_set","commit_ids":["bad-id"],"parent_policy":"first_parent"}),
            (false, true, false, false, false),
        ),
        (
            json!({"kind":"recent_commits","reference":"HEAD","window":{"kind":"lookback","seconds":0},"parent_policy":"first_parent"}),
            (false, false, false, false, false),
        ),
        (
            json!({"kind":"recent_commits","reference":"HEAD","window":{"kind":"absolute","since_utc":"2026-10-16T00:00:00Z","until_utc":"2026-10-01T00:00:00Z"},"parent_policy":"first_parent"}),
            (true, true, true, false, false),
        ),
        (
            json!({"kind":"recent_uncommitted","layers":["unstaged"],"window":{"kind":"lookback","seconds":1}}),
            (false, false, false, false, false),
        ),
    ];
    for (scope, expected) in cases {
        let mut input = milestone_two_invocation();
        input["repositories"][0]["change_scope"] = scope.clone();
        let (deserialize, validate) = native_scope_shape(&input);
        let actual = (
            reviewed.is_valid(&input),
            generated.is_valid(&input),
            deserialize,
            validate,
            native_evidence_admits_invocation_fields(&input).await,
        );
        assert_eq!(actual, expected, "layer matrix for {scope}");
    }
    let mut missing_limits = milestone_two_invocation();
    missing_limits["discovery"] = json!({"kind":"bounded"});
    assert!(!reviewed.is_valid(&missing_limits));
    assert!(!generated.is_valid(&missing_limits));
    assert_eq!(native_scope_shape(&missing_limits), (false, false));
    assert!(!native_evidence_admits_invocation_fields(&missing_limits).await);
}
#[tokio::test]
async fn closed_empty_scope_head_and_discovery_variants_refuse_extra_fields_at_every_boundary() {
    let reviewed = reviewed("rule-invocation.schema.json");
    let generated = generated::<RuleInvocationInput>();
    let mut cases = vec![];
    let mut full = milestone_two_invocation();
    full["repositories"][0]["change_scope"] = json!({"kind":"full","extra_field":true});
    cases.push(("full", full));
    let mut head = milestone_two_invocation();
    head["repositories"][0]["change_scope"] = json!({"kind":"revision_diff","base_revision":"HEAD","head":{"kind":"worktree","extra_field":true}});
    cases.push(("worktree", head));
    let mut none = milestone_two_invocation();
    none["discovery"] = json!({"kind":"none","extra_field":true});
    cases.push(("none", none));
    let mut disagreements = vec![];
    for (label, input) in cases {
        let (deserialize, validate) = native_scope_shape(&input);
        let actual = (
            reviewed.is_valid(&input),
            generated.is_valid(&input),
            deserialize,
            validate,
            native_evidence_admits_invocation_fields(&input).await,
        );
        if actual != (false, false, false, false, false) {
            disagreements.push(format!(
                "{label}: reviewed/generated/deserialize/scope/admission={actual:?}"
            ));
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("; "));
}
#[tokio::test]
async fn package_binding_schema_agrees_with_real_native_evidence_admission_including_root_prefix() {
    let reviewed = reviewed("rule-invocation.schema.json");
    let generated = generated::<RuleInvocationInput>();
    let baseline = json!({"importing_repository_id":"caller","module_prefix":"@library/","target_repository_id":"library","target_path_prefix":"src","language":"typescript"});
    let mut cases = vec![(baseline.clone(), true)];
    for language in ["rust", "typescript", "tsx"] {
        let mut root = baseline.clone();
        root["target_path_prefix"] = json!("");
        root["language"] = json!(language);
        cases.push((root, true));
    }
    for (field, value) in [
        ("target_path_prefix", json!("/absolute")),
        ("target_path_prefix", json!("../escape")),
        ("target_path_prefix", json!("src/../escape")),
        ("module_prefix", json!(" \t")),
        ("language", json!("python")),
    ] {
        let mut invalid = baseline.clone();
        invalid[field] = value;
        cases.push((invalid, false));
    }
    let mut missing = baseline.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("target_path_prefix");
    cases.push((missing, false));
    let mut extra = baseline;
    extra["extra_field"] = json!(true);
    cases.push((extra, false));
    for (binding, expected) in cases {
        let mut input = milestone_two_invocation();
        input["repositories"] = json!([{"key":"caller","root":".","input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}},{"key":"library","root":".","input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}]);
        input["package_bindings"] = json!([binding]);
        assert_eq!(
            reviewed.is_valid(&input),
            expected,
            "reviewed {}",
            input["package_bindings"]
        );
        let actual = native_evidence_admits_invocation_fields(&input).await;
        assert_eq!(
            actual, expected,
            "actual native evidence admission {}",
            input["package_bindings"]
        );
        if expected {
            assert!(generated.is_valid(&input));
        }
    }
}

#[test]
fn milestone_two_whole_unit_input_refuses_extra_fields_without_changing_minimal_wire_shape() {
    let generated = generated::<sunstack_evidence::UnitContent>();
    let valid = json!({"kind":"whole_unit"});
    let invalid = json!({"kind":"whole_unit","extra_field":true});
    assert!(generated.is_valid(&valid));
    assert!(serde_json::from_value::<sunstack_evidence::UnitContent>(valid).is_ok());
    assert!(!generated.is_valid(&invalid));
    assert!(serde_json::from_value::<sunstack_evidence::UnitContent>(invalid).is_err());
}
