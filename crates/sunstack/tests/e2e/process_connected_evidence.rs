#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::change_fixture as changes;
use sunstack_rules::test_support::rule_fixture;
#[path = "../support/rule_transport.rs"]
mod rule_transport;
#[path = "../support/mod.rs"]
mod support;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use support::*;
struct ConnectedFixture {
    definitions: tempfile::TempDir,
    library: changes::ChangeFixture,
    caller: tempfile::TempDir,
}
impl ConnectedFixture {
    fn new() -> Self {
        let definitions = tempfile::tempdir().unwrap();
        let library = changes::ChangeFixture::new();
        library.write("lib.ts", "export function target() { return 1; }");
        library.stage("lib.ts");
        library.commit();
        library.write("lib.ts", "export function target() { return 2; }");
        let caller = tempfile::tempdir().unwrap();
        std::fs::write(caller.path().join("caller.ts"),"import { target as alias } from '@library/lib';\nfunction caller() { return alias(); }").unwrap();
        std::fs::write(
            caller.path().join("target.md"),
            "U001: separate valid target\n",
        )
        .unwrap();
        let set = json!({"schema_version":1,"name":"connected process","description":"one required group","inputs":{"changes":{"kind":"change_set"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]});
        std::fs::write(definitions.path().join("set.json"), set.to_string()).unwrap();
        let mut rule = rule_fixture::rule("connected");
        rule["evidence"][0]["input"] = json!("changes");
        rule["evidence"][0]["select"] = changes::related();
        std::fs::write(definitions.path().join("rules.jsonl"), rule.to_string()).unwrap();
        Self {
            definitions,
            library,
            caller,
        }
    }
    fn rules_input(&self, endpoint: &str) -> Value {
        let mut input =
            serde_json::to_value(rule_fixture::input(self.definitions.path(), endpoint)).unwrap();
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] =
            json!({"changes":{"kind":"change_set","repository":"library"}});
        input["repositories"] = json!([self.library.binding("library",sunstack_evidence::ChangeScope::Uncommitted{layers:vec![sunstack_evidence::DirtyLayer::Unstaged]}),{"key":"caller","root":self.caller.path(),"input_paths":[{"kind":"glob","pattern":"*.ts"},{"kind":"exact","path":"target.md"}],"change_scope":{"kind":"full"}}]);
        input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        input["package_bindings"] = json!([{"importing_repository_id":"caller","module_prefix":"@library","target_repository_id":"library","target_path_prefix":"","language":"typescript"}]);
        input["limits"]["max_files"] = json!(100);
        input
    }
    fn evidence_input(&self, endpoint: &str) -> Value {
        let rules = self.rules_input(endpoint);
        let mut input = evidence_input(self.definitions.path());
        for field in ["repositories", "discovery", "package_bindings", "limits"] {
            input[field] = rules[field].clone();
        }
        input["sources"] = json!({"changes":{"kind":"change_set","repository":"library"}});
        input["declarations"] = json!([{"id":"connected","input":"changes","requirement":"required","select":changes::related()}]);
        input
    }
}
fn check_excerpts(group: &Value) {
    let excerpts = group["excerpts"].as_array().unwrap();
    for (repository, path, start, text, signature) in [
        (
            "library",
            "lib.ts",
            7,
            "function target() { return 2; }",
            "function target()",
        ),
        (
            "caller",
            "caller.ts",
            48,
            "function caller() { return alias(); }",
            "function caller()",
        ),
    ] {
        let excerpt = excerpts
            .iter()
            .find(|excerpt| excerpt["unit"]["repository"] == repository)
            .unwrap();
        assert_eq!(excerpt["text"], text);
        assert_eq!(excerpt["unit"]["path"], path);
        assert_eq!(excerpt["unit"]["revision"]["side"], "after");
        assert_eq!(excerpt["span"]["byte_start"], start);
        assert_eq!(excerpt["span"]["byte_end"], start + text.len());
        assert_eq!(excerpt["membership"], json!({"kind":"whole_unit"}));
        assert_eq!(excerpt["context"]["enclosing_signature"], signature);
        if repository == "caller" {
            assert_eq!(excerpt["context"]["import_context"][0]["byte_start"], 0);
            assert_eq!(excerpt["context"]["import_context"][0]["byte_end"], 47);
        }
    }
}
#[tokio::test]
async fn root_package_mapping_connects_one_coherent_group_through_all_three_process_operations() {
    let fixture = ConnectedFixture::new();
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let evidence = response(
        &run(
            fixture.definitions.path(),
            "prepare_evidence",
            fixture.evidence_input(&server.endpoint),
        )
        .await,
        0,
    );
    let outcome = &evidence["result"]["outcome"];
    assert_eq!(outcome["kind"], "prepared");
    let selected = &outcome["selections"][0]["readiness"];
    assert_eq!(selected["content"]["kind"], "related_group");
    assert_eq!(selected["content"]["group"]["kind"], "complete");
    check_excerpts(&selected["content"]["group"]["group"]);
    assert_eq!(outcome["change_selections"].as_array().unwrap().len(), 1);
    assert_eq!(outcome["change_selections"][0]["repository"], "library");
    assert_eq!(server.count(), 0);
    let input = fixture.rules_input(&server.endpoint);
    let prepared = response(
        &run(fixture.definitions.path(), "prepare_rules", input.clone()).await,
        0,
    );
    assert_eq!(server.count(), 0);
    assert_eq!(
        prepared["result"]["outcome"]["preparation"]["occurrences"][0]["status"]["selection"]["selected_by"],
        json!(["full_scope"])
    );
    let evaluated = response(
        &run(fixture.definitions.path(), "evaluate_rules", input).await,
        0,
    );
    let report = &evaluated["result"]["outcome"]["report"];
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["full_scope"])
    );
    let units = report["occurrences"][0]["judgment"]["target"]["units"]
        .as_array()
        .unwrap();
    assert!(
        units
            .iter()
            .any(|unit| unit["repository"] == "library" && unit["revision"]["side"] == "after")
    );
    assert_eq!(report["change_selections"].as_array().unwrap().len(), 1);
    assert_eq!(report["change_selections"][0]["repository"], "library");
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 1);
    assert_eq!(bodies[0]["questions"].as_object().unwrap().len(), 1);
    let attribution = &bodies[0]["state"]["evidence"]["rule:connected/target"];
    assert_eq!(attribution["kind"], "related_group");
    let content = &bodies[0]["state"]["contents"][attribution["content"].as_str().unwrap()];
    assert_eq!(content["kind"], "complete");
    check_excerpts(&content["group"]);
    let library = content["group"]["excerpts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["unit"]["repository"] == "library")
        .unwrap();
    assert_eq!(
        library["span"]["source"],
        report["change_selections"][0]["members"][0]["changed_paths"][0]["after"]
    );
    record_observation(
        "m2-connected-root",
        json!({"evidence":evidence,"preparation":prepared,"report":evaluated,"http":bodies,"connections":count}),
    );
}
#[tokio::test]
async fn missing_mapping_disabled_discovery_and_competing_rule_seed_keep_distinct_process_outcomes()
{
    for case in [
        "missing_mapping",
        "disabled",
        "competing_seed",
        "whole_unit_extra",
    ] {
        let fixture = ConnectedFixture::new();
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let mut input = fixture.rules_input(&server.endpoint);
        if case == "missing_mapping" {
            input["package_bindings"] = json!([]);
        }
        if case == "disabled" {
            input["discovery"] = json!({"kind":"none"});
        }
        if case == "whole_unit_extra" {
            let path = fixture.definitions.path().join("rules.jsonl");
            let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            rule["evidence"][0]["select"]["chunk_settings"]["seed_content"]["AUTHOR_SECRET_KEY"] =
                json!(true);
            std::fs::write(path, rule.to_string()).unwrap();
        }
        if case == "competing_seed" {
            let path = fixture.definitions.path().join("rules.jsonl");
            let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            rule["evidence"][0]["select"]["seeds"] = json!([{"repository":"library","selector":{"kind":"file_text","source_path":"lib.ts"}}]);
            std::fs::write(path, rule.to_string()).unwrap();
        }
        for operation in ["prepare_rules", "evaluate_rules"] {
            let wire = response(
                &run(fixture.definitions.path(), operation, input.clone()).await,
                if case == "missing_mapping" { 3 } else { 2 },
            );
            assert_eq!(wire["kind"], "operation_result");
            assert_eq!(wire["result"]["operation"], operation);
            let outcome = &wire["result"]["outcome"];
            if case == "missing_mapping" {
                assert_eq!(
                    outcome["kind"],
                    if operation == "prepare_rules" {
                        "prepared_with_failures"
                    } else {
                        "incomplete_report"
                    }
                );
                assert!(outcome.to_string().contains("missing_related_context"));
            } else {
                assert_eq!(outcome["kind"], "invalid");
                assert_eq!(
                    outcome["diagnostics"]["items"][0]["code"],
                    if case == "disabled" {
                        "discovery_not_enabled"
                    } else {
                        "unknown_field"
                    }
                );
            }
        }
        assert_eq!(server.finish().await.0, 0);
    }
}
#[tokio::test]
async fn partial_connected_context_retains_its_diagnostics_omissions_and_independent_ready_peer() {
    for required in [true, false] {
        let fixture = ConnectedFixture::new();
        let mut set: Value = serde_json::from_slice(
            &std::fs::read(fixture.definitions.path().join("set.json")).unwrap(),
        )
        .unwrap();
        set["inputs"]["subject"] = json!({"kind":"document"});
        set["rule_paths"] = json!(["rules.jsonl", "peer.jsonl"]);
        std::fs::write(fixture.definitions.path().join("set.json"), set.to_string()).unwrap();
        let mut rule = rule_fixture::rule("connected");
        let mut select = changes::related();
        select["chunk_settings"]["max_chunk_bytes"] = json!(1);
        rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"changes","requirement":if required{"required"}else{"optional"},"select":select}));
        std::fs::write(
            fixture.definitions.path().join("rules.jsonl"),
            rule.to_string(),
        )
        .unwrap();
        std::fs::write(
            fixture.definitions.path().join("peer.jsonl"),
            rule_fixture::rule("peer").to_string(),
        )
        .unwrap();
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let mut input = fixture.rules_input(&server.endpoint);
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["subject"] =
            json!({"kind":"document","repository":"caller","path":"target.md"});
        let prepared = response(
            &run(fixture.definitions.path(), "prepare_rules", input.clone()).await,
            if required { 3 } else { 0 },
        );
        assert_eq!(server.count(), 0);
        let preparation = &prepared["result"]["outcome"]["preparation"];
        if !required {
            let results = preparation["planned_requests"][0]["readiness"]["evidence"]
                .as_array()
                .unwrap();
            let result = results
                .iter()
                .find(|result| result["id"] == "context")
                .unwrap();
            assert_eq!(result["readiness"]["content"]["group"]["kind"], "partial");
            assert!(
                !result["readiness"]["omissions"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                result["readiness"]["content"]["group"]["diagnostics"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value["code"] == "required_group_incomplete")
            );
        }
        let evaluated = response(
            &run(fixture.definitions.path(), "evaluate_rules", input).await,
            if required { 3 } else { 0 },
        );
        let report = &evaluated["result"]["outcome"]["report"];
        assert_eq!(report["occurrences"][1]["kind"], "judged");
        assert_eq!(
            report["occurrences"][0]["kind"],
            if required {
                "selection_failed"
            } else {
                "judged"
            }
        );
        let (count, bodies) = server.finish().await;
        assert_eq!(count, 1);
        assert_eq!(
            bodies[0]["questions"].as_object().unwrap().len(),
            if required { 1 } else { 2 }
        );
        assert!(
            bodies[0]["questions"]
                .as_object()
                .unwrap()
                .values()
                .any(|question| question["instructions"]["question"] == "judge peer")
        );
        let attribution = &bodies[0]["state"]["evidence"]["rule:connected/context"];
        if required {
            assert!(attribution.is_null());
        } else {
            let content = &bodies[0]["state"]["contents"][attribution["content"].as_str().unwrap()];
            assert_eq!(content["kind"], "partial");
            assert_eq!(attribution["omissions"], content["omissions"]);
            assert!(!attribution["omissions"].as_array().unwrap().is_empty());
            assert!(
                content["diagnostics"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|diagnostic| diagnostic["code"] == "required_group_incomplete")
            );
        }
        record_observation(
            if required {
                "m2-required-context-refusal"
            } else {
                "m2-optional-partial-context"
            },
            json!({"preparation":prepared,"evaluation":evaluated,"http":bodies,"connections":count}),
        );
    }
}
