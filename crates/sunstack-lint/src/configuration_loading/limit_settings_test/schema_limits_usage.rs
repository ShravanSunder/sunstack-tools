#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

#[test]
fn all_limit_overrides_reach_their_native_settings_and_defaults_match_the_adopted_table() {
    let authored:LintLimits=serde_json::from_value(json!({"max_files":3,"max_file_bytes":17,"max_total_bytes":57,"max_matches":7,"max_model_requests":11,"max_input_tokens_per_request":123,"attempt_timeout_seconds":4,"max_concurrent_requests":2,"max_locate_lines":12})).unwrap();
    let effective = EffectiveLimits::from(&authored);
    assert_eq!(effective.max_locate_lines, 12);
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
    assert_eq!(defaults.max_locate_lines, 80);
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
