use crate::{decision_contracts::*, decision_diagnostics::*};
use std::sync::Arc;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenAccounting {
    Exact {
        input_tokens: u64,
        context_ceiling_tokens: u64,
    },
    Estimated {
        input_tokens: u64,
        context_ceiling_tokens: u64,
    },
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountingBasis {
    Exact,
    Estimated,
}
#[derive(Debug, Clone, PartialEq, Eq)]
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
    let exceeded = || RequestBudgetAssessment::Exceeded {
        diagnostics: DecisionDiagnostic::request(DecisionDiagnosticCode::ContextBudgetExceeded)
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
    if encoded.body.len() > settings.max_request_bytes
        || encoded.question_count > settings.max_questions
    {
        return exceeded();
    }
    let (tokens, ceiling, basis) = match accounting {
        TokenAccounting::Exact {
            input_tokens,
            context_ceiling_tokens,
        } => (input_tokens, context_ceiling_tokens, AccountingBasis::Exact),
        TokenAccounting::Estimated {
            input_tokens,
            context_ceiling_tokens,
        } if settings.accounting_policy == AccountingPolicy::AllowEstimated => (
            input_tokens,
            context_ceiling_tokens,
            AccountingBasis::Estimated,
        ),
        TokenAccounting::Estimated { .. } | TokenAccounting::Unavailable => {
            return unverifiable(DecisionDiagnosticCode::BudgetUnverifiable);
        }
    };
    if ceiling == 0 {
        return unverifiable(DecisionDiagnosticCode::BudgetUnverifiable);
    }
    let Some(total_tokens) = tokens.checked_add(settings.safety_margin_tokens) else {
        return exceeded();
    };
    if total_tokens > settings.max_input_tokens.min(ceiling) {
        return exceeded();
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
