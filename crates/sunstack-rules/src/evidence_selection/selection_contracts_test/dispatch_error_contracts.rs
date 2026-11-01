#![allow(clippy::unwrap_used)]
use crate::*;
use serde_json::json;
use sunstack_decisions::*;
use sunstack_evidence::FieldName;

#[test]
fn selection_error_display_does_not_change_serialized_reason_tags_or_payloads() {
    for (reason, expected) in [
        (
            SelectionIncompleteReason::EvidenceUnavailable {
                field: FieldName::new("body").unwrap(),
            },
            json!({"kind":"evidence_unavailable","field":"body"}),
        ),
        (
            SelectionIncompleteReason::EvidenceTooLarge,
            json!({"kind":"evidence_too_large"}),
        ),
        (
            SelectionIncompleteReason::RequestTooLarge {
                limit: RequestLimit::RequestMaxChars,
            },
            json!({"kind":"request_too_large","limit":"request_max_chars"}),
        ),
        (
            SelectionIncompleteReason::InvalidRequest,
            json!({"kind":"invalid_request"}),
        ),
        (
            SelectionIncompleteReason::ProviderFailure {
                failure_class: ProviderFailureClass::RateLimit,
                http_status: Some(429),
            },
            json!({"kind":"provider_failure","failure_class":"rate_limit","http_status":429}),
        ),
        (
            SelectionIncompleteReason::InvalidResponse,
            json!({"kind":"invalid_response"}),
        ),
        (
            SelectionIncompleteReason::RequestLimitReached,
            json!({"kind":"request_limit_reached"}),
        ),
        (
            SelectionIncompleteReason::Cancelled,
            json!({"kind":"cancelled"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(reason).unwrap(), expected);
    }
}
