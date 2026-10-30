use crate::{
    DispatchRefusal, ModelRequestDispatch, RulebookIncompleteReason, RulebookResult, RulebookRule,
    StepName, WalkOutcome, piece_state, walk_decision,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use sunstack_decisions::{
    AttemptSettings, BackendSelection, CancellationToken, DecisionBackend, DecisionRunOutcome,
    RequestBudgetSettings, StructuredJudgmentRequest,
};
use sunstack_evidence::{EvidencePiece, FileMatchIndex};
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

/// A direct rules consumer creates the same dispatcher that lint shares with selection.
pub async fn run_rulebook(
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    indexes: &BTreeMap<PathBuf, FileMatchIndex>,
    settings: RulebookRunSettings,
    backend: Option<Arc<dyn DecisionBackend>>,
    cancellation: CancellationToken,
) -> RulebookRun {
    let mut dispatcher = ModelRequestDispatch::new(settings, backend, cancellation);
    run_rulebook_with_dispatcher(rules, pieces, indexes, &mut dispatcher).await
}
pub async fn run_rulebook_with_dispatcher(
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    indexes: &BTreeMap<PathBuf, FileMatchIndex>,
    dispatcher: &mut ModelRequestDispatch,
) -> RulebookRun {
    let mut run = RulebookRun {
        results: vec![],
        requests: vec![],
    };
    let mut pending = BTreeMap::new();
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
            if dispatcher.is_cancelled() {
                run.results[pair_index].result =
                    incomplete(RulebookIncompleteReason::Cancelled, None);
                continue;
            }
            let empty = FileMatchIndex::default();
            let index = indexes.get(&piece.location.file).unwrap_or(&empty);
            let step = match walk_decision(rule, piece, index, None) {
                WalkOutcome::Settled(result) => {
                    run.results[pair_index].result = result;
                    continue;
                }
                WalkOutcome::NeedsAnswers { step } => step,
            };
            run.results[pair_index].result = incomplete(
                RulebookIncompleteReason::ProviderFailure,
                Some(step.clone()),
            );
            let Ok(state) = piece_state(rule, piece) else {
                continue;
            };
            let request = StructuredJudgmentRequest {
                questions: rule.questions.clone(),
                state,
            };
            match dispatcher.submit(request).await {
                Ok(id) => {
                    pending.insert(id, (pair_index, step));
                }
                Err(refusal) => {
                    let reason = match refusal {
                        DispatchRefusal::Cancelled => RulebookIncompleteReason::Cancelled,
                        DispatchRefusal::RequestTooLarge => {
                            RulebookIncompleteReason::RequestTooLarge
                        }
                        DispatchRefusal::RequestLimitReached => {
                            RulebookIncompleteReason::RequestLimitReached
                        }
                        DispatchRefusal::InvalidRequest => {
                            RulebookIncompleteReason::ProviderFailure
                        }
                    };
                    run.results[pair_index].result = incomplete(reason, Some(step));
                }
            }
        }
    }
    for record in dispatcher.drain().await {
        let Some((pair_index, step)) = pending.remove(&record.id) else {
            continue;
        };
        let pair = &run.results[pair_index];
        let rule = &rules[pair.rule_index];
        let piece = &pieces[pair.piece_index];
        let empty = FileMatchIndex::default();
        let index = indexes.get(&piece.location.file).unwrap_or(&empty);
        let result = match &record.outcome {
            DecisionRunOutcome::Completed { answers, .. } => {
                match walk_decision(rule, piece, index, Some(answers)) {
                    WalkOutcome::Settled(result) => result,
                    WalkOutcome::NeedsAnswers { .. } => {
                        incomplete(RulebookIncompleteReason::ProviderFailure, Some(step))
                    }
                }
            }
            DecisionRunOutcome::Cancelled { .. } => {
                incomplete(RulebookIncompleteReason::Cancelled, Some(step))
            }
            DecisionRunOutcome::InvalidRequest { .. }
            | DecisionRunOutcome::ProviderFailure { .. }
            | DecisionRunOutcome::InvalidResponse { .. } => {
                incomplete(RulebookIncompleteReason::ProviderFailure, Some(step))
            }
        };
        run.results[pair_index].result = result;
        run.requests.push(RulebookRequestRecord {
            pair_index,
            request: record.request,
            outcome: record.outcome,
        });
    }
    run.requests.sort_by_key(|record| record.pair_index);
    run
}
