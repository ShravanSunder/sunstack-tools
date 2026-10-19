#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/contract_schemas.rs"]
mod schema_support;
use schema_support::{generated, reviewed};
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_rules::*;

fn header() -> Value {
    json!({"schema_version":1,"name":"standards","description":"schema controls","inputs":{"skill":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"rule_sources":[{"markdown":"standards.md","rules":"standards.rules.json"}],"marker_prefix":"K"})
}
fn entry() -> Value {
    json!({"name":"standard.one","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8})
}
fn check_input(root: &std::path::Path) -> Value {
    json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"skill":{"kind":"document","repository":"repo","path":"missing.md"}}}]}]},"repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}],"payload_schemas":[],"limits":{"max_files":10,"max_file_bytes":65536,"max_total_bytes":131072,"max_matches":20,"max_transcript_bytes":65536,"include_ignored":false},"output":"assembled_rules"})
}

#[test]
fn reviewed_and_generated_headers_admit_both_source_styles_and_require_a_source_prefix() {
    let reviewed = reviewed("rule-set.schema.json");
    let generated = generated::<RuleSetDefinition>();
    let mut cases = vec![(header(), true)];
    let mut legacy = header();
    legacy["rule_paths"] = json!(["legacy.jsonl"]);
    legacy.as_object_mut().unwrap().remove("rule_sources");
    legacy.as_object_mut().unwrap().remove("marker_prefix");
    cases.push((legacy, true));
    let mut mixed = header();
    mixed["rule_paths"] = json!(["legacy.jsonl"]);
    cases.push((mixed, true));
    let mut empty_paths = header();
    empty_paths["rule_paths"] = json!([]);
    cases.push((empty_paths, true));
    for prefix in [None, Some(json!(null)), Some(json!("k"))] {
        let mut invalid = header();
        if let Some(value) = prefix {
            invalid["marker_prefix"] = value;
        } else {
            invalid.as_object_mut().unwrap().remove("marker_prefix");
        }
        cases.push((invalid, false));
    }
    let mut empty = header();
    empty["rule_sources"] = json!([]);
    cases.push((empty, false));
    let mut bad_path = header();
    bad_path["rule_sources"][0]["markdown"] = json!("");
    cases.push((bad_path, false));
    for (value, accepted) in cases {
        assert_eq!(reviewed.is_valid(&value), accepted, "reviewed {value}");
        assert_eq!(generated.is_valid(&value), accepted, "generated {value}");
        assert_eq!(
            decode_rule_set(&serde_json::to_vec(&value).unwrap(), "set.json".as_ref()).is_ok(),
            accepted,
            "native {value}"
        );
    }
}

#[tokio::test]
async fn source_entry_schemas_preserve_shape_and_native_cross_field_refusals() {
    let reviewed = reviewed("rule-source.schema.json");
    let generated = generated::<RuleSourceEntry>();
    let mut cases = vec![(entry(), true, true)];
    for field in ["name", "threshold"] {
        let mut value = entry();
        value.as_object_mut().unwrap().remove(field);
        cases.push((value, false, false));
    }
    let mut input = entry();
    input["target"].as_object_mut().unwrap().remove("input");
    cases.push((input, false, false));
    for (field, value) in [
        ("threshold", json!(0)),
        ("threshold", json!(1.1)),
        ("name", json!("Invalid_Name")),
        ("AUTHOR_SECRET_KEY", json!(true)),
        ("expect", json!({"present":true})),
        ("block_at_or_below", json!(-0.1)),
    ] {
        let mut invalid = entry();
        invalid[field] = value;
        cases.push((invalid, false, false));
    }
    let mut band = entry();
    band["severity"] = json!("error");
    band["block_at_or_below"] = json!(0.2);
    cases.push((band, true, true));
    let mut reserved = entry();
    reserved["context"] =
        json!([{"id":"target","input":"skill","select":{"kind":"paragraph","target_id":"U001"}}]);
    cases.push((reserved, true, false));
    let mut nudge_band = entry();
    nudge_band["block_at_or_below"] = json!(0.2);
    cases.push((nudge_band, true, false));
    for (value, shape, accepted) in cases {
        assert_eq!(generated.is_valid(&value), shape, "generated {value}");
        assert_eq!(
            reviewed.is_valid(&json!({"K014":value})),
            shape,
            "reviewed {value}"
        );
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::write(
            root.join("set.json"),
            serde_json::to_vec(&header()).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("standards.md"), "A standard<!-- K014 -->\n").unwrap();
        std::fs::write(
            root.join("standards.rules.json"),
            serde_json::to_vec(&json!({"K014":value})).unwrap(),
        )
        .unwrap();
        let input = decode_check_rules(&serde_json::to_vec(&check_input(root)).unwrap()).unwrap();
        let outcome = check_rules(input, CancellationToken::new()).await;
        assert_eq!(
            matches!(outcome, CheckRulesOutcome::Checked { .. }),
            accepted,
            "native {value}: {outcome:?}"
        );
    }
    assert!(!reviewed.is_valid(&json!({"K0014":entry()})));
}

#[tokio::test]
async fn check_schema_refuses_backend_fields_and_empty_root_at_full_admission() {
    let reviewed = reviewed("check-rules-input.schema.json");
    let generated = generated::<CheckRulesInput>();
    let root = tempfile::tempdir().unwrap();
    let baseline = check_input(root.path());
    assert!(reviewed.is_valid(&baseline));
    assert!(generated.is_valid(&baseline));
    let mut empty = baseline.clone();
    empty["definition_root"] = json!("");
    assert!(!reviewed.is_valid(&empty));
    assert!(!generated.is_valid(&empty));
    let outcome = check_rules(
        decode_check_rules(&serde_json::to_vec(&empty).unwrap()).unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(matches!(outcome, CheckRulesOutcome::Refused { .. }));
    for field in ["backend", "attempts", "request_budget"] {
        let mut extra = baseline.clone();
        extra[field] = json!({});
        assert!(!reviewed.is_valid(&extra));
        assert!(!generated.is_valid(&extra));
        assert!(decode_check_rules(&serde_json::to_vec(&extra).unwrap()).is_err());
    }
}

#[tokio::test]
async fn markdown_selector_schemas_and_check_keep_shape_and_semantic_refusals_distinct() {
    let reviewed_declaration = reviewed("evidence-declaration.schema.json");
    let generated_declaration = generated::<sunstack_evidence::EvidenceDeclaration>();
    for (selection, shape, admitted) in [
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph","list_item"],"pattern":"Use"}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"starts_with":"Completion:"}),
            true,
            true,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"]}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":[]}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"Use","starts_with":"Use"}),
            false,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"["}),
            true,
            false,
        ),
        (
            json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"\\b"}),
            true,
            false,
        ),
        (
            json!({"kind":"frontmatter_field","field":"name"}),
            true,
            false,
        ),
    ] {
        let declaration = json!({"id":"target","input":"skill","select":selection});
        assert_eq!(
            reviewed_declaration.is_valid(&declaration),
            shape,
            "reviewed {declaration}"
        );
        assert_eq!(
            generated_declaration.is_valid(&declaration),
            shape,
            "generated {declaration}"
        );
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("set.json"),
            serde_json::to_vec(&header()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("standards.md"),
            "A standard<!-- K014 -->\n",
        )
        .unwrap();
        let mut authored = entry();
        authored["target"]["select"] = selection;
        std::fs::write(
            root.path().join("standards.rules.json"),
            serde_json::to_vec(&json!({"K014":authored})).unwrap(),
        )
        .unwrap();
        let input =
            decode_check_rules(&serde_json::to_vec(&check_input(root.path())).unwrap()).unwrap();
        let outcome = check_rules(input, CancellationToken::new()).await;
        assert_eq!(
            matches!(outcome, CheckRulesOutcome::Checked { .. }),
            admitted,
            "{declaration}: {outcome:?}"
        );
        assert!(!root.path().join("missing.md").exists());
    }
}

#[test]
fn versioned_reports_require_coverage_and_reject_unknown_skip_shapes() {
    let baseline = json!({"origin":{"kind":"direct"},"routing":{"kind":"rule_sets"},"groups":[],"occurrences":[],"rule_evaluations":[],"requests":[],"captures":[],"change_selections":[],"cursors":[],"diagnostics":{"items":[],"omitted_count":0},"measurements":InvocationMeasurements::default(),"rule_coverage":"full","skipped_rules":[]});
    let reviewed = reviewed("rule-invocation-report.schema.json");
    let generated = generated::<RuleInvocationReport>();
    for coverage in ["full", "partial", "none"] {
        let mut value = baseline.clone();
        value["rule_coverage"] = json!(coverage);
        assert!(reviewed.is_valid(&value));
        assert!(generated.is_valid(&value));
        assert!(serde_json::from_value::<RuleInvocationReport>(value).is_ok());
    }
    let mut missing = baseline.clone();
    missing.as_object_mut().unwrap().remove("rule_coverage");
    let mut unknown = baseline.clone();
    unknown["skipped_rules"] = json!([{"kind":"invented"}]);
    let mut missing_evaluations = baseline.clone();
    missing_evaluations
        .as_object_mut()
        .unwrap()
        .remove("rule_evaluations");
    let mut unknown_evaluation = baseline.clone();
    unknown_evaluation["rule_evaluations"] = json!([{"kind":"waiting_for_expansion","key":{"rule_set":"set.json","group":0,"rule_id":"one"},"on":[]}]);
    let mut old_counts = baseline;
    old_counts["measurements"]
        .as_object_mut()
        .unwrap()
        .remove("invalid_rule_entries");
    for value in [
        missing,
        unknown,
        old_counts,
        missing_evaluations,
        unknown_evaluation,
    ] {
        assert!(!reviewed.is_valid(&value));
        assert!(!generated.is_valid(&value));
        assert!(serde_json::from_value::<RuleInvocationReport>(value).is_err());
    }
}
