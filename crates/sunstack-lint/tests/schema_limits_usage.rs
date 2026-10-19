#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use sunstack_decisions::{
    CancellationToken, DecisionRunOutcome, ExecutionReceipt, ExecutionUsage, ModelName,
    NamedAnswerMap,
};
use sunstack_lint::*;
use sunstack_rules::*;

#[tokio::test]
async fn every_numeric_limit_accepts_its_exact_maximum_and_rejects_the_next_integer() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    let schema = serde_json::to_value(config_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    for (field, maximum, overflow) in [
        ("max_files", u64::from(u32::MAX), "4294967296"),
        ("max_file_bytes", u64::MAX, "18446744073709551616"),
        ("max_total_bytes", u64::MAX, "18446744073709551616"),
        ("max_matches", u64::from(u32::MAX), "4294967296"),
        ("max_model_requests", u64::from(u32::MAX), "4294967296"),
        (
            "max_input_tokens_per_request",
            u64::from(u32::MAX),
            "4294967296",
        ),
        ("attempt_timeout_seconds", u64::from(u32::MAX), "4294967296"),
        ("max_concurrent_requests", u64::from(u32::MAX), "4294967296"),
    ] {
        let mut admitted = baseline.clone();
        admitted["limits"] = json!({field: maximum});
        assert!(
            validator.is_valid(&admitted),
            "schema refused {field}'s maximum"
        );
        fixture::config_file(root.path(), &admitted);
        let location = discover_config(root.path(), None).unwrap();
        let config = load_config(&location).unwrap();
        assert!(
            matches!(
                check(&config, CancellationToken::new()).await,
                CheckResult::Valid { .. }
            ),
            "native check refused {field}'s maximum"
        );

        // Preserve the overflow's authored decimal token for the native decoder.
        let raw = format!(
            "{{\"rule_sets\":{},\"limits\":{{\"{field}\":{overflow}}}}}",
            baseline["rule_sets"]
        );
        let refused: Value = serde_json::from_str(&raw).unwrap();
        assert!(
            !validator.is_valid(&refused),
            "schema accepted {field} above its range"
        );
        std::fs::write(&location.file, &raw).unwrap();
        assert!(
            load_config(&location).is_err(),
            "native decoder accepted {field} above its range"
        );
        assert_eq!(
            schema["$defs"]["LintLimits"]["properties"][field]["maximum"].as_u64(),
            Some(maximum),
            "schema rounded or omitted {field}'s maximum"
        );
    }
}

#[test]
fn all_limit_overrides_reach_their_native_settings_and_defaults_match_the_adopted_table() {
    let authored:LintLimits=serde_json::from_value(json!({"max_files":3,"max_file_bytes":17,"max_total_bytes":57,"max_matches":7,"max_model_requests":11,"max_input_tokens_per_request":123,"attempt_timeout_seconds":4,"max_concurrent_requests":2})).unwrap();
    let effective = EffectiveLimits::from(&authored);
    let native = effective.invocation_limits().unwrap();
    let budget = effective.request_budget().unwrap();
    let attempts = effective.attempts().unwrap();
    let execution = effective.execution().unwrap();
    assert_eq!(
        (
            native.max_files.get(),
            native.max_file_bytes.get(),
            native.max_total_bytes.get(),
            native.max_matches.get()
        ),
        (3, 17, 57, 7)
    );
    assert!(!native.include_ignored);
    assert_eq!((budget.max_requests, budget.max_input_tokens), (11, 123));
    assert_eq!(attempts.attempt_timeout_ms, 4000);
    assert_eq!(execution.max_concurrent_requests.get(), 2);
    let defaults = EffectiveLimits::default();
    assert_eq!(
        (
            defaults.max_files,
            defaults.max_file_bytes,
            defaults.max_total_bytes,
            defaults.max_matches
        ),
        (2000, 1_048_576, 67_108_864, 10_000)
    );
    assert_eq!(
        (
            defaults.max_model_requests,
            defaults.max_input_tokens_per_request,
            defaults.attempt_timeout_seconds,
            defaults.max_concurrent_requests
        ),
        (200, 32_000, 60, 4)
    );
    assert_eq!(
        (
            defaults.retry_count,
            defaults.initial_backoff_ms,
            defaults.max_backoff_ms
        ),
        (2, 500, 8000)
    );
}

fn usage_report(costs: &[Option<f64>]) -> RuleInvocationReport {
    RuleInvocationReport {
        rule_evaluations: vec![],
        rule_coverage: RuleCoverage::Full,
        skipped_rules: vec![],
        origin: ReportedOrigin::Direct,
        routing: RoutingReport::RuleSets,
        groups: vec![],
        occurrences: vec![],
        requests: costs
            .iter()
            .enumerate()
            .map(|(index, cost)| RequestRecord {
                index,
                owner: RequestOwner::RuleSetGroup {
                    rule_set: "set.json".into(),
                    group: index,
                    route: None,
                },
                mode: RequestMode::Rules,
                questions: vec![],
                prerequisites: vec![],
                status: RequestStatus::Sent {
                    outcome: DecisionRunOutcome::Completed {
                        answers: NamedAnswerMap::default(),
                        receipt: ExecutionReceipt {
                            resolved_model: ModelName::new("fixture").unwrap(),
                            usage: ExecutionUsage {
                                input_tokens: 10,
                                output_tokens: 2,
                                cost_usd: *cost,
                            },
                        },
                        attempts: vec![],
                    },
                },
                discarded: vec![],
            })
            .collect(),
        captures: vec![],
        change_selections: vec![],
        cursors: vec![],
        diagnostics: DiagnosticList::default(),
        measurements: InvocationMeasurements {
            model_requests: costs.len(),
            ..InvocationMeasurements::default()
        },
    }
}
#[test]
fn usage_sums_each_receipt_once_and_distinguishes_missing_cost_from_reported_zero() {
    for (costs, expected, reported, missing) in [
        (vec![], None, 0, 0),
        (vec![None], None, 0, 1),
        (vec![Some(0.0)], Some(0.0), 1, 0),
        (vec![Some(0.01), None, Some(0.02)], Some(0.03), 2, 1),
    ] {
        let summary = outcome::summarize_usage(&usage_report(&costs));
        assert_eq!(summary.model_requests, costs.len() as u32);
        assert_eq!(
            (
                summary.cost_reported_requests,
                summary.cost_missing_requests
            ),
            (reported, missing)
        );
        assert_eq!(
            summary.input_tokens,
            (!costs.is_empty()).then_some(costs.len() as u64 * 10)
        );
        assert_eq!(
            summary.output_tokens,
            (!costs.is_empty()).then_some(costs.len() as u64 * 2)
        );
        match (summary.cost_usd, expected) {
            (Some(actual), Some(expected)) => assert!((actual - expected).abs() < 1e-12),
            (None, None) => {}
            _ => panic!("missing cost became a number"),
        }
    }
}

#[tokio::test]
async fn schema_and_binary_agree_on_structure_but_loaded_reference_checks_stay_native() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    let schema = serde_json::to_value(config_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    assert!(validator.is_valid(&baseline));
    for case in [
        "unknown_root",
        "unknown_entry",
        "empty_entries",
        "empty_files",
        "empty_path",
        "invalid_selector_shape",
        "zero_limit",
        "wrong_severity",
    ] {
        let mut raw = baseline.clone();
        match case {
            "unknown_root" => raw["unknown"] = json!(true),
            "unknown_entry" => raw["rule_sets"]["docs"]["unknown"] = json!(true),
            "empty_entries" => raw["rule_sets"] = json!({}),
            "empty_files" => raw["rule_sets"]["docs"]["files"] = json!([]),
            "empty_path" => raw["rule_sets"]["docs"]["path"] = json!(""),
            "invalid_selector_shape" => raw["rules"] = json!({"docs/":"error"}),
            "zero_limit" => raw["limits"] = json!({"max_files":0}),
            "wrong_severity" => raw["rules"] = json!({"docs/*":{"kind":"error"}}),
            _ => panic!("case"),
        }
        assert!(!validator.is_valid(&raw), "schema accepted {case}");
        fixture::config_file(root.path(), &raw);
        let location = discover_config(root.path(), None).unwrap();
        assert!(load_config(&location).is_err(), "decoder accepted {case}");
    }
    for case in ["missing_file", "same_file", "unknown_rule", "bad_glob"] {
        let mut raw = baseline.clone();
        match case {
            "missing_file" => raw["rule_sets"]["docs"]["path"] = json!("absent.json"),
            "same_file" => raw["rule_sets"]["other"] = json!({"path":"./set.json","files":["**"]}),
            "unknown_rule" => raw["rules"] = json!({"docs/missing":"warn"}),
            "bad_glob" => raw["rule_sets"]["docs"]["files"] = json!(["["]),
            _ => panic!("case"),
        }
        assert!(
            validator.is_valid(&raw),
            "schema tried to check loaded semantics for {case}"
        );
        fixture::config_file(root.path(), &raw);
        let location = discover_config(root.path(), None).unwrap();
        let config = match load_config(&location) {
            Err(error) if case == "bad_glob" => {
                assert_eq!(error.code, LintInvalidCode::BadGlob);
                continue;
            }
            loaded => loaded.unwrap(),
        };
        assert!(
            matches!(
                check(&config, CancellationToken::new()).await,
                CheckResult::Invalid { .. }
            ),
            "native check accepted {case}"
        );
    }
    let canonical: Value = json!({"$schema":"https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack.lint.schema.json","rule_sets":{"skills":{"path":"rules/skill-standards.ruleset.json","files":["plugins/**/SKILL.md"]}},"rules":{"skills/*":"error","skills/skill.load-result":"warn"},"overrides":[{"files":["plugins/**/retired-skills/**"],"rules":{"skills/*":"off"}}],"ignore_patterns":["tmp/**"],"backend":{"kind":"jev_openrouter","model":"jev-latest"}});
    assert!(validator.is_valid(&canonical));
    fixture::config_file(root.path(), &canonical);
    assert!(load_config(&discover_config(root.path(), None).unwrap()).is_ok());
}
