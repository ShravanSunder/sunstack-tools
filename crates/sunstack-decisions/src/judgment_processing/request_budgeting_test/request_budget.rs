#![allow(clippy::unwrap_used, clippy::panic)]
use crate::test_support as support;
use crate::*;
use support::*;

fn declared_ceiling(tokens: u64) -> ProviderContextCeiling {
    ProviderContextCeiling::CallerDeclared {
        tokens: tokens.try_into().unwrap(),
    }
}

fn encoded() -> EncodedBackendRequest {
    EncodedBackendRequest::new(
        BackendIdentity::new(),
        b"{\"state\":\"\\\"\\n\"}".to_vec(),
        3,
    )
}
#[test]
fn exact_estimated_unavailable_and_provider_ceiling_admission() {
    let mut settings = budget();
    settings.max_input_tokens = 100;
    settings.safety_margin_tokens = 7;
    let accounting = TokenAccounting::Exact {
        input_tokens: 93,
        provider_ceiling: declared_ceiling(100),
    };
    assert!(matches!(
        assess_request_budget(encoded(), &settings, accounting),
        RequestBudgetAssessment::WithinConfiguredBudget { .. }
    ));
    for accounting in [
        TokenAccounting::Exact {
            input_tokens: 94,
            provider_ceiling: declared_ceiling(100),
        },
        TokenAccounting::Exact {
            input_tokens: 93,
            provider_ceiling: declared_ceiling(99),
        },
        TokenAccounting::Exact {
            input_tokens: u64::MAX,
            provider_ceiling: declared_ceiling(u64::MAX),
        },
    ] {
        assert!(matches!(
            assess_request_budget(encoded(), &settings, accounting),
            RequestBudgetAssessment::Exceeded { .. }
        ));
    }
    settings.accounting_policy = AccountingPolicy::RequireExact;
    for accounting in [
        TokenAccounting::Estimated {
            input_tokens: 1,
            provider_ceiling: declared_ceiling(100),
        },
        TokenAccounting::Unavailable,
        TokenAccounting::Estimated {
            input_tokens: 1,
            provider_ceiling: ProviderContextCeiling::NotDeclared,
        },
    ] {
        assert!(matches!(
            assess_request_budget(encoded(), &settings, accounting),
            RequestBudgetAssessment::Unverifiable { .. }
        ));
    }
    settings.accounting_policy = AccountingPolicy::AllowEstimated;
    match assess_request_budget(
        encoded(),
        &settings,
        TokenAccounting::Estimated {
            input_tokens: 12,
            provider_ceiling: declared_ceiling(100),
        },
    ) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => {
            assert_eq!(admitted_request.counts().input_tokens, 19);
            assert_eq!(
                admitted_request.counts().basis,
                AccountingBasis::Estimated {
                    provider_ceiling: declared_ceiling(100)
                }
            );
        }
        _ => panic!("explicit estimated allowance refused"),
    }
}
