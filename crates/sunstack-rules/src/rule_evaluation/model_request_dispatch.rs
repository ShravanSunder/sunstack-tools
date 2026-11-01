use crate::{RequestLimit, RulebookRunSettings};
use std::sync::Arc;
use sunstack_decisions::*;
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Debug, Clone, thiserror::Error)]
pub enum DispatchRefusal {
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
/// One request allowance, concurrency owner and receipt list for selection and rule work.
pub struct ModelRequestDispatch {
    pub settings: RulebookRunSettings,
    backend: Option<Arc<dyn DecisionBackend>>,
    cancellation: CancellationToken,
    semaphore: Arc<Semaphore>,
    tasks: JoinSet<DispatchedRequest>,
    requested: usize,
    pub records: Vec<DispatchedRequest>,
}
impl ModelRequestDispatch {
    pub fn new(
        settings: RulebookRunSettings,
        backend: Option<Arc<dyn DecisionBackend>>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(settings.max_concurrent_requests.max(1))),
            settings,
            backend,
            cancellation,
            tasks: JoinSet::new(),
            requested: 0,
            records: vec![],
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
    pub fn assess_selection(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<(), crate::SelectionIncompleteReason> {
        use crate::SelectionIncompleteReason as Reason;
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
        let Some(backend) = &self.backend else {
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
            .backend
            .clone()
            .ok_or(DispatchRefusal::InvalidRequest)?;
        let permit = tokio::select! {biased;_=self.cancellation.cancelled()=>return Err(DispatchRefusal::Cancelled),permit=self.semaphore.clone().acquire_owned()=>permit.map_err(|_|DispatchRefusal::InvalidRequest)?};
        if self.is_cancelled() {
            return Err(DispatchRefusal::Cancelled);
        }
        let id = self.requested;
        self.requested += 1;
        let input = RunJudgmentsInput {
            request: request.clone(),
            backend: self.settings.backend.clone(),
            attempts: self.settings.attempts.clone(),
            request_budget: self.settings.request_budget.clone(),
        };
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
