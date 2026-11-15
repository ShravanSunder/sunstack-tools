use crate::{RequestLimit, RulebookRunSettings};
use std::sync::Arc;
use sunstack_decisions::*;
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Debug, Clone, thiserror::Error)]
pub enum DispatchRefusal {
    #[error("model capability is unavailable")]
    Unavailable(crate::WorkUnavailableReason),
    #[error(transparent)]
    ChoiceOptionsExceeded(#[from] ChoiceOptionsExceeded),
    #[error("request exceeds the configured size limit")]
    RequestTooLarge,
    #[error("model request allowance is exhausted")]
    RequestLimitReached,
    #[error("model request was cancelled")]
    Cancelled,
    #[error("model request is invalid")]
    InvalidRequest,
}
#[derive(Debug, Clone)]
pub struct DispatchedRequest {
    pub id: usize,
    pub request: StructuredJudgmentRequest,
    pub outcome: DecisionRunOutcome,
}
#[derive(Debug, Clone)]
pub struct PreparedModelRequest {
    pub id: usize,
    pub request: StructuredJudgmentRequest,
}
#[derive(Debug, Clone)]
pub struct SelectionRequestOwner {
    pub request_id: usize,
    pub selection_kind: sunstack_evidence::EvidenceKindName,
    pub file: std::path::PathBuf,
    pub candidates: Vec<sunstack_evidence::SelectionCandidate>,
    pub shared: sunstack_evidence::SharedView,
}
#[derive(Debug, Clone)]
pub struct RecordedSelection {
    pub source_request_id: usize,
    pub source_revision: Option<String>,
    pub selection_kind: sunstack_evidence::EvidenceKindName,
    pub request: StructuredJudgmentRequest,
    pub input_sha256: String,
    pub outcome: DecisionRunOutcome,
}
#[derive(Debug, Clone)]
pub struct ReusedSelection {
    pub request_id: usize,
    pub source_request_id: usize,
    pub source_revision: Option<String>,
    pub outcome: DecisionRunOutcome,
}
pub fn selection_input_hash(
    request: &StructuredJudgmentRequest,
) -> Result<String, serde_json::Error> {
    serde_json::to_vec(request).map(|bytes| sunstack_evidence::content_digest(&bytes))
}
#[derive(Clone)]
pub enum ModelDispatchPolicy {
    LocalOnly,
    Preparation { encoder: Arc<dyn DecisionBackend> },
    Execution { backend: Arc<dyn DecisionBackend> },
    CredentialUnavailable,
}
/// One request allowance, concurrency owner and receipt list for selection and rule work.
pub struct ModelRequestDispatch {
    pub settings: RulebookRunSettings,
    policy: ModelDispatchPolicy,
    cancellation: CancellationToken,
    semaphore: Arc<Semaphore>,
    tasks: JoinSet<DispatchedRequest>,
    requested: usize,
    next_request_id: usize,
    pub records: Vec<DispatchedRequest>,
    pub prepared: Vec<PreparedModelRequest>,
    pub selection_requests: Vec<SelectionRequestOwner>,
    recorded_selections: Vec<RecordedSelection>,
    pub reused_selections: Vec<ReusedSelection>,
}
impl ModelRequestDispatch {
    pub fn new(
        settings: RulebookRunSettings,
        policy: ModelDispatchPolicy,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(settings.max_concurrent_requests.max(1))),
            settings,
            policy,
            cancellation,
            tasks: JoinSet::new(),
            requested: 0,
            next_request_id: 0,
            records: vec![],
            prepared: vec![],
            selection_requests: vec![],
            recorded_selections: vec![],
            reused_selections: vec![],
        }
    }
    pub fn is_preparing(&self) -> bool {
        matches!(self.policy, ModelDispatchPolicy::Preparation { .. })
    }
    fn backend(&self) -> Option<&Arc<dyn DecisionBackend>> {
        match &self.policy {
            ModelDispatchPolicy::Preparation { encoder } => Some(encoder),
            ModelDispatchPolicy::Execution { backend } => Some(backend),
            ModelDispatchPolicy::LocalOnly | ModelDispatchPolicy::CredentialUnavailable => None,
        }
    }
    /// Read the run's capability before building or reserving reached model work.
    pub fn check_capability(&self) -> Result<(), DispatchRefusal> {
        if self.is_cancelled() {
            return Err(DispatchRefusal::Cancelled);
        }
        match self.policy {
            ModelDispatchPolicy::CredentialUnavailable => Err(DispatchRefusal::Unavailable(
                crate::WorkUnavailableReason::CredentialUnavailable {},
            )),
            ModelDispatchPolicy::LocalOnly => Err(DispatchRefusal::InvalidRequest),
            ModelDispatchPolicy::Preparation { .. } | ModelDispatchPolicy::Execution { .. } => {
                Ok(())
            }
        }
    }
    pub fn with_recorded_selections(mut self, records: Vec<RecordedSelection>) -> Self {
        self.recorded_selections = records;
        self
    }
    pub fn reuse_selection(
        &mut self,
        kind: &sunstack_evidence::EvidenceKindName,
        request: &StructuredJudgmentRequest,
    ) -> Option<(usize, DecisionRunOutcome)> {
        if !self.is_preparing()
            || self.is_cancelled()
            || self.requested >= self.settings.max_model_requests
        {
            return None;
        }
        self.assess_selection(request).ok()?;
        let hash = selection_input_hash(request).ok()?;
        let BackendSelection::JevOpenrouter { model, .. } = &self.settings.backend;
        let mut matches = self.recorded_selections.iter().filter(|record| record.source_request_id > 0
            && &record.selection_kind == kind && record.input_sha256 == hash
            && selection_input_hash(&record.request).ok().as_ref() == Some(&hash)
            && matches!(&record.outcome, DecisionRunOutcome::Completed { receipt, .. } if &receipt.resolved_model == model));
        let record = matches.next()?.clone();
        let DecisionRunOutcome::Completed {
            answers, receipt, ..
        } = &record.outcome
        else {
            return None;
        };
        let recorded_answers = serde_json::to_vec(answers).ok()?;
        for other in matches {
            let DecisionRunOutcome::Completed {
                answers, receipt, ..
            } = &other.outcome
            else {
                return None;
            };
            sunstack_decisions::judgment_processing::judgment_validation::validate_response(
                request,
                answers.clone(),
                receipt.clone(),
            )
            .ok()?;
            if serde_json::to_vec(answers).ok()? != recorded_answers {
                return None;
            }
        }
        sunstack_decisions::judgment_processing::judgment_validation::validate_response(
            request,
            answers.clone(),
            receipt.clone(),
        )
        .ok()?;
        let input = RunJudgmentsInput {
            request: request.clone(),
            backend: self.settings.backend.clone(),
            attempts: self.settings.attempts.clone(),
            request_budget: self.settings.request_budget.clone(),
        };
        prepare_judgments(&input, self.backend()?.as_ref()).ok()?;
        self.requested += 1;
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.prepared.push(PreparedModelRequest {
            id,
            request: request.clone(),
        });
        self.reused_selections.push(ReusedSelection {
            request_id: id,
            source_request_id: record.source_request_id,
            source_revision: record.source_revision.clone(),
            outcome: record.outcome.clone(),
        });
        Some((id, record.outcome))
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
    pub fn assess_selection(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<(), crate::SelectionIncompleteReason> {
        use crate::SelectionIncompleteReason as Reason;
        validate_choice_options(&request.questions).map_err(|_| Reason::InvalidRequest)?;
        let BackendSelection::JevOpenrouter { model, .. } = &self.settings.backend;
        let body = encode_judgment_body(request, model).map_err(|_| Reason::InvalidRequest)?;
        let chars = std::str::from_utf8(&body)
            .map_err(|_| Reason::InvalidRequest)?
            .chars()
            .count();
        if chars > self.settings.request_max_chars {
            return Err(Reason::RequestTooLarge {
                limit: RequestLimit::RequestMaxChars,
            });
        }
        let budget = &self.settings.request_budget;
        let Some(backend) = self.backend() else {
            return Err(Reason::InvalidRequest);
        };
        let encoded = backend
            .encode_request(request)
            .map_err(|_| Reason::InvalidRequest)?;
        let encoded_bytes = encoded.body().len();
        let accounting = backend.account_request(&encoded);
        match assess_request_budget(encoded, budget, accounting.clone()) {
            RequestBudgetAssessment::WithinConfiguredBudget { .. } => Ok(()),
            RequestBudgetAssessment::Unverifiable { .. } => Err(Reason::InvalidRequest),
            RequestBudgetAssessment::Exceeded { .. } => {
                let limit = if request.questions.len() > budget.max_questions {
                    RequestLimit::QuestionsPerRequest
                } else if encoded_bytes > budget.max_request_bytes {
                    RequestLimit::RequestBytes
                } else {
                    RequestLimit::InputTokens
                };
                Err(Reason::RequestTooLarge { limit })
            }
        }
    }
    pub async fn submit(
        &mut self,
        request: StructuredJudgmentRequest,
    ) -> Result<usize, DispatchRefusal> {
        if self.is_cancelled() {
            return Err(DispatchRefusal::Cancelled);
        }
        match self.check_capability() {
            // A local-only direct caller still observes existing request guards before the missing backend.
            Err(DispatchRefusal::InvalidRequest) => {}
            availability => availability?,
        }
        validate_choice_options(&request.questions)?;
        let BackendSelection::JevOpenrouter { model, .. } = &self.settings.backend;
        let body =
            encode_judgment_body(&request, model).map_err(|_| DispatchRefusal::InvalidRequest)?;
        let chars = std::str::from_utf8(&body)
            .map_err(|_| DispatchRefusal::InvalidRequest)?
            .chars()
            .count();
        if chars > self.settings.request_max_chars {
            return Err(DispatchRefusal::RequestTooLarge);
        }
        if self.requested >= self.settings.max_model_requests {
            return Err(DispatchRefusal::RequestLimitReached);
        }
        let backend = self
            .backend()
            .cloned()
            .ok_or(DispatchRefusal::InvalidRequest)?;
        let input = RunJudgmentsInput {
            request: request.clone(),
            backend: self.settings.backend.clone(),
            attempts: self.settings.attempts.clone(),
            request_budget: self.settings.request_budget.clone(),
        };
        if self.is_preparing() {
            // Ordinary execution reserves its allowance before local admission runs.
            self.requested += 1;
            prepare_judgments(&input, backend.as_ref())
                .map_err(|_| DispatchRefusal::InvalidRequest)?;
            let id = self.next_request_id;
            self.next_request_id += 1;
            self.prepared.push(PreparedModelRequest { id, request });
            return Ok(id);
        }
        let permit = tokio::select! {biased;_=self.cancellation.cancelled()=>return Err(DispatchRefusal::Cancelled),permit=self.semaphore.clone().acquire_owned()=>permit.map_err(|_|DispatchRefusal::InvalidRequest)?};
        if self.is_cancelled() {
            return Err(DispatchRefusal::Cancelled);
        }
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.requested += 1;
        let cancellation = self.cancellation.clone();
        self.tasks.spawn(async move {
            let _permit = permit;
            let outcome = run_judgments(&input, backend.as_ref(), &cancellation).await;
            DispatchedRequest {
                id,
                request,
                outcome,
            }
        });
        Ok(id)
    }
    pub async fn drain(&mut self) -> Vec<DispatchedRequest> {
        let mut completed = Vec::new();
        while let Some(result) = self.tasks.join_next().await {
            if let Ok(record) = result {
                completed.push(record);
            }
        }
        completed.sort_by_key(|record| record.id);
        self.records.extend(completed.iter().cloned());
        completed
    }
}

#[cfg(test)]
#[path = "model_request_dispatch_test.rs"]
mod tests;
