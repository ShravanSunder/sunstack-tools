#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::{Value, json};
use sunstack_decisions::CancellationToken;
use sunstack_lint::test_support::lint_fixture as fixture;
use sunstack_lint::*;
use sunstack_rules::test_support::controlled_http as controlled;

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
        ("max_locate_lines", u64::from(u32::MAX), "4294967296"),
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
    assert_eq!(value["results"][0]["path"], json!(["starts"]));
    assert_eq!(value["results"][0]["basis"], "mechanical");
    assert!(value["results"][0].get("answers").is_none());
    assert_eq!(value["findings"][0]["basis"], "mechanical");
    assert_eq!(value["findings"][0]["anchor"], "default");
    let schema = serde_json::to_value(report_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    assert!(
        validator.is_valid(&value),
        "{:?}",
        validator.iter_errors(&value).collect::<Vec<_>>()
    );
    let mut missing = value.clone();
    missing["results"][0]
        .as_object_mut()
        .unwrap()
        .remove("path");
    assert!(!validator.is_valid(&missing));
    let mut missing = value.clone();
    missing["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("basis");
    assert!(!validator.is_valid(&missing));
    let mut missing = value.clone();
    missing["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("related");
    assert!(!validator.is_valid(&missing));
    let mut missing = value.clone();
    missing["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("anchor");
    assert!(!validator.is_valid(&missing));
    let mut bad_anchor = value.clone();
    bad_anchor["findings"][0]["anchor"] = json!("unknown");
    assert!(!validator.is_valid(&bad_anchor));
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

#[tokio::test]
async fn report_schema_validates_real_provider_payloads_and_default_format_names_the_class() {
    for connection in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = controlled::ControlledServer::start().await;
        let endpoint = if connection {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}/v1/systemone", listener.local_addr().unwrap())
        } else {
            server.endpoint.clone()
        };
        fixture::judged(root.path(), &endpoint);
        let operation = tokio::spawn(run(
            fixture::request(root.path()),
            CancellationToken::new(),
            Some("SYNTHETIC_LINT_CREDENTIAL".into()),
        ));
        if !connection {
            for _ in 0..3 {
                server
                    .next()
                    .await
                    .respond(429, json!({"error":"controlled rate limit"}));
            }
        }
        let report = tokio::time::timeout(std::time::Duration::from_secs(5), operation)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.finish().await, if connection { 0 } else { 3 });
        assert_eq!(report.status, LintStatus::Incomplete);
        let value = serde_json::to_value(&report).unwrap();
        let reason = if connection {
            json!({"kind":"provider_failure","failure_class":"connection"})
        } else {
            json!({"kind":"provider_failure","failure_class":"rate_limit","http_status":429})
        };
        assert_eq!(value["incomplete"][0]["reason"], reason);
        let schema = report_schema();
        let validator = jsonschema::validator_for(schema.as_value()).unwrap();
        assert!(validator.is_valid(&value), "{value}");
        let mut wrong = value.clone();
        wrong["incomplete"][0]["reason"]
            .as_object_mut()
            .unwrap()
            .remove("failure_class");
        assert!(!validator.is_valid(&wrong));
        let text = render(&report, OutputFormat::Default);
        assert!(
            text.contains(if connection {
                "provider failure (connection)"
            } else {
                "provider failure (rate_limit, HTTP 429)"
            }),
            "{text}"
        );
        let sarif = render(&report, OutputFormat::Sarif);
        assert!(
            sarif.contains(if connection {
                "provider failure (connection)"
            } else {
                "provider failure (rate_limit, HTTP 429)"
            }),
            "{sarif}"
        );
    }
}
