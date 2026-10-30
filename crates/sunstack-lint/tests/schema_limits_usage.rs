#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::{Value, json};
use sunstack_decisions::{
    CancellationToken, DecisionRunOutcome, ExecutionReceipt, ExecutionUsage, ModelName,
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
        let baseline_text = serde_json::to_string(&baseline).unwrap();
        let raw = format!(
            "{},\"limits\":{{\"{field}\":{overflow}}}}}",
            baseline_text.strip_suffix('}').unwrap()
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

fn usage_report(costs: &[Option<f64>]) -> RulebookRun {
    RulebookRun {
        results: vec![],
        requests: costs
            .iter()
            .enumerate()
            .map(|(index, cost)| RulebookRequestRecord {
                pair_index: index,
                request: serde_json::from_value(
                    json!({"questions":{"q":{"type":"noul","instructions":"Evaluate"}},"state":{}}),
                )
                .unwrap(),
                outcome: DecisionRunOutcome::Completed {
                    answers: serde_json::from_value(
                        json!({"q":{"type":"noul","yes_probability":1.0}}),
                    )
                    .unwrap(),
                    receipt: ExecutionReceipt {
                        resolved_model: ModelName::new("fixture").unwrap(),
                        usage: ExecutionUsage {
                            input_tokens: 10,
                            output_tokens: 2,
                            cost_usd: *cost,
                        },
                    },
                    attempts: vec![sunstack_decisions::InferenceAttemptRecord {
                        ordinal: 1,
                        evidence: sunstack_decisions::AttemptEvidence::Completed {
                            http_status: 200,
                        },
                    }],
                },
            })
            .collect(),
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
        let summary =
            sunstack_lint::lint_reporting::outcome::summarize_usage(&usage_report(&costs));
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
async fn schema_and_native_decoder_agree_on_structure_but_loaded_checks_stay_native() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    let schema = serde_json::to_value(config_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    assert!(validator.is_valid(&baseline));
    for (pointer, value) in [
        ("/rulebook", json!([])),
        ("/check/files", json!([])),
        ("/limits", json!({"max_files":0})),
        ("/decision_model/model", json!(null)),
    ] {
        let mut value_to_check = baseline.clone();
        if pointer == "/limits" {
            value_to_check["limits"] = value;
        } else {
            *value_to_check.pointer_mut(pointer).unwrap() = value;
        }
        assert!(!validator.is_valid(&value_to_check), "{pointer}");
        fixture::config_file(root.path(), &value_to_check);
        assert!(load_config(&discover_config(root.path(), None).unwrap()).is_err());
    }
    for key in [
        "rule_sets",
        "rules",
        "overrides",
        "ignore_patterns",
        "backend",
        "unknown",
    ] {
        let mut raw = baseline.clone();
        raw[key] = json!({});
        assert!(!validator.is_valid(&raw));
        fixture::config_file(root.path(), &raw);
        assert!(load_config(&discover_config(root.path(), None).unwrap()).is_err());
    }
    let mut missing = baseline;
    missing["evidence"]["ast_grep"] = json!("missing.yml");
    assert!(validator.is_valid(&missing));
    fixture::config_file(root.path(), &missing);
    let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
    assert!(matches!(
        check(&loaded, CancellationToken::new()).await,
        CheckResult::Invalid { .. }
    ));
}

#[tokio::test]
async fn report_schema_validates_real_pair_results_and_requires_their_typed_fields() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let report = run(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
    )
    .await;
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["results"].as_array().unwrap().len(), 1);
    assert_eq!(value["results"][0]["outcome"], "finding");
    assert_eq!(value["results"][0]["step"], "starts");
    let schema = serde_json::to_value(report_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    assert!(
        validator.is_valid(&value),
        "{:?}",
        validator.iter_errors(&value).collect::<Vec<_>>()
    );
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("results");
    assert!(!validator.is_valid(&missing));
    let mut wrong = value;
    wrong["results"][0]["outcome"] = json!("unknown");
    assert!(!validator.is_valid(&wrong));
    let mut without_results = report.clone();
    without_results.results.clear();
    for format in [
        OutputFormat::Default,
        OutputFormat::Github,
        OutputFormat::Sarif,
    ] {
        assert_eq!(render(&report, format), render(&without_results, format));
    }
}

async fn assert_kind_refusal_location(scenario: &str) {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["child", "docs/*.md"]);
    config["evidence"]["kinds"]["child"] = json!({"rule":"skill","fields":{}});
    match scenario {
        "undefined_kind" => {
            config["evidence"]["kinds"]["child"]["fields"]["body"] =
                json!({"from":"inside","kind":"missing"})
        }
        "cycle" => {
            config["evidence"]["kinds"]["child"]["within"] = json!(["skill", "z"]);
            config["evidence"]["kinds"]["z"] =
                json!({"rule":"skill","fields":{},"within":["child"]});
        }
        "undefined_within_entry" => {
            config["evidence"]["kinds"]["child"]["within"] = json!(["skill", "missing"])
        }
        _ => panic!(),
    }
    fixture::config_file(root.path(), &config);
    let path = root.path().join("sunstack.lint.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let position = text.find("\"child\": {").unwrap();
    let prefix = &text[..position];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
    let column = u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
    let loaded = load_config(&ConfigLocation {
        file: path.clone(),
        directory: root.path().to_owned(),
    })
    .unwrap();
    let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
    else {
        panic!()
    };

    let diagnostic = diagnostics
        .iter()
        .find(|row| {
            row.message.starts_with("undefined kind ")
                || row.message.starts_with("kind dependency cycle")
        })
        .unwrap();
    assert_eq!(diagnostic.file, Some(path));
    assert_eq!(
        (diagnostic.line, diagnostic.column),
        (Some(line), Some(column)),
        "{scenario}: {diagnostics:?}"
    );
    assert!(line > 1);
}

#[tokio::test]
async fn kind_plan_refusals_locate_undefined_kind() {
    assert_kind_refusal_location("undefined_kind").await;
}

#[tokio::test]
async fn kind_plan_refusals_locate_cycle() {
    assert_kind_refusal_location("cycle").await;
}

#[tokio::test]
async fn kind_plan_refusals_locate_undefined_within_entry() {
    assert_kind_refusal_location("undefined_within_entry").await;
}

#[tokio::test]
async fn cycle_location_and_path_are_stable_across_declared_kind_orders() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["evidence"]["kinds"] = json!("KINDS_PLACEHOLDER");
    let envelope = serde_json::to_string(&config).unwrap();
    let child = r#""child":{"rule":"skill","fields":{},"within":["skill","z"]}"#;
    let skill =
        r#""skill":{"rule":"skill","fields":{"description":{"from":"yaml","key":"description"}}}"#;
    let z = r#""z":{"rule":"skill","fields":{},"within":["child"]}"#;
    let mut observed = Vec::new();
    for kinds in [
        format!("{{{child},{skill},{z}}}"),
        format!("{{{child},{z},{skill}}}"),
    ] {
        let text = envelope.replace("\"KINDS_PLACEHOLDER\"", &kinds);
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let loaded = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!()
        };
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("child -> z -> child"));
        let row = serde_json::to_value(&diagnostics[0]).unwrap();
        assert_eq!(
            diagnostics[0].column,
            Some(u32::try_from(text.find("\"child\":").unwrap() + 1).unwrap())
        );
        observed.push(row);
    }
    assert_eq!(observed[0], observed[1]);
}
