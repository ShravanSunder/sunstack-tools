use crate::{
    RulebookIncompleteReason, RulebookResult, RulebookRule, StepName, WalkOutcome, piece_state,
    walk_decision,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use sunstack_decisions::{
    AttemptSettings, BackendSelection, CancellationToken, DecisionBackend, DecisionRunOutcome,
    RequestBudgetSettings, RunJudgmentsInput, StructuredJudgmentRequest, encode_judgment_body,
    run_judgments,
};
use sunstack_evidence::{EvidencePiece, FileMatchIndex};
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Debug, Clone)]
pub struct RulebookRunSettings {
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
    pub request_max_chars: usize,
    pub max_concurrent_requests: usize,
    pub max_model_requests: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookPairResult {
    pub rule_index: usize,
    pub piece_index: usize,
    #[serde(flatten)]
    pub result: RulebookResult,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookRequestRecord {
    pub pair_index: usize,
    pub request: StructuredJudgmentRequest,
    pub outcome: DecisionRunOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookRun {
    pub results: Vec<RulebookPairResult>,
    pub requests: Vec<RulebookRequestRecord>,
}
fn incomplete(reason: RulebookIncompleteReason, step: Option<StepName>) -> RulebookResult {
    RulebookResult::Incomplete {
        reason,
        field: None,
        step,
    }
}

/// Walk and dispatch without acquiring files; each task owns one rule/piece request.
pub async fn run_rulebook(
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    indexes: &BTreeMap<PathBuf, FileMatchIndex>,
    settings: RulebookRunSettings,
    backend: Option<Arc<dyn DecisionBackend>>,
    cancellation: CancellationToken,
) -> RulebookRun {
    let mut run = RulebookRun {
        results: vec![],
        requests: vec![],
    };
    let semaphore = Arc::new(Semaphore::new(settings.max_concurrent_requests.max(1)));
    let mut tasks = JoinSet::new();
    let mut requested = 0usize;
    for (rule_index, rule) in rules.iter().enumerate() {
        for (piece_index, piece) in pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.kind == rule.kind)
        {
            let pair_index = run.results.len();
            run.results.push(RulebookPairResult {
                rule_index,
                piece_index,
                result: incomplete(RulebookIncompleteReason::ProviderFailure, None),
            });
            if cancellation.is_cancelled() {
                run.results[pair_index].result =
                    incomplete(RulebookIncompleteReason::Cancelled, None);
                continue;
            }
            let empty = FileMatchIndex::default();
            let index = indexes.get(&piece.location.file).unwrap_or(&empty);
            let pending_step = match walk_decision(rule, piece, index, None) {
                WalkOutcome::Settled(result) => {
                    run.results[pair_index].result = result;
                    continue;
                }
                WalkOutcome::NeedsAnswers { step } => step,
            };
            run.results[pair_index].result = incomplete(
                RulebookIncompleteReason::ProviderFailure,
                Some(pending_step.clone()),
            );
            let state = match piece_state(rule, piece) {
                Ok(state) => state,
                Err(_) => continue,
            };
            let request = StructuredJudgmentRequest {
                questions: rule.questions.clone(),
                state,
            };
            let BackendSelection::JevOpenrouter { model, .. } = &settings.backend;
            let body = match encode_judgment_body(&request, model) {
                Ok(body) => body,
                Err(_) => continue,
            };
            let chars = match std::str::from_utf8(&body) {
                Ok(body) => body.chars().count(),
                Err(_) => continue,
            };
            if chars > settings.request_max_chars {
                run.results[pair_index].result = incomplete(
                    RulebookIncompleteReason::RequestTooLarge,
                    Some(pending_step.clone()),
                );
                continue;
            }
            if requested >= settings.max_model_requests {
                run.results[pair_index].result = incomplete(
                    RulebookIncompleteReason::RequestLimitReached,
                    Some(pending_step.clone()),
                );
                continue;
            }
            let Some(backend) = backend.clone() else {
                continue;
            };
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => continue,
            };
            if cancellation.is_cancelled() {
                run.results[pair_index].result =
                    incomplete(RulebookIncompleteReason::Cancelled, Some(pending_step));
                continue;
            }
            requested += 1;
            let input = RunJudgmentsInput {
                request: request.clone(),
                backend: settings.backend.clone(),
                attempts: settings.attempts.clone(),
                request_budget: settings.request_budget.clone(),
            };
            let cancellation = cancellation.clone();
            let rule = rule.clone();
            let piece = piece.clone();
            let index = index.clone();
            tasks.spawn(async move {
                let _permit = permit;
                let outcome = run_judgments(&input, backend.as_ref(), &cancellation).await;
                let result = match &outcome {
                    DecisionRunOutcome::Completed { answers, .. } => {
                        match walk_decision(&rule, &piece, &index, Some(answers)) {
                            WalkOutcome::Settled(result) => result,
                            WalkOutcome::NeedsAnswers { .. } => incomplete(
                                RulebookIncompleteReason::ProviderFailure,
                                Some(pending_step.clone()),
                            ),
                        }
                    }
                    DecisionRunOutcome::Cancelled { .. } => incomplete(
                        RulebookIncompleteReason::Cancelled,
                        Some(pending_step.clone()),
                    ),
                    DecisionRunOutcome::InvalidRequest { .. }
                    | DecisionRunOutcome::ProviderFailure { .. }
                    | DecisionRunOutcome::InvalidResponse { .. } => incomplete(
                        RulebookIncompleteReason::ProviderFailure,
                        Some(pending_step),
                    ),
                };
                (
                    pair_index,
                    result,
                    RulebookRequestRecord {
                        pair_index,
                        request,
                        outcome,
                    },
                )
            });
        }
    }
    while let Some(completed) = tasks.join_next().await {
        if let Ok((pair_index, result, record)) = completed {
            if let Some(pair) = run.results.get_mut(pair_index) {
                pair.result = result;
            }
            run.requests.push(record);
        }
    }
    run.requests.sort_by_key(|record| record.pair_index);
    run
}
