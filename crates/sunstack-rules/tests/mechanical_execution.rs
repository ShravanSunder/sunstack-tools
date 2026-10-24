#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[path = "support/change_fixture.rs"]
mod changes;
#[path = "support/contract_schemas.rs"]
mod schema_support;

fn mechanical_rule(id: &str, select: Value, check: Value) -> Value {
    let mut rule = fixture::rule(id);
    rule["evidence"][0]["select"] = select;
    rule["verdict"] = json!({"kind":"mechanical","explanation":"A local standard","check":check});
    rule
}
async fn evaluate(root: &std::path::Path) -> Value {
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let outcome = evaluate_rules(
        fixture::input(root, endpoint),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    serde_json::to_value(outcome).unwrap()
}

#[tokio::test]
async fn each_expect_form_checks_good_and_bad_units_without_any_model_request() {
    for (expect, text, passed) in [
        (
            json!({"matches":"MUST"}),
            "You MUST load the standard.\n",
            true,
        ),
        (
            json!({"matches":"MUST"}),
            "You may load the standard.\n",
            false,
        ),
        (
            json!({"not_matches":"forbidden"}),
            "Use the standard.\n",
            true,
        ),
        (
            json!({"not_matches":"forbidden"}),
            "Use the forbidden standard.\n",
            false,
        ),
        (json!({"starts_with":"Use"}), "Use the standard.\n", true),
        (json!({"starts_with":"Use"}), "Avoid the standard.\n", false),
        (
            json!({"matches":"MUST"}),
            "Use `MUST` only as an example.\n",
            false,
        ),
        (json!({"matches":"MUST"}), "```text\nMUST\n```\n", true),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let fenced = text.starts_with("```");
        let rule = mechanical_rule(
            "local.one",
            json!({"kind":"markdown_units","mode":"unit","units":if fenced {vec!["section"]} else {vec!["paragraph"]}}),
            json!({"kind":"expect","expect":expect}),
        );
        fixture::write_set(directory.path(), &[rule], vec![]);
        std::fs::write(
            directory.path().join("source.md"),
            if fenced {
                format!("## Standard\n\n{text}")
            } else {
                text.to_owned()
            },
        )
        .unwrap();
        let wire = evaluate(directory.path()).await;
        assert_eq!(wire["kind"], "complete_report", "{wire}");
        let report = &wire["report"];
        assert_eq!(report["measurements"]["model_requests"], 0);
        assert_eq!(report["measurements"]["inference_attempts"], 0);
        assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
        let result = &report["occurrences"][0];
        assert_eq!(result["kind"], "checked");
        assert_eq!(result["outcome"]["kind"], "completed");
        assert_eq!(
            result["outcome"]["result"],
            if passed { "pass" } else { "fail" }
        );
        for field in ["judgment", "answers", "confidence", "receipt"] {
            assert!(result.get(field).is_none(), "{result}");
        }
        assert!(
            report["requests"]
                .as_array()
                .unwrap()
                .iter()
                .all(|request| request["questions"].as_array().unwrap().is_empty())
        );
    }
}

#[tokio::test]
async fn frontmatter_present_absent_and_unavailable_remain_distinct_and_keep_a_completed_peer() {
    let reviewed = schema_support::reviewed("rule-invocation-report.schema.json");
    let generated = schema_support::generated::<RuleInvocationReport>();
    for (text, expect, completed, passed) in [
        (
            "---\ndescription: Ready\n---\n",
            json!({"matches":"^Ready$"}),
            true,
            true,
        ),
        (
            "---\ndescription: 12\n---\n",
            json!({"starts_with":"12"}),
            true,
            true,
        ),
        (
            "---\ndescription: false\n---\n",
            json!({"matches":"^false$"}),
            true,
            true,
        ),
        (
            "---\ndescription: null\n---\n",
            json!({"present":true}),
            true,
            true,
        ),
        (
            "---\ndescription: null\n---\n",
            json!({"matches":"^null$"}),
            true,
            true,
        ),
        (
            "---\ndescription: [a, b]\n---\n",
            json!({"matches":"^\\[\\\"a\\\",\\\"b\\\"\\]$"}),
            true,
            true,
        ),
        (
            "---\ndescription: {z: 1, a: 2}\n---\n",
            json!({"starts_with":"{\"a\":2,\"z\":1}"}),
            true,
            true,
        ),
        (
            "---\nname: other\n---\n",
            json!({"present":true}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"not_matches":"bad"}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"matches":".*"}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"starts_with":""}),
            true,
            false,
        ),
        ("## No header\n", json!({"present":true}), false, false),
        (
            "---\ndescription: [\n---\n",
            json!({"present":true}),
            false,
            false,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut field = mechanical_rule(
            "local.field",
            json!({"kind":"frontmatter_field","field":"description"}),
            json!({"kind":"expect","expect":expect}),
        );
        field["severity"] = json!("error");
        let mut peer = mechanical_rule(
            "local.peer",
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            json!({"kind":"expect","expect":{"starts_with":"U001:"}}),
        );
        peer["evidence"][0]["input"] = json!("other");
        fixture::write_set(directory.path(), &[field, peer], vec![]);
        std::fs::write(directory.path().join("source.md"), text).unwrap();
        let wire = evaluate(directory.path()).await;
        assert_eq!(
            wire["kind"],
            if completed {
                "complete_report"
            } else {
                "incomplete_report"
            },
            "{text}: {wire}"
        );
        let report = &wire["report"];
        assert!(reviewed.is_valid(report), "reviewed schema: {report}");
        assert!(generated.is_valid(report), "generated schema: {report}");
        let mut fabricated = report.clone();
        fabricated["occurrences"][0]["outcome"]["confidence"] = json!(0.9);
        assert!(!reviewed.is_valid(&fabricated));
        assert!(serde_json::from_value::<RuleInvocationReport>(fabricated).is_err());
        assert_eq!(report["measurements"]["model_requests"], 0);
        let results = report["occurrences"].as_array().unwrap();
        let field = results
            .iter()
            .find(|r| r["key"]["rule_id"] == "local.field")
            .unwrap();
        assert_eq!(field["kind"], "checked");
        assert_eq!(field["declared_severity"], "error");
        assert_eq!(
            field["outcome"]["kind"],
            if completed { "completed" } else { "unresolved" }
        );
        if completed {
            assert_eq!(
                field["outcome"]["result"],
                if passed { "pass" } else { "fail" }
            );
        }
        let peer = results
            .iter()
            .find(|r| r["key"]["rule_id"] == "local.peer")
            .unwrap();
        assert_eq!(peer["outcome"]["result"], "pass");
    }
}

async fn presence_outcome(input: RuleInvocationInput) -> (Value, u8) {
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let outcome = evaluate_rules(
        input,
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let exit = outcome.exit_code();
    (serde_json::to_value(outcome).unwrap(), exit)
}
fn presence_input(root: &std::path::Path, kind: &str, binding: Value) -> Value {
    let set_path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&set_path).unwrap()).unwrap();
    set["inputs"] = json!({"subject":{"kind":kind}});
    std::fs::write(set_path, serde_json::to_vec(&set).unwrap()).unwrap();
    let mut input =
        serde_json::to_value(fixture::input(root, "http://127.0.0.1:9/v1/systemone")).unwrap();
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"] =
        json!({"subject":binding});
    input
}
fn assert_presence_report<'report>(
    wire: &'report Value,
    exit: u8,
    expected: Option<&str>,
) -> &'report Value {
    assert_eq!(
        wire["kind"],
        if expected.is_some() {
            "complete_report"
        } else {
            "incomplete_report"
        },
        "{wire}"
    );
    assert_eq!(exit, if expected.is_some() { 0 } else { 3 });
    let report = &wire["report"];
    assert_eq!(report["measurements"]["model_requests"], 0);
    assert_eq!(report["measurements"]["inference_attempts"], 0);
    assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
    let occurrence = &report["occurrences"][0];
    assert_eq!(occurrence["kind"], "checked");
    assert_eq!(
        occurrence["outcome"]["kind"],
        if expected.is_some() {
            "completed"
        } else {
            "unresolved"
        }
    );
    if let Some(expected) = expected {
        assert_eq!(occurrence["outcome"]["result"], expected);
    }
    occurrence
}
fn assert_report_contracts(wire: &Value) {
    assert!(
        schema_support::reviewed("rule-invocation-report.schema.json").is_valid(&wire["report"]),
        "reviewed: {wire}"
    );
    assert!(
        schema_support::generated::<RuleInvocationReport>().is_valid(&wire["report"]),
        "generated: {wire}"
    );
    assert!(serde_json::from_value::<RuleInvocationOutcome>(wire.clone()).is_ok());
}
async fn conversation_presence(
    present: bool,
    count: usize,
    complete: bool,
    expected: Option<&str>,
) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let selector = json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"excluded","assistant_messages":"excluded","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"excluded","input_pattern":"needle"},"hook_injected":"excluded"});
    let rule = mechanical_rule(
        "local.presence",
        selector,
        json!({"kind":"expect","expect":{"present":present}}),
    );
    fixture::write_set(root, &[rule], vec![]);
    let mut bytes = String::new();
    let mut locations = Vec::new();
    for index in 0..count {
        locations.push(json!({"record_id":format!("tool:call{index}"),"position":bytes.len()}));
        bytes.push_str(&format!("{}\n",json!({"type":"response_item","payload":{"type":"custom_tool_call","call_id":format!("call{index}"),"name":"exec","input":"needle"}})));
    }
    if !complete {
        bytes.push_str("{\"type\":");
    }
    let path = root.join("rollout.jsonl");
    std::fs::write(&path, bytes).unwrap();
    let mut input = presence_input(
        root,
        "conversation",
        json!({"kind":"conversation","transcript_path":path,"format":"codex_rollout_jsonl"}),
    );
    input["repositories"] = json!([]);
    let (wire, exit) = presence_outcome(serde_json::from_value(input).unwrap()).await;
    let occurrence = assert_presence_report(&wire, exit, expected);
    assert_eq!(occurrence["target"]["kind"], "conversation_records");
    assert_eq!(occurrence["target"]["records"], json!(locations));
    assert!(occurrence["target"].get("record_ids").is_none());
    if !complete {
        assert_eq!(wire["report"]["captures"][0]["tail"]["kind"], "incomplete");
    }
    assert_report_contracts(&wire);
}

#[tokio::test]
async fn absent_check_with_selected_items_fails_for_complete_and_incomplete_capture() {
    for complete in [true, false] {
        conversation_presence(false, 2, complete, Some("fail")).await;
    }
}
#[tokio::test]
async fn absent_check_with_complete_empty_capture_passes() {
    conversation_presence(false, 0, true, Some("pass")).await;
}
#[tokio::test]
async fn absent_check_with_incomplete_empty_capture_is_unresolved() {
    conversation_presence(false, 0, false, None).await;
}
#[tokio::test]
async fn present_check_with_selected_items_passes_for_complete_and_incomplete_capture() {
    for complete in [true, false] {
        conversation_presence(true, 2, complete, Some("pass")).await;
    }
}
#[tokio::test]
async fn present_check_with_complete_empty_capture_fails() {
    conversation_presence(true, 0, true, Some("fail")).await;
}
#[tokio::test]
async fn present_check_with_incomplete_empty_capture_is_unresolved() {
    conversation_presence(true, 0, false, None).await;
}

#[tokio::test]
async fn change_presence_keeps_every_span_and_obeys_completeness_for_empty_and_nonempty_matches() {
    for present in [true, false] {
        for (limit, expected_count, expected) in [
            (1000, 2, Some(if present { "pass" } else { "fail" })),
            (2, 1, Some(if present { "pass" } else { "fail" })),
            (1, 0, None),
        ] {
            let fixture = changes::ChangeFixture::new();
            let root = fixture.root.path();
            fixture.write(
                "source.ts",
                "this.ctx.storage.put('existing', 0);\nother();\n",
            );
            fixture.stage("source.ts");
            fixture.commit();
            fixture.write("source.ts","this.ctx.storage.put('existing', 0);\nthis.ctx.storage.put('first', 1);\nthis.ctx.storage.put('second', 2);\n");
            let selector = json!({"kind":"change_set_ast_pattern","paths":[{"kind":"exact","path":"source.ts"}],"language":"typescript","pattern":"$OBJ.storage.put($$$)","sides":["added"]});
            let rule = mechanical_rule(
                "local.changes",
                selector,
                json!({"kind":"expect","expect":{"present":present}}),
            );
            fixture::write_set(root, &[rule], vec![]);
            let mut input = presence_input(
                root,
                "change_set",
                json!({"kind":"change_set","repository":"repo"}),
            );
            input["repositories"] = json!([fixture.binding(
                "repo",
                sunstack_evidence::ChangeScope::Uncommitted {
                    layers: vec![sunstack_evidence::DirtyLayer::Unstaged]
                }
            )]);
            input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
            input["discovery"]["limits"]["max_matches"] = json!(limit);
            let (wire, exit) = presence_outcome(serde_json::from_value(input).unwrap()).await;
            let occurrence = assert_presence_report(&wire, exit, expected);
            assert_eq!(occurrence["target"]["kind"], "change_matches");
            let matches = occurrence["target"]["matches"].as_array().unwrap();
            assert_eq!(matches.len(), expected_count);
            for (index, span) in matches.iter().enumerate() {
                assert_eq!(span["start_line"], index + 2);
                assert_eq!(span["source"]["path"], "source.ts");
                assert_eq!(span["source"]["revision"]["side"], "after");
                assert!(span["byte_end"].as_u64().unwrap() > span["byte_start"].as_u64().unwrap());
            }
            assert_report_contracts(&wire);
        }
    }
}

#[test]
fn selection_presence_admits_only_the_presence_check_and_refuses_empty_change_sides() {
    for (selector, kind) in [
        (
            json!({"kind":"conversation","turns":{"kind":"all"},"user_messages":"excluded","assistant_messages":"excluded","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"excluded"},"hook_injected":"excluded"}),
            "conversation",
        ),
        (
            json!({"kind":"change_set_literal_text","paths":[{"kind":"exact","path":"source.ts"}],"text":"call","sides":["added"]}),
            "change_set",
        ),
    ] {
        let inputs = serde_json::from_value(json!({"subject":{"kind":kind}})).unwrap();
        for (check, admitted) in [
            (json!({"kind":"expect","expect":{"present":true}}), true),
            (json!({"kind":"expect","expect":{"present":false}}), true),
            (json!({"kind":"expect","expect":{"matches":"call"}}), false),
            (
                json!({"kind":"expect","expect":{"not_matches":"call"}}),
                false,
            ),
            (
                json!({"kind":"expect","expect":{"starts_with":"call"}}),
                false,
            ),
            (
                json!({"kind":"ast_grep","ast_grep":{"pattern":"call()"}}),
                false,
            ),
        ] {
            let rule: RuleDefinition =
                serde_json::from_value(mechanical_rule("local.presence", selector.clone(), check))
                    .unwrap();
            let outcome = validate_rule_definition(&rule, &inputs);
            assert_eq!(outcome.is_ok(), admitted, "{rule:?}: {outcome:?}");
            if !admitted {
                assert!(
                    serde_json::to_string(&outcome.unwrap_err())
                        .unwrap()
                        .contains("invalid_target")
                );
            }
        }
        if kind == "change_set" {
            let mut rule = mechanical_rule(
                "local.presence",
                selector,
                json!({"kind":"expect","expect":{"present":false}}),
            );
            rule["evidence"][0]["select"]["sides"] = json!([]);
            assert!(
                validate_rule_definition(&serde_json::from_value(rule).unwrap(), &inputs).is_err()
            );
        }
    }
}

#[tokio::test]
async fn judged_rule_keeps_the_same_limited_change_selection_comparison_refusal() {
    let fixture = changes::ChangeFixture::new();
    let root = fixture.root.path();
    fixture.write(
        "source.ts",
        "this.ctx.storage.put('existing', 0);\nother();\n",
    );
    fixture.write("source.md", "U001: original content\n");
    fixture.stage("source.ts");
    fixture.stage("source.md");
    fixture.commit();
    fixture.write("source.ts", "this.ctx.storage.put('existing', 0);\nthis.ctx.storage.put('first', 1);\nthis.ctx.storage.put('second', 2);\n");
    let mut rule = fixture::rule("judged.peer");
    rule["evidence"].as_array_mut().unwrap().push(json!({"id":"context","input":"changes","select":{"kind":"change_set_ast_pattern","paths":[{"kind":"exact","path":"source.ts"}],"language":"typescript","pattern":"$OBJ.storage.put($$$)","sides":["added"]}}));
    fixture::write_set(root, &[rule], vec![]);
    fixture.write("source.md", "U001: changed content\n");
    let mut input = presence_input(
        root,
        "document",
        json!({"kind":"document","repository":"repo","path":"source.md"}),
    );
    let set_path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&set_path).unwrap()).unwrap();
    set["inputs"]["changes"] = json!({"kind":"change_set"});
    std::fs::write(set_path, serde_json::to_vec(&set).unwrap()).unwrap();
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["changes"] =
        json!({"kind":"change_set","repository":"repo"});
    input["repositories"] = json!([fixture.binding(
        "repo",
        sunstack_evidence::ChangeScope::Uncommitted {
            layers: vec![sunstack_evidence::DirtyLayer::Unstaged]
        }
    )]);
    input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
    input["discovery"]["limits"]["max_matches"] = json!(2);
    let (wire, exit) = presence_outcome(serde_json::from_value(input).unwrap()).await;
    assert_eq!(wire["kind"], "incomplete_report", "{wire}");
    assert_eq!(exit, 3);
    assert_eq!(
        wire["report"]["occurrences"][0]["kind"], "selection_failed",
        "{wire}"
    );
    assert!(
        wire["report"]["occurrences"][0]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "discovery_limit")
    );
    assert_eq!(wire["report"]["measurements"]["model_requests"], 0);
    assert_eq!(wire["report"]["measurements"]["inference_attempts"], 0);
    assert_report_contracts(&wire);
}
