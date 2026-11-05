#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::change_fixture as changes;
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;

struct ConnectedFixture {
    definitions: tempfile::TempDir,
    library: changes::ChangeFixture,
    caller: tempfile::TempDir,
}
impl ConnectedFixture {
    fn new() -> Self {
        let library = changes::ChangeFixture::new();
        library.write("lib.ts", "export function target() { return 1; }");
        library.stage("lib.ts");
        library.commit();
        library.write("lib.ts", "export function target() { return 2; }");
        let caller = tempfile::tempdir().unwrap();
        std::fs::write(
            caller.path().join("caller.ts"),
            "import { target as alias } from '@library';\nfunction caller() { return alias(); }",
        )
        .unwrap();
        let definitions = tempfile::tempdir().unwrap();
        let set = json!({"schema_version":1,"name":"connected","description":"literal cross-repository rule","inputs":{"changes":{"kind":"change_set"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]});
        std::fs::write(definitions.path().join("set.json"), set.to_string()).unwrap();
        let mut rule = fixture::rule("connected");
        rule["evidence"][0]["input"] = json!("changes");
        rule["evidence"][0]["select"] = changes::related();
        std::fs::write(definitions.path().join("rules.jsonl"), rule.to_string()).unwrap();
        Self {
            definitions,
            library,
            caller,
        }
    }
    fn input(&self, endpoint: &str, mapped: bool) -> RuleInvocationInput {
        let mut value =
            serde_json::to_value(fixture::input(self.definitions.path(), endpoint)).unwrap();
        value["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] =
            json!({"changes":{"kind":"change_set","repository":"library"}});
        value["repositories"] = json!([
            self.library.binding(
                "library",
                ChangeScope::Uncommitted {
                    layers: vec![DirtyLayer::Unstaged]
                }
            ),
            RepositoryBinding {
                key: RepositoryId::new("caller").unwrap(),
                root: self.caller.path().into(),
                input_paths: vec![QueryPath::Glob {
                    pattern: "*.ts".into()
                }],
                change_scope: ChangeScope::Full {}
            }
        ]);
        value["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        value["limits"]["max_files"] = json!(30);
        value["package_bindings"] = if mapped {
            json!([{"importing_repository_id":"caller","module_prefix":"@library","target_repository_id":"library","target_path_prefix":"lib.ts","language":"typescript"}])
        } else {
            json!([])
        };
        serde_json::from_value(value).unwrap()
    }
}
fn context(endpoint: &str) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(transport::backend(endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}
#[tokio::test]
async fn connected_rule_forwards_mapping_one_group_and_exact_cross_repository_evidence_to_http() {
    let fixture = ConnectedFixture::new();
    let mut server = controlled::ControlledServer::start().await;
    let input = fixture.input(&server.endpoint, true);
    let prepared = prepare_rules(input.clone(), context(&server.endpoint)).await;
    assert!(
        matches!(prepared, RulePreparationOutcome::Prepared { .. }),
        "{prepared:?}"
    );
    let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
    let state = &request.body["state"];
    let attribution = &state["evidence"]["rule:connected/target"];
    assert_eq!(attribution["kind"], "related_group");
    assert_eq!(attribution["coverage"]["kind"], "change_set");
    let content = &state["contents"][attribution["content"].as_str().unwrap()];
    let excerpts = content["group"]["excerpts"].as_array().unwrap();
    assert!(
        excerpts
            .iter()
            .any(|value| value["text"] == "function target() { return 2; }"
                && value["unit"]["repository"] == "library")
    );
    assert!(excerpts.iter().any(
        |value| value["text"] == "function caller() { return alias(); }"
            && value["unit"]["repository"] == "caller"
    ));
    assert!(
        excerpts
            .iter()
            .all(|value| value["membership"]["kind"] == "whole_unit")
    );
    for (repository, path, start_line, start_byte, text, signature) in [
        (
            "library",
            "lib.ts",
            1,
            7,
            "function target() { return 2; }",
            "function target()",
        ),
        (
            "caller",
            "caller.ts",
            2,
            44,
            "function caller() { return alias(); }",
            "function caller()",
        ),
    ] {
        let excerpt = excerpts
            .iter()
            .find(|value| value["unit"]["repository"] == repository)
            .unwrap();
        assert_eq!(excerpt["unit"]["path"], path);
        assert_eq!(excerpt["unit"]["revision"]["side"], "after");
        assert_eq!(excerpt["span"]["start_line"], start_line);
        assert_eq!(excerpt["span"]["byte_start"], start_byte);
        assert_eq!(excerpt["span"]["byte_end"], start_byte + text.len());
        assert_eq!(excerpt["context"]["enclosing_signature"], signature);
        if repository == "caller" {
            let imports = excerpt["context"]["import_context"].as_array().unwrap();
            assert_eq!(imports.len(), 1);
            assert_eq!(imports[0]["byte_start"], 0);
            assert_eq!(imports[0]["byte_end"], 43);
            assert_eq!(imports[0]["source"]["repository"], "caller");
        }
    }
    request.noul(0.9);
    let outcome = task.await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    let RuleOccurrenceResult::Judged { judgment, .. } = &report.occurrences[0] else {
        panic!("not judged")
    };
    assert!(
        matches!(&judgment.target,TargetLocation::CodeUnits{units} if units.iter().any(|unit|unit.repository.as_str()=="library"))
    );
    assert_eq!(report.measurements.files_read, 3);
    assert_eq!(report.change_selections.len(), 1);
    assert_eq!(report.change_selections[0].repository.as_str(), "library");
    assert_eq!(server.finish().await, 1);
}
#[tokio::test]
async fn missing_mapping_and_disabled_discovery_are_distinct_without_inference() {
    for enabled in [true, false] {
        let fixture = ConnectedFixture::new();
        let server = controlled::ControlledServer::start().await;
        let mut input = fixture.input(&server.endpoint, false);
        if !enabled {
            input.discovery = EvidenceDiscovery::None {};
        }
        let outcome = evaluate_rules(input, context(&server.endpoint)).await;
        if enabled {
            assert!(
                matches!(outcome,RuleInvocationOutcome::IncompleteReport{ref report} if matches!(report.occurrences[0],RuleOccurrenceResult::SelectionFailed{..})),
                "{outcome:?}"
            );
        } else {
            assert!(
                matches!(outcome,RuleInvocationOutcome::Invalid{ref diagnostics} if diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==EvidenceCode::DiscoveryNotEnabled))),
                "{outcome:?}"
            );
        }
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn competing_rule_seed_fields_refuse_before_session_open_and_data_words_remain_valid() {
    for field in [
        "seeds",
        "repository_ids",
        "seed_repository",
        "selector",
        "query",
        "source_units",
    ] {
        let fixture = ConnectedFixture::new();
        let server = controlled::ControlledServer::start().await;
        let path = fixture.definitions.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["evidence"][0]["select"][field] =
            json!({"repository":"library","selector":{"kind":"file_text","source_path":"lib.ts"}});
        std::fs::write(&path, rule.to_string()).unwrap();
        let expected_path = path.canonicalize().unwrap();
        let outcome = prepare_rules(
            fixture.input(&server.endpoint, true),
            context(&server.endpoint),
        )
        .await;
        let RulePreparationOutcome::Invalid { diagnostics } = outcome else {
            panic!("competing {field} was not rejected at admission: {outcome:?}")
        };
        assert!(diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Rule(diagnostic) if diagnostic.code==RuleDiagnosticCode::UnknownField && matches!(&diagnostic.location,RuleDiagnosticLocation::Definition{file,line:1,..} if file==&expected_path))),"{field}: {diagnostics:?}; expected {expected_path:?}");
        assert_eq!(server.finish().await, 0);
    }
    let fixture = ConnectedFixture::new();
    let server = controlled::ControlledServer::start().await;
    let path = fixture.definitions.path().join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["verdict"]["question"]["instructions"] = json!({"seeds":{"repository_ids":["data"]},"query":{"selector":"data","source_units":"data"}});
    std::fs::write(path, rule.to_string()).unwrap();
    let outcome = prepare_rules(
        fixture.input(&server.endpoint, true),
        context(&server.endpoint),
    )
    .await;
    assert!(
        matches!(outcome,RulePreparationOutcome::Prepared{ref preparation} if preparation.measurements.files_read==3),
        "{outcome:?}"
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn required_connected_failures_isolate_the_owner_and_optional_partial_context_reaches_http() {
    for (case, expected) in [
        ("unresolved", EvidenceCode::RelationshipUnresolved),
        ("minimum", EvidenceCode::MissingRelatedContext),
        ("overflow", EvidenceCode::RequiredGroupIncomplete),
        ("visits", EvidenceCode::DiscoveryLimit),
    ] {
        for required in [true, false] {
            let fixture = ConnectedFixture::new();
            fixture.library.write("lib.ts",if case=="unresolved" {"import { helper } from './helper'; export function target() { helper(); receiver.dynamic(); }"} else {"import { helper } from './helper'; export function target() { helper(); }"});
            fixture
                .library
                .write("helper.ts", "export function helper() { return 1; }");
            std::fs::write(
                fixture.caller.path().join("target.md"),
                "U001: complete target\n",
            )
            .unwrap();
            let mut set: Value = serde_json::from_slice(
                &std::fs::read(fixture.definitions.path().join("set.json")).unwrap(),
            )
            .unwrap();
            set["inputs"]["subject"] = json!({"kind":"document"});
            set["rule_paths"] = json!(["rules.jsonl", "peer.jsonl"]);
            std::fs::write(fixture.definitions.path().join("set.json"), set.to_string()).unwrap();
            let mut rule = fixture::rule("connected");
            let mut select = changes::related();
            select["directions"] = json!(["callees"]);
            if case == "minimum" {
                select["minimum_related_units"] = json!(100);
            }
            if case == "overflow" {
                select["chunk_settings"]["max_chunk_bytes"] = json!(1);
            }
            if case == "visits" {
                select["requested_hops"] = json!(2);
            }
            rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"changes","requirement":if required{"required"}else{"optional"},"select":select}));
            std::fs::write(
                fixture.definitions.path().join("rules.jsonl"),
                rule.to_string(),
            )
            .unwrap();
            std::fs::write(
                fixture.definitions.path().join("peer.jsonl"),
                fixture::rule("peer").to_string(),
            )
            .unwrap();
            let mut server = controlled::ControlledServer::start().await;
            let mut input = fixture.input(&server.endpoint, true);
            let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
                panic!("owner")
            };
            rule_sets[0].binding_groups[0].bindings.insert(
                SourceName::new("subject").unwrap(),
                InputBinding::Document {
                    repository: RepositoryId::new("caller").unwrap(),
                    path: "target.md".into(),
                },
            );
            input.repositories[1].input_paths.push(QueryPath::Exact {
                path: "target.md".into(),
            });
            if case == "visits" {
                let EvidenceDiscovery::Bounded { limits } = &mut input.discovery else {
                    panic!("discovery")
                };
                limits.max_unit_visits = 1.try_into().unwrap();
            }
            let task = tokio::spawn(evaluate_rules(input, context(&server.endpoint)));
            let request = server.next().await;
            assert_eq!(
                request.body["questions"].as_object().unwrap().len(),
                if required { 1 } else { 2 },
                "{case}/{required}: {:?}",
                request.body
            );
            assert!(
                request.body["questions"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|question| question["instructions"]["question"] == "judge peer")
            );
            let attribution = &request.body["state"]["evidence"]["rule:connected/context"];
            if required {
                assert!(attribution.is_null());
            } else {
                let content =
                    &request.body["state"]["contents"][attribution["content"].as_str().unwrap()];
                assert_eq!(attribution["kind"], "related_group");
                assert_eq!(content["kind"], "partial", "{case}: {content}");
                assert!(
                    content["diagnostics"]["items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|diagnostic| diagnostic["code"]
                            == serde_json::to_value(expected).unwrap()),
                    "{case}: {content}"
                );
                assert_eq!(
                    attribution["omissions"], content["omissions"],
                    "{case}: native partial omissions must survive"
                );
                if case == "overflow" {
                    assert!(
                        !attribution["omissions"].as_array().unwrap().is_empty(),
                        "{attribution}"
                    );
                }
            }
            request.noul(0.9);
            let outcome = task.await.unwrap();
            let report = match outcome {
                RuleInvocationOutcome::IncompleteReport { report } if required => report,
                RuleInvocationOutcome::CompleteReport { report } if !required => report,
                other => panic!("{case}/{required}: {other:?}"),
            };
            assert!(matches!(
                report.occurrences[1],
                RuleOccurrenceResult::Judged { .. }
            ));
            if required {
                let RuleOccurrenceResult::SelectionFailed { diagnostics, .. } =
                    &report.occurrences[0]
                else {
                    panic!("{case}: {:?}", report.occurrences)
                };
                assert!(diagnostics.items.iter().any(|diagnostic|matches!(diagnostic,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==expected)),"{case}: {diagnostics:?}");
            } else {
                assert!(matches!(
                    report.occurrences[0],
                    RuleOccurrenceResult::Judged { .. }
                ));
            }
            assert_eq!(server.finish().await, 1);
        }
    }
}

#[tokio::test]
async fn reused_related_selector_is_checked_against_each_owners_kind_correct_binding() {
    let fixture = ConnectedFixture::new();
    std::fs::write(
        fixture.caller.path().join("subject.md"),
        "U001: kind-correct document",
    )
    .unwrap();
    let mut set: Value = serde_json::from_slice(
        &std::fs::read(fixture.definitions.path().join("set.json")).unwrap(),
    )
    .unwrap();
    set["inputs"]["changes"] = json!({"kind":"document"});
    std::fs::write(
        fixture.definitions.path().join("document-set.json"),
        set.to_string(),
    )
    .unwrap();
    let server = controlled::ControlledServer::start().await;
    let mut value = serde_json::to_value(fixture.input(&server.endpoint, true)).unwrap();
    value["repositories"][1]["input_paths"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"exact","path":"subject.md"}));
    value["definitions"]["rule_sets"].as_array_mut().unwrap().push(json!({"path":"document-set.json","binding_groups":[{"bindings":{"changes":{"kind":"document","repository":"caller","path":"subject.md"}}}]}));
    let outcome = prepare_rules(
        serde_json::from_value(value).unwrap(),
        context(&server.endpoint),
    )
    .await;
    let RulePreparationOutcome::Invalid { diagnostics } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(diagnostics.items.iter().any(|diagnostic|matches!(diagnostic,ReportDiagnostic::Rule(diagnostic) if diagnostic.code==RuleDiagnosticCode::InputKindMismatch && matches!(&diagnostic.location,RuleDiagnosticLocation::Definition{file,line:1,..} if file.ends_with("rules.jsonl")))),"{diagnostics:?}");
    assert_eq!(server.finish().await, 0);
}
