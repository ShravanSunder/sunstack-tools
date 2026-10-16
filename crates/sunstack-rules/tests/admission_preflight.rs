#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::*;
use sunstack_rules::*;
struct CountingAdapter {
    format: TranscriptFormatName,
    calls: Arc<AtomicUsize>,
    builtins: AdapterRegistry,
}
impl TranscriptAdapter for CountingAdapter {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.format
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.builtins
            .adapter(&TranscriptFormatName::new("claude_code_jsonl").unwrap())
            .unwrap()
            .normalize(capture)
    }
}
fn conversation() -> Value {
    json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"excluded"},"hook_injected":"excluded"})
}
fn base(root: &std::path::Path, endpoint: &str) -> (Value, Value, Value) {
    std::fs::write(root.join("session.jsonl"),b"{\"type\":\"user\",\"uuid\":\"u1\",\"message\":{\"role\":\"user\",\"content\":\"SOURCE_CONTROL_TEXT\"}}\n").unwrap();
    let set = json!({"schema_version":1,"name":"admission","description":"source boundary","inputs":{"subject":{"kind":"conversation"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]});
    let mut rule = fixture::rule("one");
    rule["evidence"][0]["select"] = conversation();
    let mut input = serde_json::to_value(fixture::input(root, endpoint)).unwrap();
    input["repositories"] = json!([]);
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] = json!({"subject":{"kind":"conversation","transcript_path":root.join("session.jsonl"),"format":"counting_claude"}});
    (set, rule, input)
}
fn registry(calls: Arc<AtomicUsize>) -> AdapterRegistry {
    AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(Arc::new(CountingAdapter {
            format: TranscriptFormatName::new("counting_claude").unwrap(),
            calls,
            builtins: AdapterRegistry::builtins().unwrap(),
        }))
        .unwrap()
}
#[tokio::test]
async fn definition_and_binding_preflight_refusals_never_reach_source_capture_or_http() {
    let cases = [
        "set_unknown",
        "set_version",
        "set_inputs_empty",
        "set_paths_empty",
        "set_name_empty",
        "set_description_empty",
        "rule_unknown",
        "missing_verdict",
        "missing_threshold",
        "zero_threshold",
        "over_threshold",
        "invalid_rule_id",
        "evidence_empty",
        "unknown_target",
        "undeclared_reference",
        "selector_kind",
        "question_array",
        "choice_empty",
        "score_empty",
        "noul_partial",
        "decision_evidence_empty",
        "composite_empty",
        "duplicate_rule_id",
        "duplicate_rule_source",
        "empty_rule_file",
        "array_rule_line",
        "wrapped_rule_line",
        "malformed_rule_line",
        "binding_unbound",
        "binding_extra",
        "binding_kind",
        "duplicate_ruleset_path",
        "groups_empty",
        "sets_empty",
        "unknown_format",
    ];
    for case in cases {
        let root = tempfile::tempdir().unwrap();
        let server = controlled::ControlledServer::start().await;
        let (mut set, mut rule, mut input) = base(root.path(), &server.endpoint);
        let mut raw_rule = None;
        match case {
            "set_unknown" => set["PRIVATE_AUTHOR_KEY"] = json!("PRIVATE_AUTHOR_VALUE"),
            "set_version" => set["schema_version"] = json!(2),
            "set_inputs_empty" => set["inputs"] = json!({}),
            "set_paths_empty" => set["rule_paths"] = json!([]),
            "set_name_empty" => set["name"] = json!(""),
            "set_description_empty" => set["description"] = json!(""),
            "rule_unknown" => rule["PRIVATE_AUTHOR_KEY"] = json!("PRIVATE_AUTHOR_VALUE"),
            "missing_verdict" => {
                rule.as_object_mut().unwrap().remove("verdict");
            }
            "missing_threshold" => {
                rule["verdict"].as_object_mut().unwrap().remove("threshold");
            }
            "zero_threshold" => rule["verdict"]["threshold"] = json!(0),
            "over_threshold" => rule["verdict"]["threshold"] = json!(1.1),
            "invalid_rule_id" => rule["rule_id"] = json!("PRIVATE_AUTHOR_VALUE"),
            "evidence_empty" => rule["evidence"] = json!([]),
            "unknown_target" => rule["target"] = json!("missing"),
            "undeclared_reference" => rule["evidence"][0]["input"] = json!("missing"),
            "selector_kind" => {
                rule["evidence"][0]["select"] = json!({"kind":"paragraph","target_id":"U001"})
            }
            "question_array" => rule["questions"] = json!([]),
            "choice_empty" => {
                rule["questions"] =
                    json!({"detail":{"type":"choice","instructions":"select","criteria":{}}})
            }
            "score_empty" => {
                rule["questions"] =
                    json!({"detail":{"type":"score","instructions":"score","criteria":[]}})
            }
            "noul_partial" => rule["verdict"]["question"]["criteria"] = json!({"true":"only half"}),
            "decision_evidence_empty" => {
                set["applicability"] = json!([{"kind":"decision","question":{"type":"noul","instructions":"gate"},"evidence":[],"threshold":0.5}])
            }
            "composite_empty" => {
                rule["verdict"] = json!({"kind":"composite","dimensions":[],"threshold":0.8})
            }
            "duplicate_rule_id" => raw_rule = Some(format!("{}\n{}", rule, rule)),
            "duplicate_rule_source" => set["rule_paths"] = json!(["rules.jsonl", "./rules.jsonl"]),
            "empty_rule_file" => raw_rule = Some(" \n\n".into()),
            "array_rule_line" => raw_rule = Some(json!([rule.clone()]).to_string()),
            "wrapped_rule_line" => raw_rule = Some(json!({"rules":[rule.clone()]}).to_string()),
            "malformed_rule_line" => raw_rule = Some("{\"PRIVATE_AUTHOR_KEY\": ".into()),
            "binding_unbound" => {
                input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] = json!({})
            }
            "binding_extra" => {
                let binding = input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]
                    ["subject"]
                    .clone();
                input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["private_author_key"] =
                    binding;
            }
            "binding_kind" => {
                input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["subject"] =
                    json!({"kind":"payload","value":{}})
            }
            "duplicate_ruleset_path" => {
                let mut requested = input["definitions"]["rule_sets"][0].clone();
                requested["path"] = json!("./set.json");
                input["definitions"]["rule_sets"]
                    .as_array_mut()
                    .unwrap()
                    .push(requested);
            }
            "groups_empty" => input["definitions"]["rule_sets"][0]["binding_groups"] = json!([]),
            "sets_empty" => input["definitions"]["rule_sets"] = json!([]),
            "unknown_format" => {
                input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["subject"]["format"] =
                    json!("absent_format")
            }
            _ => panic!("unknown case"),
        }
        std::fs::write(
            root.path().join("set.json"),
            serde_json::to_vec(&set).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("rules.jsonl"),
            raw_rule.unwrap_or_else(|| rule.to_string()),
        )
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let outcome = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: registry(calls.clone()),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let RuleInvocationOutcome::Invalid { diagnostics } = outcome else {
            panic!("{case}: {outcome:?}")
        };
        assert!(!diagnostics.is_empty(), "{case}");
        let safe = serde_json::to_string(&diagnostics).unwrap();
        assert!(!safe.contains("PRIVATE_AUTHOR_VALUE"), "{case}");
        assert!(!safe.contains("PRIVATE_AUTHOR_KEY"), "{case}");
        assert!(!safe.contains("private_author_key"), "{case}");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{case}");
        assert_eq!(server.finish().await, 0, "{case}");
    }
}
#[tokio::test]
async fn empty_applicability_and_question_maps_and_reused_ids_across_rulesets_are_admitted() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    let (set, rule, mut input) = base(root.path(), &server.endpoint);
    std::fs::write(root.path().join("set.json"), set.to_string()).unwrap();
    std::fs::write(root.path().join("second.json"), set.to_string()).unwrap();
    std::fs::write(root.path().join("rules.jsonl"), format!("\n{rule}\n\n")).unwrap();
    let mut second = input["definitions"]["rule_sets"][0].clone();
    second["path"] = json!("second.json");
    input["definitions"]["rule_sets"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let calls = Arc::new(AtomicUsize::new(0));
    let execution = evaluate_rules(
        serde_json::from_value(input).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: registry(calls.clone()),
            cancellation: CancellationToken::new(),
        },
    );
    tokio::pin!(execution);
    let mut observed = 0;
    let outcome=tokio::time::timeout(Duration::from_secs(4),async{loop{tokio::select!{outcome=&mut execution=>break outcome,request=server.next()=>{observed+=1;assert!(serde_json::to_string(&request.body["state"]).unwrap().contains("SOURCE_CONTROL_TEXT"));request.noul(0.9);}}}}).await.unwrap();
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.occurrences.len(), 2);
    assert!(
        report
            .occurrences
            .iter()
            .all(|result| matches!(result, RuleOccurrenceResult::Judged { .. }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(report.measurements.transcript_bytes_read > 0);
    assert_eq!(observed, 2);
    assert_eq!(server.finish().await, 2);
}
#[test]
fn raw_nested_duplicate_keys_are_refused_before_any_typed_map_can_erase_them() {
    for raw in [
        r#"{"schema_version":1,"inputs":{"subject":{"kind":"document"},"subject":{"kind":"document"}}}"#,
        r#"{"rule_id":"one","questions":{"detail":{"type":"noul","instructions":"a"},"detail":{"type":"noul","instructions":"b"}}}"#,
        r#"{"rule_id":"one","verdict":{"kind":"noul","question":{"type":"noul","instructions":{"PRIVATE_AUTHOR_KEY":1,"PRIVATE_AUTHOR_KEY":2}},"threshold":0.8}}"#,
    ] {
        let diagnostics = if raw.contains("schema_version") {
            decode_rule_set(raw.as_bytes(), "set.json".as_ref()).unwrap_err()
        } else {
            decode_rule_definition(raw.as_bytes(), "rules.jsonl".as_ref(), 1).unwrap_err()
        };
        let safe = serde_json::to_string(&diagnostics).unwrap();
        assert!(safe.contains("duplicate_field"));
        assert!(!safe.contains("PRIVATE_AUTHOR_KEY"));
    }
    let root = tempfile::tempdir().unwrap();
    let (_, _, input) = base(root.path(), "http://127.0.0.1:1");
    let raw=input.to_string().replace("\"bindings\":{","\"bindings\":{\"subject\":{\"kind\":\"conversation\",\"transcript_path\":\"PRIVATE_AUTHOR_VALUE\",\"format\":\"counting_claude\"},");
    let diagnostics = decode_rule_invocation(raw.as_bytes()).unwrap_err();
    assert!(
        serde_json::to_string(&diagnostics)
            .unwrap()
            .contains("duplicate_field")
    );
    assert!(!format!("{diagnostics:?}").contains("PRIVATE_AUTHOR_VALUE"));
}

#[tokio::test]
async fn primitive_and_selector_diagnostics_keep_their_native_codes_and_definition_locations() {
    for owner in ["primitive", "selector"] {
        let root = tempfile::tempdir().unwrap();
        let server = controlled::ControlledServer::start().await;
        let (set, mut rule, input) = base(root.path(), &server.endpoint);
        if owner == "primitive" {
            rule["questions"] =
                json!({"detail":{"type":"score","instructions":"rate","criteria":[]}});
        } else {
            rule["evidence"][0]["select"] = json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"excluded","assistant_messages":"excluded","tool_use":{"kind":"excluded"},"hook_injected":"excluded"});
        }
        std::fs::write(root.path().join("set.json"), set.to_string()).unwrap();
        std::fs::write(root.path().join("rules.jsonl"), format!("\n{rule}\n")).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let outcome = evaluate_rules(
            serde_json::from_value(input).unwrap(),
            RuleExecutionContext {
                backend: Arc::new(transport::backend(&server.endpoint)),
                adapters: registry(calls.clone()),
                cancellation: CancellationToken::new(),
            },
        )
        .await;
        let RuleInvocationOutcome::Invalid { diagnostics } = outcome else {
            panic!("{outcome:?}")
        };
        let value = serde_json::to_value(&diagnostics).unwrap();
        assert_eq!(
            value["items"][0]["location"]["kind"], "definition",
            "{owner}: {value}"
        );
        assert_eq!(value["items"][0]["location"]["line"], 2);
        assert!(
            value["items"][0]["location"]["file"]
                .as_str()
                .unwrap()
                .ends_with("rules.jsonl")
        );
        assert_eq!(
            value["items"][0]["code"],
            if owner == "primitive" {
                "invalid_rubric"
            } else {
                "empty_projection"
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(server.finish().await, 0);
    }
}

#[tokio::test]
async fn shared_rule_file_is_validated_for_each_owner_with_kind_correct_bindings() {
    let root = tempfile::tempdir().unwrap();
    let server = controlled::ControlledServer::start().await;
    let (conversation_set, rule, mut input) = base(root.path(), &server.endpoint);
    let mut document_set = conversation_set.clone();
    document_set["inputs"]["subject"] = json!({"kind":"document"});
    std::fs::write(
        root.path().join("source.md"),
        "U001: valid document binding\n",
    )
    .unwrap();
    std::fs::write(root.path().join("set.json"), conversation_set.to_string()).unwrap();
    std::fs::write(root.path().join("second.json"), document_set.to_string()).unwrap();
    // Blank line makes the shared selector's exact owning definition line observable.
    std::fs::write(root.path().join("rules.jsonl"), format!("\n{rule}\n")).unwrap();
    input["repositories"] = json!([{"key":"repo","root":root.path(),"input_paths":[{"kind":"exact","path":"source.md"}],"change_scope":{"kind":"full"}}]);
    input["definitions"]["rule_sets"].as_array_mut().unwrap().push(json!({"path":"second.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"}}}]}));
    let calls = Arc::new(AtomicUsize::new(0));
    // Both owners' binding kinds are valid. The only incompatibility is the cached
    // shared rule's conversation selector under the second owner's document input.
    let outcome = prepare_rules(
        serde_json::from_value(input).unwrap(),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(&server.endpoint)),
            adapters: registry(calls.clone()),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let RulePreparationOutcome::Invalid { diagnostics } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(diagnostics.items.len(), 1);
    let ReportDiagnostic::Rule(diagnostic) = &diagnostics.items[0] else {
        panic!("{diagnostics:?}")
    };
    assert_eq!(diagnostic.code, RuleDiagnosticCode::InputKindMismatch);
    assert!(
        matches!(&diagnostic.location, RuleDiagnosticLocation::Definition {file,line:2,column:1,..} if file == &root.path().join("rules.jsonl").canonicalize().unwrap()),
        "{diagnostic:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(server.finish().await, 0);
}
