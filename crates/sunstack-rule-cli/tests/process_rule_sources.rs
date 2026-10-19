#![allow(clippy::unwrap_used, clippy::panic)]
mod support;
use serde_json::{Value, json};
use support::*;

#[path = "support/rule_source_fixture.rs"]
mod rule_source_fixture;
use rule_source_fixture::source_fixture;
#[path = "support/signal_process.rs"]
mod signal_process;

#[tokio::test]
async fn check_returns_native_cancelled_exit_after_observed_signal_without_a_credential() {
    use signal_process::{
        collect_output, expect_line, ready_process_without_credential, send_signal,
    };
    use tokio::io::AsyncWriteExt;
    let (directory, input) = source_fixture();
    let (mut process, mut stderr, mut transcript) =
        ready_process_without_credential(directory.path()).await;
    send_signal(&process, "-TERM").await;
    expect_line(
        &mut stderr,
        "sunstack-rule: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let bytes =
        serde_json::to_vec(&json!({"protocol_version":1,"operation":"check_rules","input":input}))
            .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    stdin.write_all(&bytes).await.unwrap();
    drop(stdin);
    let output = collect_output(process, stderr, transcript).await;
    record_output(directory.path(), &bytes, &output);
    let wire = response(&output, 4);
    assert_eq!(wire["result"]["operation"], "check_rules");
    assert_eq!(wire["result"]["outcome"]["kind"], "cancelled");
    assert_eq!(
        wire["result"]["outcome"]["measurements"]["model_requests"],
        0
    );
}

fn write_entry(root: &std::path::Path, entry: Value) {
    std::fs::write(
        root.join("teaching.rules.json"),
        serde_json::to_vec(&json!({"K014":entry})).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn mixed_jsonl_and_markdown_sources_print_together_but_jsonl_errors_stay_strict() {
    let (directory, input) = source_fixture();
    let root = directory.path();
    let mut header: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    header["rule_paths"] = json!(["legacy.jsonl"]);
    std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
    let legacy = json!({"rule_id":"legacy.rule","evidence":[{"id":"target","input":"skill","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"Legacy question"},"threshold":0.8}});
    std::fs::write(root.join("legacy.jsonl"), format!("{}\n", legacy)).unwrap();
    let checked = response(
        &run_without_credential(root, "check_rules", input.clone()).await,
        0,
    );
    let printed = checked["result"]["outcome"]["printed_rules"]
        .as_array()
        .unwrap();
    assert_eq!(printed.len(), 2);
    assert_eq!(printed[0]["definition"]["rule_id"], "legacy.rule");
    assert_eq!(printed[0]["source"]["kind"], "jsonl");
    assert_eq!(printed[0]["source"]["line"], 1);
    assert_eq!(printed[1]["source"]["kind"], "markdown");
    std::fs::write(root.join("legacy.jsonl"), "{\n").unwrap();
    let invalid = response(
        &run(
            root,
            "evaluate_rules",
            rule_source_fixture::runtime_input(input, "http://127.0.0.1:1/v1/systemone"),
        )
        .await,
        2,
    );
    assert_eq!(invalid["result"]["outcome"]["kind"], "invalid");
    assert_eq!(
        invalid["result"]["outcome"]["diagnostics"]["items"][0]["code"],
        "malformed_json"
    );
}

#[tokio::test]
async fn source_context_and_error_band_lower_through_native_admission() {
    let (directory, input) = source_fixture();
    write_entry(
        directory.path(),
        json!({"name":"standard.visible","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"context":[{"id":"background","input":"skill","select":{"kind":"whole_document"}}],"threshold":0.8,"severity":"error","error_at_or_below":0.2}),
    );
    let wire = response(
        &run_without_credential(directory.path(), "check_rules", input).await,
        0,
    );
    let definition = &wire["result"]["outcome"]["printed_rules"][0]["definition"];
    assert_eq!(definition["severity"], "error");
    assert_eq!(definition["verdict"]["error_at_or_below"], 0.2);
    assert_eq!(definition["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(definition["evidence"][1]["id"], "background");
    assert_eq!(
        definition["evidence"][1]["select"]["kind"],
        "whole_document"
    );
}

#[tokio::test]
async fn source_qualified_markers_allow_distinct_names_and_refuse_colliding_names() {
    let (directory, input) = source_fixture();
    let root = directory.path();
    std::fs::write(root.join("other.md"), "Another standard<!-- K014 -->\n").unwrap();
    let mut header: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    header["rule_sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"markdown":"other.md","rules":"other.rules.json"}));
    std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
    let mut entry = json!({"name":"other.standard","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8});
    for duplicate in [false, true] {
        if duplicate {
            entry["name"] = json!("standard.visible");
        }
        std::fs::write(
            root.join("other.rules.json"),
            serde_json::to_vec(&json!({"K014":entry})).unwrap(),
        )
        .unwrap();
        let wire = response(
            &run_without_credential(root, "check_rules", input.clone()).await,
            if duplicate { 2 } else { 0 },
        );
        if duplicate {
            assert_eq!(wire["result"]["outcome"]["refused_rule_count"], 2);
        } else {
            assert_eq!(wire["result"]["outcome"]["checked_rule_count"], 2);
            let printed = wire["result"]["outcome"]["printed_rules"]
                .as_array()
                .unwrap();
            assert_ne!(
                printed[0]["definition"]["rule_id"],
                printed[1]["definition"]["rule_id"]
            );
            assert_eq!(
                printed[0]["source"]["provenance"]["marker_id"],
                printed[1]["source"]["provenance"]["marker_id"]
            );
            assert_ne!(
                printed[0]["source"]["provenance"]["markdown"],
                printed[1]["source"]["provenance"]["markdown"]
            );
        }
    }
}

#[tokio::test]
async fn source_word_cap_uses_default_four_hundred_and_explicit_override() {
    for cap in [None, Some(2)] {
        let (directory, input) = source_fixture();
        let root = directory.path();
        if let Some(cap) = cap {
            let mut header: Value =
                serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
            header["max_question_words"] = json!(cap);
            std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
        } else {
            std::fs::write(
                root.join("teaching.md"),
                format!("{}<!-- K014 -->\n", "word ".repeat(401)),
            )
            .unwrap();
        }
        let wire = response(&run_without_credential(root, "check_rules", input).await, 2);
        assert_eq!(wire["result"]["outcome"]["refused_rule_count"], 1);
        assert_eq!(
            wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
            "question_too_large"
        );
        let diagnostic = &wire["result"]["outcome"]["diagnostics"]["items"][0];
        assert_eq!(diagnostic["location"]["field_path"], json!(["K014"]));
        assert_eq!(
            diagnostic["guidance"],
            "Mark a narrower host, such as a subsection, a paragraph or a list item."
        );
    }
}

#[tokio::test]
async fn check_prints_exact_lowered_rule_without_credential_or_reviewed_document_read() {
    let (directory, input) = source_fixture();
    let output = run_without_credential(directory.path(), "check_rules", input.clone()).await;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["result"]["operation"], "check_rules");
    let checked = &wire["result"]["outcome"];
    assert_eq!(checked["kind"], "checked");
    assert_eq!(checked["checked_rule_count"], 1);
    let printed = &checked["printed_rules"][0];
    assert_eq!(printed["definition"]["rule_id"], "standard.visible");
    assert_eq!(
        printed["definition"]["verdict"]["question"]["instructions"],
        "Standard\n\nKeep visible words and code."
    );
    assert_eq!(printed["definition"]["verdict"]["threshold"], 0.8);
    assert_eq!(printed["definition"]["severity"], "warn");
    assert_eq!(printed["definition"]["target"], "target");
    assert_eq!(printed["definition"]["evidence"][0]["input"], "skill");
    assert_eq!(printed["source"]["kind"], "markdown");
    assert_eq!(printed["source"]["provenance"]["marker_id"], "K014");
    assert!(
        printed["source"]["provenance"]["marker_span"]
            .get("source")
            .is_none()
    );
    let repeated = run_without_credential(directory.path(), "check_rules", input).await;
    assert_eq!(output.stdout, repeated.stdout);
    assert!(!directory.path().join("missing-reviewed.md").exists());
}

#[tokio::test]
async fn check_collects_both_marker_and_entry_refusals_without_printing_partial_rules() {
    let (directory, input) = source_fixture();
    std::fs::write(
        directory.path().join("teaching.md"),
        "first<!-- K014 -->\n\nsecond<!-- K015 -->\n",
    )
    .unwrap();
    let rules = json!({"K014":{"name":"first","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0},
        "K015":{"name":"second","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U002"}},"threshold":0.8,"error_at_or_below":0.9}});
    std::fs::write(
        directory.path().join("teaching.rules.json"),
        serde_json::to_vec_pretty(&rules).unwrap(),
    )
    .unwrap();
    let output = run_without_credential(directory.path(), "check_rules", input).await;
    let wire = response(&output, 2);
    let outcome = &wire["result"]["outcome"];
    assert_eq!(outcome["kind"], "refused");
    assert_eq!(outcome["refused_rule_count"], 2);
    assert!(outcome.get("printed_rules").is_none());
    let diagnostics = outcome["diagnostics"]["items"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    for diagnostic in diagnostics {
        assert_eq!(
            diagnostic["location"]["file"],
            json!(
                directory
                    .path()
                    .canonicalize()
                    .unwrap()
                    .join("teaching.rules.json")
            )
        );
        assert!(diagnostic["location"]["line"].as_u64().unwrap() > 1);
        assert_eq!(diagnostic["location"]["column"], 11);
    }
}

#[tokio::test]
async fn strict_source_check_refuses_each_invalid_entry_without_losing_its_location() {
    let baseline = json!({"name":"standard.visible","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8});
    for variant in [
        "missing_input",
        "unknown_field",
        "bad_threshold",
        "bad_band",
        "reserved_context",
        "undeclared_context",
        "bad_regex",
        "no_verdict",
        "two_verdicts",
    ] {
        let (directory, input) = source_fixture();
        let mut entry = baseline.clone();
        match variant {
            "missing_input" => {
                entry["target"].as_object_mut().unwrap().remove("input");
            }
            "unknown_field" => entry["AUTHOR_SECRET_KEY"] = json!("PAYLOAD_SECRET_SENTINEL"),
            "bad_threshold" => entry["threshold"] = json!(1.1),
            "bad_band" => entry["error_at_or_below"] = json!(0.2),
            "reserved_context" => {
                entry["context"] = json!([{"id":"target","input":"skill","select":{"kind":"paragraph","target_id":"U001"}}])
            }
            "undeclared_context" => {
                entry["context"] = json!([{"id":"extra","input":"undeclared","select":{"kind":"paragraph","target_id":"U001"}}])
            }
            "bad_regex" => {
                entry["context"] = json!([{"id":"extra","input":"skill","select":{"kind":"regex_text","pattern":"("}}])
            }
            "no_verdict" => {
                entry.as_object_mut().unwrap().remove("threshold");
            }
            "two_verdicts" => entry["expect"] = json!({"present":true}),
            _ => panic!("case"),
        }
        std::fs::write(
            directory.path().join("teaching.rules.json"),
            serde_json::to_vec(&json!({"K014":entry})).unwrap(),
        )
        .unwrap();
        let output = run_without_credential(directory.path(), "check_rules", input).await;
        let wire = response(&output, 2);
        let outcome = &wire["result"]["outcome"];
        assert_eq!(outcome["kind"], "refused", "{variant}: {wire}");
        assert_eq!(outcome["refused_rule_count"], 1, "{variant}: {wire}");
        assert!(
            !outcome["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            outcome["diagnostics"]["items"][0]["location"]["file"],
            json!(
                directory
                    .path()
                    .canonicalize()
                    .unwrap()
                    .join("teaching.rules.json")
            )
        );
    }
}

#[tokio::test]
async fn duplicate_json_keys_are_preserved_until_their_rules_are_refused() {
    for nested in [false, true] {
        let (directory, input) = source_fixture();
        let entry = r#"{"name":"standard.visible","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8}"#;
        let rules = if nested {
            format!(
                "{{\"K014\":{}}}",
                entry.replace("\"threshold\":0.8", "\"threshold\":0.8,\"threshold\":0.9")
            )
        } else {
            format!("{{\"K014\":{entry},\"K014\":{entry}}}")
        };
        std::fs::write(directory.path().join("teaching.rules.json"), rules).unwrap();
        let output = run_without_credential(directory.path(), "check_rules", input).await;
        let wire = response(&output, 2);
        assert_eq!(wire["result"]["outcome"]["kind"], "refused");
        assert!(
            wire["result"]["outcome"]["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "duplicate_field")
        );
    }
}

#[tokio::test]
async fn consistency_checks_markers_and_entries_in_both_directions() {
    for (text, code, guidance) in [
        (
            "ordinary without marker\n",
            "orphaned_rule_entry",
            "Add the corresponding marker or remove the orphaned rules entry.",
        ),
        (
            "# Forbidden<!-- K014 -->\n",
            "forbidden_marker_host",
            "Move the marker to a heading of level 2–6, a paragraph or a list item, outside block quotes, tables and HTML.",
        ),
        (
            "> forbidden<!-- K014 -->\n",
            "forbidden_marker_host",
            "Move the marker to a heading of level 2–6, a paragraph or a list item, outside block quotes, tables and HTML.",
        ),
        (
            "## Spaced <!-- K014 -->\n",
            "spaced_heading_marker",
            "Remove the space before the heading marker.",
        ),
        (
            "one<!-- K014 -->\n\ntwo<!-- K014 -->\n",
            "duplicate_marker",
            "Use each marker ID at most once in a Markdown source.",
        ),
        (
            "four<!-- K0014 -->\n",
            "orphaned_rule_entry",
            "Add the corresponding marker or remove the orphaned rules entry.",
        ),
    ] {
        let (directory, input) = source_fixture();
        std::fs::write(directory.path().join("teaching.md"), text).unwrap();
        let wire = response(
            &run_without_credential(directory.path(), "check_rules", input).await,
            2,
        );
        assert_eq!(wire["result"]["outcome"]["kind"], "refused", "{text}");
        assert_eq!(wire["result"]["outcome"]["refused_rule_count"], 1);
        for diagnostic in wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
        {
            assert_eq!(diagnostic["code"], code);
            assert_eq!(diagnostic["location"]["field_path"], json!(["K014"]));
            assert_eq!(diagnostic["guidance"], guidance);
        }
    }
    let (directory, input) = source_fixture();
    std::fs::write(directory.path().join("teaching.rules.json"), "{}").unwrap();
    let wire = response(
        &run_without_credential(directory.path(), "check_rules", input).await,
        2,
    );
    assert_eq!(
        wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
        "missing_rule_entry"
    );
    let diagnostic = &wire["result"]["outcome"]["diagnostics"]["items"][0];
    assert_eq!(diagnostic["location"]["field_path"], json!(["K014"]));
    assert_eq!(
        diagnostic["guidance"],
        "Add a rules entry for this marker ID."
    );
}

#[tokio::test]
async fn check_rejects_backend_fields_and_malformed_sources_at_the_native_boundary() {
    let (directory, mut input) = source_fixture();
    input["backend"] = json!({"kind":"jev_openrouter"});
    let wire = response(
        &run_without_credential(directory.path(), "check_rules", input).await,
        2,
    );
    assert_eq!(
        wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
        "unknown_field"
    );
    let (directory, input) = source_fixture();
    std::fs::write(directory.path().join("teaching.rules.json"), "{").unwrap();
    let wire = response(
        &run_without_credential(directory.path(), "check_rules", input).await,
        2,
    );
    assert_eq!(wire["result"]["outcome"]["refused_rule_count"], 1);
    assert_eq!(
        wire["result"]["outcome"]["diagnostics"]["items"][0]["location"]["file"],
        json!(
            directory
                .path()
                .canonicalize()
                .unwrap()
                .join("teaching.rules.json")
        )
    );
}
