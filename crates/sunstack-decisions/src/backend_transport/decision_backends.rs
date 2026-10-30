use crate::{
    decision_protocol::decision_contracts::*, decision_protocol::decision_diagnostics::*,
    judgment_processing::request_budgeting::*,
};
use serde::Serialize;
use std::{future::Future, pin::Pin};

pub type BackendAttemptFuture<'request> =
    Pin<Box<dyn Future<Output = Result<BackendResponse, BackendAttemptFailure>> + Send + 'request>>;
pub struct BackendResponse {
    pub http_status: u16,
    pub body: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Decision transport failed: {failure_class:?}.")]
pub struct BackendAttemptFailure {
    pub failure_class: AttemptFailureClass,
    pub evidence: AttemptEvidence,
}
impl BackendAttemptFailure {
    pub fn unknown(
        failure_class: impl Into<AttemptFailureClass>,
        http_status: Option<u16>,
    ) -> Self {
        let failure_class = failure_class.into();
        Self {
            failure_class,
            evidence: AttemptEvidence::RemoteOutcomeUnknown {
                failure_class,
                http_status,
            },
        }
    }
    pub fn not_dispatched(failure_class: impl Into<AttemptFailureClass>) -> Self {
        let failure_class = failure_class.into();
        Self {
            failure_class,
            evidence: AttemptEvidence::NotDispatched { failure_class },
        }
    }
}
/// A backend owns its attempt future. Dropping it must settle all local work;
/// implementations may not detach workers. The shared controller is the only
/// retry/timeout owner. Credentials never enter a request or diagnostic.
pub trait DecisionBackend: Send + Sync {
    fn identity(&self) -> &BackendIdentity;
    fn selection(&self) -> &BackendSelection;
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic>;
    fn account_request(&self, encoded: &EncodedBackendRequest) -> TokenAccounting;
    fn dispatch<'request>(
        &'request self,
        request: &'request AdmittedBackendRequest,
        max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request>;
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic>;
}

pub enum BackendPreparation {
    Ready { backend: JevOpenRouterBackend },
    Refused { diagnostics: DiagnosticList },
}
// Observation points are compiled only into the crate's own unit-test binary.
// They pause real IO after reading, without substituting the HTTP adapter.
#[cfg(test)]
#[derive(Clone)]
enum AdapterTestObservation {
    BodyReceived {
        received: std::sync::Arc<tokio::sync::Notify>,
        resume: std::sync::Arc<tokio::sync::Notify>,
    },
    HttpFailure {
        observed: std::sync::Arc<tokio::sync::Notify>,
    },
}

pub struct JevOpenRouterBackend {
    identity: BackendIdentity,
    selection: BackendSelection,
    endpoint: reqwest::Url,
    authorization: reqwest::header::HeaderValue,
    client: reqwest::Client,
    #[cfg(test)]
    test_observation: Option<AdapterTestObservation>,
}
impl JevOpenRouterBackend {
    pub fn prepare(selection: BackendSelection, api_key: &str) -> BackendPreparation {
        match Self::new(selection, api_key) {
            Ok(backend) => BackendPreparation::Ready { backend },
            Err(diagnostic) => BackendPreparation::Refused {
                diagnostics: diagnostic.into(),
            },
        }
    }
    pub fn new(selection: BackendSelection, api_key: &str) -> Result<Self, DecisionDiagnostic> {
        let invalid = || DecisionDiagnostic::request(DecisionDiagnosticCode::InvalidConfiguration);
        let BackendSelection::JevOpenrouter { endpoint, .. } = &selection;
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| invalid())?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || api_key.is_empty()
        {
            return Err(invalid());
        }
        let mut authorization =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|_| invalid())?;
        authorization.set_sensitive(true);
        let client = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| invalid())?;
        Ok(Self {
            identity: BackendIdentity::new(),
            selection,
            endpoint,
            authorization,
            client,
            #[cfg(test)]
            test_observation: None,
        })
    }
}
impl DecisionBackend for JevOpenRouterBackend {
    fn identity(&self) -> &BackendIdentity {
        &self.identity
    }
    fn selection(&self) -> &BackendSelection {
        &self.selection
    }
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic> {
        let BackendSelection::JevOpenrouter { model, .. } = &self.selection;
        let body = encode_judgment_body(request, model)?;
        Ok(EncodedBackendRequest::new(
            self.identity.clone(),
            body,
            request.questions.len(),
        ))
    }
    fn account_request(&self, encoded: &EncodedBackendRequest) -> TokenAccounting {
        if !encoded.backend_matches(&self.identity) {
            return TokenAccounting::Unavailable;
        }
        let BackendSelection::JevOpenrouter {
            context_ceiling_tokens,
            ..
        } = &self.selection;
        let provider_ceiling = match context_ceiling_tokens {
            Some(tokens) => ProviderContextCeiling::CallerDeclared { tokens: *tokens },
            None => ProviderContextCeiling::NotDeclared,
        };
        // System One publishes no tokenizer or model ceiling. The byte upper
        // estimate stays labeled estimated; the caller declares the ceiling.
        match u64::try_from(encoded.body().len()) {
            Ok(input_tokens) => TokenAccounting::Estimated {
                input_tokens,
                provider_ceiling,
            },
            Err(_) => TokenAccounting::Unavailable,
        }
    }
    fn dispatch<'request>(
        &'request self,
        request: &'request AdmittedBackendRequest,
        max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        Box::pin(async move {
            if !request.belongs_to(&self.identity) {
                return Err(BackendAttemptFailure::not_dispatched(
                    ProviderFailureClass::ProviderRejectedRequest,
                ));
            }
            let mut response = self
                .client
                .post(self.endpoint.clone())
                .header(reqwest::header::AUTHORIZATION, self.authorization.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(request.body().to_vec())
                .send()
                .await
                .map_err(|error| {
                    let class = if error.is_timeout() {
                        ProviderFailureClass::Timeout
                    } else if error.is_connect() {
                        ProviderFailureClass::Connection
                    } else {
                        ProviderFailureClass::UnexpectedProviderError
                    };
                    // Even a connection failure may have crossed an intermediary.
                    BackendAttemptFailure::unknown(class, None)
                })?;
            let status = response.status().as_u16();
            if !(200..300).contains(&status) {
                #[cfg(test)]
                if let Some(AdapterTestObservation::HttpFailure { observed }) =
                    &self.test_observation
                {
                    observed.notify_one();
                }
                return Err(BackendAttemptFailure::unknown(
                    classify_http_status(status),
                    Some(status),
                ));
            }
            if response
                .content_length()
                .is_some_and(|length| length > max_response_bytes as u64)
            {
                return Err(BackendAttemptFailure::unknown(
                    AttemptFailureClass::InvalidResponse,
                    Some(status),
                ));
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| {
                BackendAttemptFailure::unknown(ProviderFailureClass::Connection, Some(status))
            })? {
                if chunk.len() > max_response_bytes.saturating_sub(body.len()) {
                    return Err(BackendAttemptFailure::unknown(
                        AttemptFailureClass::InvalidResponse,
                        Some(status),
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            #[cfg(test)]
            if let Some(AdapterTestObservation::BodyReceived { received, resume }) =
                &self.test_observation
            {
                received.notify_one();
                resume.notified().await;
            }
            Ok(BackendResponse {
                http_status: status,
                body,
            })
        })
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        crate::judgment_processing::judgment_validation::decode_jev_response(request, body)
    }
}

/// Encode exactly the body the Jev backend dispatches, before constructing a backend.
pub fn encode_judgment_body(
    request: &StructuredJudgmentRequest,
    model: &ModelName,
) -> Result<Vec<u8>, DecisionDiagnostic> {
    crate::judgment_processing::judgment_validation::validate_request(request)?;
    #[derive(Serialize)]
    struct JevRequest<'a> {
        model: &'a ModelName,
        state: &'a SharedEvidenceState,
        questions: &'a NamedQuestionMap,
    }
    serde_json::to_vec(&JevRequest {
        model,
        state: &request.state,
        questions: &request.questions,
    })
    .map_err(|_| DecisionDiagnostic::request(DecisionDiagnosticCode::InvalidQuestion))
}

#[cfg(test)]
mod adapter_cancellation_tests;
