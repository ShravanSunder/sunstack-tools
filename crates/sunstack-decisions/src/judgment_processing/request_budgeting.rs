use crate::{decision_protocol::decision_contracts::*, decision_protocol::decision_diagnostics::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU64, sync::Arc};

/// Opaque, instance-specific backend identity. Clone only for the same backend.
#[derive(Debug, Clone)]
pub struct BackendIdentity(Arc<()>);
impl Default for BackendIdentity {
    fn default() -> Self {
        Self::new()
    }
}
impl BackendIdentity {
    pub fn new() -> Self {
        Self(Arc::new(()))
    }
    pub fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone)]
pub struct EncodedBackendRequest {
    backend: BackendIdentity,
    body: Arc<[u8]>,
    question_count: usize,
}
impl EncodedBackendRequest {
    /// Backend implementations create the exact body once, before accounting.
    pub fn new(backend: BackendIdentity, body: Vec<u8>, question_count: usize) -> Self {
        Self {
            backend,
            body: body.into(),
            question_count,
        }
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn backend_matches(&self, identity: &BackendIdentity) -> bool {
        self.backend.matches(identity)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TokenAccounting {
    Exact {
        input_tokens: u64,
        provider_ceiling: ProviderContextCeiling,
    },
    Estimated {
        input_tokens: u64,
        provider_ceiling: ProviderContextCeiling,
    },
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderContextCeiling {
    CallerDeclared { tokens: NonZeroU64 },
    NotDeclared,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AccountingBasis {
    Exact {
        provider_ceiling: ProviderContextCeiling,
    },
    Estimated {
        provider_ceiling: ProviderContextCeiling,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDimension {
    RequestChars,
    RequestBytes,
    QuestionCount,
    InputTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "basis", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdmissionMeasurement {
    Exact {
        observed: u64,
    },
    Estimated {
        observed: u64,
    },
    Overflow {
        input_tokens: u64,
        safety_margin_tokens: u64,
        accounting: AccountingBasis,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdmissionDiagnosticDetail {
    Limit {
        dimension: AdmissionDimension,
        measurement: AdmissionMeasurement,
        cap: u64,
    },
    Accounting {
        accounting: TokenAccounting,
        policy: AccountingPolicy,
        cap: u64,
    },
}
impl AdmissionDiagnosticDetail {
    pub fn summary(&self) -> String {
        match self {
            Self::Limit {
                dimension,
                measurement,
                cap,
            } => {
                let dimension = match dimension {
                    AdmissionDimension::RequestChars => "request characters",
                    AdmissionDimension::RequestBytes => "request bytes",
                    AdmissionDimension::QuestionCount => "question count",
                    AdmissionDimension::InputTokens => "input tokens",
                };
                let measured = match measurement {
                    AdmissionMeasurement::Exact { observed } => format!("{observed} (exact)"),
                    AdmissionMeasurement::Estimated { observed } => {
                        format!("{observed} (estimated)")
                    }
                    AdmissionMeasurement::Overflow {
                        input_tokens,
                        safety_margin_tokens,
                        ..
                    } => format!(
                        "overflow adding {input_tokens} and safety margin {safety_margin_tokens}"
                    ),
                };
                format!("{dimension}: {measured}; cap {cap}.")
            }
            Self::Accounting {
                accounting,
                policy,
                cap,
            } => {
                let basis = match accounting {
                    TokenAccounting::Exact { .. } => "exact",
                    TokenAccounting::Estimated { .. } => "estimated",
                    TokenAccounting::Unavailable => "unavailable",
                };
                format!("Input-token accounting {basis}; policy {policy:?}; cap {cap}.")
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestBudgetCounts {
    pub input_tokens: u64,
    pub request_bytes: usize,
    pub questions: usize,
    pub basis: AccountingBasis,
}
#[derive(Clone)]
pub struct AdmittedBackendRequest {
    encoded: EncodedBackendRequest,
    counts: RequestBudgetCounts,
}
impl AdmittedBackendRequest {
    pub fn body(&self) -> &[u8] {
        self.encoded.body()
    }
    pub fn counts(&self) -> &RequestBudgetCounts {
        &self.counts
    }
    pub fn belongs_to(&self, backend: &BackendIdentity) -> bool {
        self.encoded.backend.matches(backend)
    }
}
pub enum RequestBudgetAssessment {
    WithinConfiguredBudget {
        admitted_request: AdmittedBackendRequest,
    },
    Exceeded {
        diagnostics: DiagnosticList,
    },
    Unverifiable {
        diagnostics: DiagnosticList,
    },
}
pub fn assess_request_budget(
    encoded: EncodedBackendRequest,
    settings: &RequestBudgetSettings,
    accounting: TokenAccounting,
) -> RequestBudgetAssessment {
    let exceeded = |dimension, measurement, cap| RequestBudgetAssessment::Exceeded {
        diagnostics: DecisionDiagnostic::request(DecisionDiagnosticCode::ContextBudgetExceeded)
            .with_admission(AdmissionDiagnosticDetail::Limit {
                dimension,
                measurement,
                cap,
            })
            .into(),
    };
    let unverifiable = |code| RequestBudgetAssessment::Unverifiable {
        diagnostics: DecisionDiagnostic::request(code).into(),
    };
    if settings.max_input_tokens == 0
        || settings.max_request_bytes == 0
        || settings.max_questions == 0
        || settings.max_requests == 0
    {
        return unverifiable(DecisionDiagnosticCode::InvalidConfiguration);
    }
    if encoded.body.len() > settings.max_request_bytes {
        return exceeded(
            AdmissionDimension::RequestBytes,
            AdmissionMeasurement::Exact {
                observed: u64::try_from(encoded.body.len()).unwrap_or(u64::MAX),
            },
            u64::try_from(settings.max_request_bytes).unwrap_or(u64::MAX),
        );
    }
    if encoded.question_count > settings.max_questions {
        return exceeded(
            AdmissionDimension::QuestionCount,
            AdmissionMeasurement::Exact {
                observed: u64::try_from(encoded.question_count).unwrap_or(u64::MAX),
            },
            u64::try_from(settings.max_questions).unwrap_or(u64::MAX),
        );
    }
    let (tokens, provider_ceiling, basis) = match accounting {
        TokenAccounting::Exact {
            input_tokens,
            provider_ceiling,
        } => (
            input_tokens,
            provider_ceiling,
            AccountingBasis::Exact { provider_ceiling },
        ),
        TokenAccounting::Estimated {
            input_tokens,
            provider_ceiling,
        } if settings.accounting_policy == AccountingPolicy::AllowEstimated => (
            input_tokens,
            provider_ceiling,
            AccountingBasis::Estimated { provider_ceiling },
        ),
        accounting @ (TokenAccounting::Estimated { .. } | TokenAccounting::Unavailable) => {
            return RequestBudgetAssessment::Unverifiable {
                diagnostics: DecisionDiagnostic::request(
                    DecisionDiagnosticCode::BudgetUnverifiable,
                )
                .with_admission(AdmissionDiagnosticDetail::Accounting {
                    accounting,
                    policy: settings.accounting_policy.clone(),
                    cap: settings.max_input_tokens,
                })
                .into(),
            };
        }
    };
    let input_limit = match provider_ceiling {
        ProviderContextCeiling::CallerDeclared { tokens } => {
            settings.max_input_tokens.min(tokens.get())
        }
        ProviderContextCeiling::NotDeclared => settings.max_input_tokens,
    };
    let Some(total_tokens) = tokens.checked_add(settings.safety_margin_tokens) else {
        return exceeded(
            AdmissionDimension::InputTokens,
            AdmissionMeasurement::Overflow {
                input_tokens: tokens,
                safety_margin_tokens: settings.safety_margin_tokens,
                accounting: basis,
            },
            input_limit,
        );
    };
    if total_tokens > input_limit {
        let measurement = match basis {
            AccountingBasis::Exact { .. } => AdmissionMeasurement::Exact {
                observed: total_tokens,
            },
            AccountingBasis::Estimated { .. } => AdmissionMeasurement::Estimated {
                observed: total_tokens,
            },
        };
        return exceeded(AdmissionDimension::InputTokens, measurement, input_limit);
    }
    let counts = RequestBudgetCounts {
        input_tokens: total_tokens,
        request_bytes: encoded.body.len(),
        questions: encoded.question_count,
        basis,
    };
    RequestBudgetAssessment::WithinConfiguredBudget {
        admitted_request: AdmittedBackendRequest { encoded, counts },
    }
}

#[cfg(test)]
#[path = "request_budgeting_test.rs"]
mod tests;
