use crate::{
    DispatchRefusal, ModelRequestDispatch, RulebookIncompleteReason, RulebookResult, RulebookRule,
    StepName, WalkOutcome, piece_state,
    rulebook_planning::decision_walking::walk_decision_with_omissions,
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
    pub max_locate_lines: usize,
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
pub struct UnsentRulebookRequest {
    pub pair_index: usize,
    pub request: StructuredJudgmentRequest,
    pub reason: RulebookIncompleteReason,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookRun {
    pub not_sent: Vec<UnsentRulebookRequest>,
    pub skips: Vec<crate::FilterSkip>,
    pub results: Vec<RulebookPairResult>,
    pub requests: Vec<RulebookRequestRecord>,
}
fn incomplete(
    reason: RulebookIncompleteReason,
    step: Option<StepName>,
    path_taken: Vec<StepName>,
) -> RulebookResult {
    RulebookResult::Incomplete {
        reason,
        field: None,
        step,
        path_taken,
        answers_read: BTreeMap::new(),
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
    run_rulebook_with_dispatcher(
        rules,
        pieces,
        indexes,
        &mut dispatcher,
        &crate::FilterAnswers::default(),
    )
    .await
}
pub async fn run_rulebook_with_dispatcher(
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    indexes: &BTreeMap<PathBuf, FileMatchIndex>,
    dispatcher: &mut ModelRequestDispatch,
    filters: &crate::FilterAnswers,
) -> RulebookRun {
    let mut run = RulebookRun {
        not_sent: vec![],
        skips: vec![],
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
            match crate::gate_for(rule, filters, &piece.location.file) {
                crate::GateOutcome::Walk => {}
                crate::GateOutcome::Exclude(reason) => {
                    run.results.push(RulebookPairResult {
                        rule_index,
                        piece_index,
                        result: RulebookResult::NotApplicable {
                            reason,
                            path_taken: vec![],
                            answers_read: BTreeMap::new(),
                        },
                    });
                    continue;
                }
                crate::GateOutcome::Skip(mut skip) => {
                    if let Some(prior) = run.skips.iter_mut().find(|prior| {
                        prior.rule == skip.rule
                            && prior.file == skip.file
                            && prior.filter == skip.filter
                    }) {
                        if !prior.locations.contains(&piece.location) {
                            prior.locations.push(piece.location.clone());
                        }
                    } else {
                        skip.locations.push(piece.location.clone());
                        run.skips.push(skip);
                    }
                    continue;
                }
            }
            let pair_index = run.results.len();
            run.results.push(RulebookPairResult {
                rule_index,
                piece_index,
                result: incomplete(RulebookIncompleteReason::InvalidResponse, None, vec![]),
            });
            if dispatcher.is_cancelled() {
                run.results[pair_index].result =
                    incomplete(RulebookIncompleteReason::Cancelled, None, vec![]);
                continue;
            }
            let empty = FileMatchIndex::default();
            let index = indexes.get(&piece.location.file).unwrap_or(&empty);
            let omissions = super::locator_questions::plan_locator_omissions(
                rule,
                piece,
                dispatcher.settings.max_locate_lines,
            );
            let (step, path_taken) = match first_walk(rule, piece, index, &omissions) {
                WalkOutcome::Settled(result) => {
                    run.results[pair_index].result = result;
                    continue;
                }
                WalkOutcome::NeedsAnswers { step, path_taken } => (step, path_taken),
            };
            run.results[pair_index].result = incomplete(
                RulebookIncompleteReason::InvalidResponse,
                Some(step.clone()),
                path_taken.clone(),
            );
            let Ok(state) = piece_state(rule, piece) else {
                run.results[pair_index].result = incomplete(
                    RulebookIncompleteReason::EvidenceUnavailable,
                    Some(step),
                    path_taken,
                );
                continue;
            };
            let mut request = StructuredJudgmentRequest {
                questions: rule.questions.clone(),
                state,
            };
            let submitted =
                match super::locator_questions::add_locators(rule, piece, dispatcher, &mut request)
                {
                    Ok(()) => dispatcher.submit(request.clone()).await,
                    Err(refusal) => Err(refusal),
                };
            match submitted {
                Ok(id) => {
                    pending.insert(id, (pair_index, step, path_taken, omissions));
                }
                Err(refusal) => {
                    let reason = match refusal {
                        DispatchRefusal::ChoiceOptionsExceeded(error) => {
                            RulebookIncompleteReason::ChoiceOptionsExceeded {
                                options: error.options,
                                limit: error.limit,
                            }
                        }
                        DispatchRefusal::Cancelled => RulebookIncompleteReason::Cancelled,
                        DispatchRefusal::RequestTooLarge => {
                            RulebookIncompleteReason::RequestTooLarge
                        }
                        DispatchRefusal::RequestLimitReached => {
                            RulebookIncompleteReason::RequestLimitReached
                        }
                        DispatchRefusal::InvalidRequest => RulebookIncompleteReason::InvalidRequest,
                    };
                    run.not_sent.push(UnsentRulebookRequest {
                        pair_index,
                        request,
                        reason,
                    });
                    run.results[pair_index].result = incomplete(reason, Some(step), path_taken);
                }
            }
        }
    }
    for record in dispatcher.drain().await {
        let Some((pair_index, step, path_taken, omissions)) = pending.remove(&record.id) else {
            continue;
        };
        let pair = &run.results[pair_index];
        let rule = &rules[pair.rule_index];
        let piece = &pieces[pair.piece_index];
        let empty = FileMatchIndex::default();
        let index = indexes.get(&piece.location.file).unwrap_or(&empty);
        let result = match &record.outcome {
            DecisionRunOutcome::Completed { answers, .. } => {
                match walk_decision_with_omissions(rule, piece, index, Some(answers), &omissions) {
                    WalkOutcome::Settled(result) => result,
                    WalkOutcome::NeedsAnswers { .. } => incomplete(
                        RulebookIncompleteReason::InvalidResponse,
                        Some(step),
                        path_taken,
                    ),
                }
            }
            DecisionRunOutcome::Cancelled { .. } => {
                incomplete(RulebookIncompleteReason::Cancelled, Some(step), path_taken)
            }
            DecisionRunOutcome::InvalidRequest { .. } => incomplete(
                RulebookIncompleteReason::InvalidRequest,
                Some(step),
                path_taken,
            ),
            DecisionRunOutcome::InvalidResponse { .. } => incomplete(
                RulebookIncompleteReason::InvalidResponse,
                Some(step),
                path_taken,
            ),
            DecisionRunOutcome::ProviderFailure {
                failure_class,
                http_status,
                ..
            } => incomplete(
                RulebookIncompleteReason::ProviderFailure {
                    failure_class: *failure_class,
                    http_status: *http_status,
                },
                Some(step),
                path_taken,
            ),
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

fn first_walk(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    index: &FileMatchIndex,
    omissions: &BTreeMap<sunstack_decisions::QuestionName, crate::LocatorOmission>,
) -> WalkOutcome {
    #[cfg(test)]
    FIRST_WALKS.with(|walks| walks.set(walks.get() + 1));
    walk_decision_with_omissions(rule, piece, index, None, omissions)
}

#[cfg(test)]
thread_local! { static FIRST_WALKS:std::cell::Cell<usize>=const {std::cell::Cell::new(0)}; }
#[cfg(test)]
#[path = "rulebook_run_test.rs"]
mod tests;
