use crate::{
    backend_transport::decision_backends::*, decision_protocol::decision_contracts::*,
    decision_protocol::decision_diagnostics::*, judgment_processing::request_budgeting::*,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// One run, one attempt controller, no spawned local work. Cancellation drops
/// the in-flight IO future before returning; remote effects stay unknown.
pub async fn run_judgments(
    input: &RunJudgmentsInput,
    backend: &dyn DecisionBackend,
    cancellation: &CancellationToken,
) -> DecisionRunOutcome {
    let invalid = |diagnostic: DecisionDiagnostic| DecisionRunOutcome::InvalidRequest {
        diagnostics: diagnostic.into(),
    };
    if let Err(diagnostic) =
        crate::judgment_processing::judgment_validation::validate_request(&input.request)
            .and_then(|()| input.attempts.validate())
    {
        return invalid(diagnostic);
    }
    if input.backend != *backend.selection() {
        return invalid(DecisionDiagnostic::request(
            DecisionDiagnosticCode::InvalidConfiguration,
        ));
    }
    if cancellation.is_cancelled() {
        return cancelled_before_dispatch(Vec::new(), 1);
    }
    let encoded = match backend.encode_request(&input.request) {
        Ok(encoded) => encoded,
        Err(diagnostic) => return invalid(diagnostic),
    };
    let accounting = backend.account_request(&encoded);
    let admitted = match assess_request_budget(encoded, &input.request_budget, accounting) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => admitted_request,
        RequestBudgetAssessment::Exceeded { diagnostics }
        | RequestBudgetAssessment::Unverifiable { diagnostics } => {
            return DecisionRunOutcome::InvalidRequest { diagnostics };
        }
    };
    run_admitted_judgments(
        &input.request,
        backend,
        &admitted,
        &input.attempts,
        cancellation,
    )
    .await
}

pub async fn run_admitted_judgments(
    request: &StructuredJudgmentRequest,
    backend: &dyn DecisionBackend,
    admitted: &AdmittedBackendRequest,
    settings: &AttemptSettings,
    cancellation: &CancellationToken,
) -> DecisionRunOutcome {
    if let Err(diagnostic) =
        crate::judgment_processing::judgment_validation::validate_request(request)
            .and_then(|()| settings.validate())
    {
        return DecisionRunOutcome::InvalidRequest {
            diagnostics: diagnostic.into(),
        };
    }
    if !admitted.belongs_to(backend.identity()) {
        return DecisionRunOutcome::InvalidRequest {
            diagnostics: DecisionDiagnostic::request(DecisionDiagnosticCode::InvalidConfiguration)
                .into(),
        };
    }
    // The public admitted route must bind response admission to the same request.
    let encoded = match backend.encode_request(request) {
        Ok(encoded) => encoded,
        Err(diagnostic) => {
            return DecisionRunOutcome::InvalidRequest {
                diagnostics: diagnostic.into(),
            };
        }
    };
    if encoded.body() != admitted.body() {
        return DecisionRunOutcome::InvalidRequest {
            diagnostics: DecisionDiagnostic::request(DecisionDiagnosticCode::InvalidConfiguration)
                .into(),
        };
    }
    let mut attempts = Vec::new();
    let mut backoff_ms = settings.initial_backoff_ms;
    for ordinal in 1..=settings.retry_count + 1 {
        if cancellation.is_cancelled() {
            return cancelled_before_dispatch(attempts, ordinal);
        }
        let result = {
            let attempt = backend.dispatch(admitted, settings.max_response_bytes);
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => None,
                result = tokio::time::timeout(Duration::from_millis(settings.attempt_timeout_ms), attempt) => Some(result),
            }
        }; // The attempt future has settled or been dropped at this point.
        let failure = match result {
            None => {
                attempts.push(InferenceAttemptRecord {
                    ordinal,
                    evidence: AttemptEvidence::RemoteOutcomeUnknown {
                        failure_class: AttemptFailureClass::CallerAborted,
                        http_status: None,
                    },
                });
                return cancelled(attempts);
            }
            Some(Err(_)) => BackendAttemptFailure::unknown(ProviderFailureClass::Timeout, None),
            Some(Ok(Err(failure))) => failure,
            Some(Ok(Ok(response))) => {
                let admitted_response = backend.admit_response(request, &response.body);
                if cancellation.is_cancelled() {
                    attempts.push(InferenceAttemptRecord {
                        ordinal,
                        evidence: AttemptEvidence::RemoteOutcomeUnknown {
                            failure_class: AttemptFailureClass::CallerAborted,
                            http_status: Some(response.http_status),
                        },
                    });
                    return cancelled(attempts);
                }
                match admitted_response {
                    Ok((answers, receipt)) => {
                        attempts.push(InferenceAttemptRecord {
                            ordinal,
                            evidence: AttemptEvidence::Completed {
                                http_status: response.http_status,
                            },
                        });
                        return DecisionRunOutcome::Completed {
                            answers,
                            receipt,
                            attempts,
                        };
                    }
                    Err(diagnostic) => {
                        attempts.push(InferenceAttemptRecord {
                            ordinal,
                            evidence: AttemptEvidence::RemoteOutcomeUnknown {
                                failure_class: AttemptFailureClass::InvalidResponse,
                                http_status: Some(response.http_status),
                            },
                        });
                        return DecisionRunOutcome::InvalidResponse {
                            diagnostics: diagnostic.into(),
                            attempts,
                        };
                    }
                }
            }
        };
        let class = failure.failure_class;
        let status = match &failure.evidence {
            AttemptEvidence::RemoteOutcomeUnknown { http_status, .. } => *http_status,
            AttemptEvidence::NotDispatched { .. } | AttemptEvidence::Completed { .. } => None,
        };
        attempts.push(InferenceAttemptRecord {
            ordinal,
            evidence: failure.evidence,
        });
        if cancellation.is_cancelled() {
            return cancelled(attempts);
        }
        if class == AttemptFailureClass::InvalidResponse {
            return DecisionRunOutcome::InvalidResponse {
                diagnostics: DecisionDiagnostic::response(DecisionDiagnosticCode::InvalidBody)
                    .into(),
                attempts,
            };
        }
        if class == AttemptFailureClass::CallerAborted {
            return cancelled(attempts);
        }
        let Some(class) = class.provider_class() else {
            return DecisionRunOutcome::InvalidResponse {
                diagnostics: DecisionDiagnostic::response(DecisionDiagnosticCode::InvalidBody)
                    .into(),
                attempts,
            };
        };
        let transient = matches!(
            class,
            ProviderFailureClass::RateLimit
                | ProviderFailureClass::ProviderUnavailable
                | ProviderFailureClass::Timeout
                | ProviderFailureClass::Connection
        );
        if ordinal > settings.retry_count || !transient {
            return DecisionRunOutcome::ProviderFailure {
                failure_class: class,
                http_status: status,
                diagnostics: provider_diagnostic(class).into(),
                attempts,
            };
        }
        tokio::select! { biased; _ = cancellation.cancelled() => return cancelled_before_dispatch(attempts, ordinal + 1), _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {} }
        backoff_ms = backoff_ms.saturating_mul(2).min(settings.max_backoff_ms);
    }
    DecisionRunOutcome::ProviderFailure {
        failure_class: ProviderFailureClass::UnexpectedProviderError,
        http_status: None,
        diagnostics: provider_diagnostic(ProviderFailureClass::UnexpectedProviderError).into(),
        attempts,
    }
}
fn cancelled(attempts: Vec<InferenceAttemptRecord>) -> DecisionRunOutcome {
    DecisionRunOutcome::Cancelled {
        diagnostics: DecisionDiagnostic::new(
            DecisionDiagnosticOrigin::Execution,
            DecisionDiagnosticCode::CallerAborted,
            RetryAdvice::DoNotRetry,
        )
        .into(),
        attempts,
    }
}

fn cancelled_before_dispatch(
    mut attempts: Vec<InferenceAttemptRecord>,
    ordinal: u32,
) -> DecisionRunOutcome {
    attempts.push(InferenceAttemptRecord {
        ordinal,
        evidence: AttemptEvidence::NotDispatched {
            failure_class: AttemptFailureClass::CallerAborted,
        },
    });
    cancelled(attempts)
}

#[cfg(test)]
#[path = "judgment_execution_test.rs"]
mod tests;
