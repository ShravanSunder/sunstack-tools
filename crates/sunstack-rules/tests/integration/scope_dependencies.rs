#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::scope_fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use scope_fixture::{ScopeFixture, changes, fixture};
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;
fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
#[tokio::test]
async fn leading_stops_keep_path_resolution_but_skip_refused_content_and_keep_a_ready_peer() {
    for set_level in [false, true] {
        for enabled in [false, true] {
            let fixture = ScopeFixture::new();
            let root = fixture.repository.root.path();
            let poison = tempfile::tempdir().unwrap();
            std::fs::write(
                poison.path().join("source.md"),
                "U001: deliberately unused source\n",
            )
            .unwrap();
            let condition = json!({"kind":"condition","test":{"kind":"payload_field_equals","input":"flags","pointer":"/enabled","value":true}});
            let mut rule = fixture::rule("changed");
            if !set_level {
                rule["applicability"] = json!([condition.clone()]);
            }
            fixture.repository.write("changed.jsonl", &rule.to_string());
            let set = json!({"schema_version":1,"name":"stopped","description":"no eager Git","inputs":{"subject":{"kind":"document"},"flags":{"kind":"payload","schema":"flags"}},"applicability":if set_level{json!([condition])}else{json!([])},"common_ruleset_instructions":null,"rule_paths":["changed.jsonl"]});
            fixture.repository.write("set.json", &set.to_string());
            let ready = json!({"schema_version":1,"name":"ready","description":"independent ready peer","inputs":{"peer":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["ready.jsonl"]});
            fixture.repository.write("ready.json", &ready.to_string());
            let mut ready_rule = fixture::rule("ready");
            ready_rule["evidence"][0]["input"] = json!("peer");
            ready_rule["evidence"][0]["select"]["target_id"] = json!("U002");
            fixture
                .repository
                .write("ready.jsonl", &ready_rule.to_string());
            fixture.repository.write("flags.schema.json",r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"],"additionalProperties":false}"#);
            for path in [
                "changed.jsonl",
                "set.json",
                "ready.json",
                "ready.jsonl",
                "flags.schema.json",
            ] {
                fixture.repository.stage(path);
            }
            fixture.repository.commit();
            fixture
                .repository
                .write("source.md", "U001: original\n\nU002: ready peer changed\n");
            let mut server = controlled::ControlledServer::start().await;
            let mut value = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
            value["repositories"].as_array_mut().unwrap().push(json!({"key":"poison","root":poison.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"revision_diff","base_revision":"unresolvable","head":{"kind":"worktree"}}}));
            value["payload_schemas"] = json!([{"name":"flags","path":"flags.schema.json"}]);
            value["definitions"]["rule_sets"] = json!([
                {"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"poison","path":"source.md"},"flags":{"kind":"payload","value":{"enabled":enabled}}}}]},
                {"path":"ready.json","binding_groups":[{"bindings":{"peer":{"kind":"document","repository":"repo","path":"source.md"}}}]}
            ]);
            let input: RuleInvocationInput = serde_json::from_value(value).unwrap();
            let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
            let request = server.next().await;
            assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
            assert_eq!(
                request.body["questions"]["q000001"]["instructions"]["question"],
                "judge ready"
            );
            request.noul(0.9);
            let outcome = task.await.unwrap();
            let report = match outcome {
                RuleInvocationOutcome::CompleteReport { report } if !enabled => report,
                RuleInvocationOutcome::IncompleteReport { report } if enabled => report,
                other => panic!("{other:?}"),
            };
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::Judged { .. }
            ));
            if enabled {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::SelectionFailed { .. }
                ));
                assert!(
                    report
                        .change_selections
                        .iter()
                        .any(|selection| selection.repository.as_str() == "poison")
                );
            } else {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::NotApplicable { .. }
                ));
                assert!(
                    report
                        .change_selections
                        .iter()
                        .any(|selection| selection.repository.as_str() == "poison")
                );
                assert!(report.captures.iter().all(|capture|!matches!(&capture.source,SourceRef::RepositoryFile{repository,..} if repository.as_str()=="poison")));
            }
            assert_eq!(server.finish().await, 1);
            assert!(root.exists());
        }
    }
}

#[tokio::test]
async fn each_nontracked_context_or_condition_dependency_prevents_false_irrelevance() {
    for case in [
        "full_document",
        "conversation",
        "payload",
        "payload_condition",
        "origin_condition",
        "outside_definitions",
    ] {
        let fixture = ScopeFixture::new();
        let root = fixture.repository.root.path();
        let external = tempfile::tempdir().unwrap();
        std::fs::write(external.path().join("context.md"), "external context").unwrap();
        let transcript = external.path().join("session.jsonl");
        std::fs::write(&transcript,b"{\"type\":\"user\",\"uuid\":\"one\",\"message\":{\"role\":\"user\",\"content\":\"external conversation\"}}\n").unwrap();
        let mut set: Value =
            serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
        set["rule_paths"] = json!(["changed.jsonl"]);
        let mut rule = fixture::rule("changed");
        let mut binding = None;
        let mut payload = false;
        match case {
            "full_document" => {
                set["inputs"]["external"] = json!({"kind":"document"});
                binding =
                    Some(json!({"kind":"document","repository":"external","path":"context.md"}));
                rule["evidence"].as_array_mut().unwrap().push(
                    json!({"id":"context","input":"external","select":{"kind":"whole_document"}}),
                );
            }
            "conversation" => {
                set["inputs"]["external"] = json!({"kind":"conversation"});
                binding = Some(
                    json!({"kind":"conversation","transcript_path":transcript,"format":"claude_code_jsonl"}),
                );
                rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"external","select":{"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"excluded","tool_use":{"kind":"excluded"},"hook_injected":"excluded"}}));
            }
            "payload" | "payload_condition" => {
                payload = true;
                set["inputs"]["external"] = json!({"kind":"payload","schema":"external"});
                binding = Some(json!({"kind":"payload","value":{"enabled":true}}));
                if case == "payload" {
                    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"external","select":{"kind":"whole_payload"}}));
                } else {
                    rule["applicability"] = json!([{"kind":"condition","test":{"kind":"payload_field_equals","input":"external","pointer":"/enabled","value":true}}]);
                }
            }
            "origin_condition" => {
                rule["applicability"] = json!([{"kind":"condition","test":{"kind":"origin_field_equals","field":"host","value":"fixture"}}]);
            }
            "outside_definitions" => {}
            _ => panic!("case"),
        }
        fixture.repository.write("set.json", &set.to_string());
        fixture.repository.write("changed.jsonl", &rule.to_string());
        fixture.repository.write("external.schema.json",r#"{"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"],"additionalProperties":false}"#);
        for path in ["set.json", "changed.jsonl", "external.schema.json"] {
            fixture.repository.stage(path);
        }
        fixture.repository.commit();
        let server = controlled::ControlledServer::start().await;
        let mut value = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
        if let Some(binding) = binding {
            value["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["external"] =
                binding;
        }
        if payload {
            value["payload_schemas"] = json!([{"name":"external","path":"external.schema.json"}]);
        }
        if case == "full_document" {
            value["repositories"].as_array_mut().unwrap().push(json!({"key":"external","root":external.path(),"input_paths":[{"kind":"exact","path":"context.md"}],"change_scope":{"kind":"full"}}));
        }
        if case == "origin_condition" {
            value["origin"] = json!({"kind":"hook","host":"fixture","event":"test"});
        }
        if case == "outside_definitions" {
            std::fs::write(external.path().join("set.json"), set.to_string()).unwrap();
            std::fs::write(external.path().join("changed.jsonl"), rule.to_string()).unwrap();
            value["definition_root"] = json!(external.path());
        }
        let outcome = prepare_rules(
            serde_json::from_value(value).unwrap(),
            context(&server.endpoint),
        )
        .await;
        let RulePreparationOutcome::Prepared { preparation } = outcome else {
            panic!("{case}: {outcome:?}")
        };
        let serialized = serde_json::to_value(preparation).unwrap();
        assert_eq!(
            serialized["occurrences"][0]["status"]["selection"]["selected_by"],
            json!(["full_scope"]),
            "{case}"
        );
        assert_eq!(server.finish().await, 0);
    }
}
#[tokio::test]
async fn pattern_condition_compares_its_whole_input_even_when_the_condition_stays_true() {
    for set_level in [false, true] {
        let fixture = ScopeFixture::new();
        let root = fixture.repository.root.path();
        let condition = json!({"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"original"},"max_scan_bytes":10000}});
        if set_level {
            let mut set: Value =
                serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
            set["applicability"] = json!([condition]);
            fixture.repository.write("set.json", &set.to_string());
            fixture.repository.stage("set.json");
        } else {
            let mut rule = fixture::rule("changed");
            rule["applicability"] = json!([condition]);
            fixture.repository.write("changed.jsonl", &rule.to_string());
            fixture.repository.stage("changed.jsonl");
        }
        fixture.repository.commit();
        fixture.repository.write(
            "source.md",
            "U001: original\n\nU002: changed outside the target paragraph\n",
        );
        let server = controlled::ControlledServer::start().await;
        let outcome =
            prepare_rules(fixture.input(&server.endpoint), context(&server.endpoint)).await;
        let RulePreparationOutcome::Prepared { preparation } = outcome else {
            panic!("{outcome:?}")
        };
        let serialized = serde_json::to_value(preparation).unwrap();
        assert_eq!(
            serialized["occurrences"][0]["status"]["selection"]["selected_by"],
            json!(["applicability_changed"])
        );
        assert_eq!(server.finish().await, 0);
    }
}
#[tokio::test]
async fn repeated_groups_reuse_captures_and_report_one_resolved_binding_joined_to_the_judged_source()
 {
    let fixture = ScopeFixture::new();
    fixture
        .repository
        .write("source.md", "U001: changed\n\nU002: neighbor\n");
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture.input(&server.endpoint);
    let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
        panic!("owner")
    };
    let group = rule_sets[0].binding_groups[0].clone();
    rule_sets[0].binding_groups.push(group);
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let first = server.next().await;
    let second = server.next().await;
    second.noul(0.9);
    first.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.change_selections.len(), 1);
    assert_eq!(report.measurements.files_read, 8);
    assert_eq!(report.occurrences.len(), 4);
    let after = report.change_selections[0].members[0].changed_paths[0]
        .after
        .as_ref()
        .unwrap();
    for index in [0, 2] {
        let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[index] else {
            panic!("not judged")
        };
        let TargetLocation::Paragraph { span, .. } = &judgment.target else {
            panic!("not paragraph")
        };
        assert_eq!(&span.source, after);
    }
    assert_eq!(report.occurrences[0].key().group, 0);
    assert_eq!(report.occurrences[2].key().group, 1);
    assert_eq!(
        serde_json::to_value(&report).unwrap()["change_selections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn unresolved_current_condition_keeps_the_path_selection_without_a_fabricated_full_scope_reason()
 {
    let fixture = ScopeFixture::new();
    let root = fixture.repository.root.path();
    let mut set: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    set["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"original"},"max_scan_bytes":1}}]);
    fixture.repository.write("set.json", &set.to_string());
    fixture.repository.stage("set.json");
    fixture.repository.commit();
    fixture
        .repository
        .write("source.md", "U001: current change\n\nU002: neighbor\n");
    let server = controlled::ControlledServer::start().await;
    let outcome = evaluate_rules(fixture.input(&server.endpoint), context(&server.endpoint)).await;
    let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(
        report.occurrences.iter().all(|occurrence| matches!(
            occurrence,
            RuleOccurrenceResult::EvaluationFailed {
                cause: Cause::Condition { .. },
                selection: OccurrenceSelection::ComparisonPending {},
                ..
            }
        )),
        "{:?}",
        report.occurrences
    );
    assert_eq!(report.change_selections.len(), 1);
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn unchanged_paths_skip_leading_scans_entirely() {
    let fixture = ScopeFixture::new();
    let root = fixture.repository.root.path();
    let mut set: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    set["applicability"] = json!([{"kind":"condition","test":{"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"original"},"max_scan_bytes":1}}]);
    fixture.repository.write("set.json", &set.to_string());
    fixture.repository.stage("set.json");
    fixture.repository.commit();
    let server = controlled::ControlledServer::start().await;
    let outcome = evaluate_rules(fixture.input(&server.endpoint), context(&server.endpoint)).await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(
        report.occurrences.iter().all(|occurrence| matches!(
            occurrence,
            RuleOccurrenceResult::OutsideChangeScope { .. }
        )),
        "{:?}",
        report.occurrences
    );
    assert_eq!(report.measurements.bytes_scanned, 0);
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn distinct_group_sources_share_one_cumulative_capture_budget() {
    let fixture = ScopeFixture::new();
    let second = changes::ChangeFixture::new();
    second.write("source.md", "U001: second old\n\nU002: second peer\n");
    second.stage("source.md");
    second.commit();
    second.write("source.md", "U001: second changed\n\nU002: second peer\n");
    fixture
        .repository
        .write("source.md", "U001: first changed\n\nU002: neighbor\n");
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture.input(&server.endpoint);
    input.repositories[0].input_paths = vec![QueryPath::Exact {
        path: "source.md".into(),
    }];
    let mut binding = second.binding(
        "second",
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Unstaged],
        },
    );
    binding.input_paths = vec![QueryPath::Exact {
        path: "source.md".into(),
    }];
    input.repositories.push(binding);
    let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
        panic!("discovery")
    };
    limits.max_files = 3.try_into().unwrap();
    let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
        panic!("owner")
    };
    let mut group = rule_sets[0].binding_groups[0].clone();
    group.bindings.insert(
        SourceName::new("subject").unwrap(),
        InputBinding::Document {
            repository: RepositoryId::new("second").unwrap(),
            path: "source.md".into(),
        },
    );
    rule_sets[0].binding_groups.push(group);
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 2);
    request.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::IncompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.measurements.files_read, 3);
    assert_eq!(report.change_selections.len(), 2);
    assert!(
        report.occurrences[..2]
            .iter()
            .all(|occurrence| matches!(occurrence, RuleOccurrenceResult::Judged { .. }))
    );
    for occurrence in &report.occurrences[2..] {
        let RuleOccurrenceResult::SelectionFailed { diagnostics, .. } = occurrence else {
            panic!("{occurrence:?}")
        };
        assert!(diagnostics.items.iter().any(|diagnostic|matches!(diagnostic,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==EvidenceCode::DiscoveryLimit)),"{diagnostics:?}");
    }
    assert_eq!(server.finish().await, 1);
}
