#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "request_budgeting_test/request_budget.rs"]
mod request_budget;

#[test]
fn rejected_budget_diagnostics_keep_dimension_measurement_cap_and_accounting() {
    use super::*;
    let baseline = RequestBudgetSettings {
        max_input_tokens: 100,
        max_request_bytes: 100,
        max_questions: 10,
        max_requests: 10,
        safety_margin_tokens: 0,
        accounting_policy: AccountingPolicy::AllowEstimated,
    };
    for (dimension, observed, cap) in [
        ("request_bytes", 3, 2),
        ("question_count", 2, 1),
        ("input_tokens", 11, 10),
    ] {
        let mut settings = baseline.clone();
        match dimension {
            "request_bytes" => settings.max_request_bytes = cap as usize,
            "question_count" => settings.max_questions = cap as usize,
            _ => settings.max_input_tokens = cap,
        }
        let encoded = EncodedBackendRequest::new(BackendIdentity::new(), b"abc".to_vec(), 2);
        let outcome = assess_request_budget(
            encoded,
            &settings,
            TokenAccounting::Estimated {
                input_tokens: 11,
                provider_ceiling: ProviderContextCeiling::NotDeclared,
            },
        );
        let RequestBudgetAssessment::Exceeded { diagnostics } = outcome else {
            panic!("expected refusal")
        };
        let value = serde_json::to_value(diagnostics).unwrap();
        assert_eq!(value["items"][0]["code"], "context_budget_exceeded");
        assert_eq!(
            value["items"][0]["admission"],
            serde_json::json!({
                "kind":"limit","dimension":dimension,"measurement":{"basis":if dimension == "input_tokens" {"estimated"} else {"exact"},"observed":observed},"cap":cap
            })
        );
    }
    let outcome = assess_request_budget(
        EncodedBackendRequest::new(BackendIdentity::new(), b"abc".to_vec(), 2),
        &baseline,
        TokenAccounting::Unavailable,
    );
    let RequestBudgetAssessment::Unverifiable { diagnostics } = outcome else {
        panic!("expected unavailable accounting")
    };
    let value = serde_json::to_value(diagnostics).unwrap();
    assert_eq!(value["items"][0]["admission"]["kind"], "accounting");
    assert_eq!(
        value["items"][0]["admission"]["accounting"]["kind"],
        "unavailable"
    );
    assert!(value["items"][0]["admission"].get("observed").is_none());
}
