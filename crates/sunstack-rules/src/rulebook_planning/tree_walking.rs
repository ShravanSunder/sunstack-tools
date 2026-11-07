use super::decision_walking::{binary_route, mechanical_check, unavailable_question_field};
use super::rulebook_admission::normalized_answer;
use crate::{DecisionGraph, DecisionRoute, DecisionStep, RulebookIncompleteReason, StepName};
use sunstack_decisions::{JudgmentAnswer, NamedAnswerMap, NamedQuestionMap};
use sunstack_evidence::{EvidencePiece, FieldName, FileMatchIndex, ModelView};

pub(crate) struct TreeWalkRequest<'a> {
    pub decision: &'a DecisionGraph,
    pub questions: &'a NamedQuestionMap,
    pub piece: &'a EvidencePiece,
    pub index: &'a FileMatchIndex,
    pub answers: Option<&'a NamedAnswerMap>,
}
pub(crate) enum TreeWalkOutcome {
    NeedsAnswers {
        step: StepName,
        path_taken: Vec<StepName>,
    },
    Incomplete {
        reason: RulebookIncompleteReason,
        field: Option<FieldName>,
        step: Option<StepName>,
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
    },
    Ended {
        route: DecisionRoute,
        step: StepName,
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
        combined_value: Option<f64>,
    },
}
pub(crate) fn walk_tree(request: TreeWalkRequest<'_>) -> TreeWalkOutcome {
    let mut current = request.decision.start.clone();
    let mut path = Vec::new();
    let mut answers_read = NamedAnswerMap::new();
    let mut combined_value = None;
    loop {
        let Some(step) = request.decision.steps.get(&current) else {
            return TreeWalkOutcome::Incomplete {
                reason: RulebookIncompleteReason::EvidenceUnavailable,
                field: None,
                step: None,
                path_taken: path,
                answers_read,
            };
        };
        path.push(current.clone());
        let incomplete =
            |reason, field, answers_read: &NamedAnswerMap| TreeWalkOutcome::Incomplete {
                reason,
                field,
                step: Some(current.clone()),
                path_taken: path.clone(),
                answers_read: answers_read.clone(),
            };
        if !matches!(step, DecisionStep::Mechanical { .. })
            && let Some(field) =
                unavailable_question_field(request.questions, &request.piece.fields)
        {
            return incomplete(
                RulebookIncompleteReason::EvidenceUnavailable,
                Some(field),
                &answers_read,
            );
        }
        if !matches!(step, DecisionStep::Mechanical { .. })
            && matches!(request.piece.model_view, ModelView::TooLarge { .. })
        {
            return incomplete(
                RulebookIncompleteReason::EvidenceTooLarge,
                None,
                &answers_read,
            );
        }
        let route = match step {
            DecisionStep::Mechanical { tool, yes, no } => {
                match mechanical_check(tool, request.piece, request.index) {
                    Ok(true) => yes,
                    Ok(false) => no,
                    Err(field) => {
                        return incomplete(
                            RulebookIncompleteReason::EvidenceUnavailable,
                            Some(field),
                            &answers_read,
                        );
                    }
                }
            }
            DecisionStep::AskNoul { question, routes }
            | DecisionStep::AskScore {
                question, routes, ..
            } => {
                let Some(answers) = request.answers else {
                    return TreeWalkOutcome::NeedsAnswers {
                        step: current,
                        path_taken: path,
                    };
                };
                let Some(value) = normalized_answer(question, request.questions, answers) else {
                    return incomplete(
                        RulebookIncompleteReason::InvalidResponse,
                        None,
                        &answers_read,
                    );
                };
                if let Some(answer) = answers.get(question) {
                    answers_read.insert(question.clone(), answer.clone());
                }
                binary_route(value, routes).unwrap_or(&routes.uncertain)
            }
            DecisionStep::AskChoice {
                uncertain,
                question,
                min_probability,
                routes,
            } => {
                let Some(answers) = request.answers else {
                    return TreeWalkOutcome::NeedsAnswers {
                        step: current,
                        path_taken: path,
                    };
                };
                let Some(JudgmentAnswer::Choice {
                    selected_label,
                    probabilities,
                    ..
                }) = answers.get(question)
                else {
                    return incomplete(
                        RulebookIncompleteReason::InvalidResponse,
                        None,
                        &answers_read,
                    );
                };
                let Some(probability) = probabilities.get(selected_label) else {
                    return incomplete(
                        RulebookIncompleteReason::InvalidResponse,
                        None,
                        &answers_read,
                    );
                };
                if let Some(answer) = answers.get(question) {
                    answers_read.insert(question.clone(), answer.clone());
                }
                if probability < min_probability {
                    uncertain
                } else {
                    let Some(route) = routes.get(selected_label) else {
                        return incomplete(
                            RulebookIncompleteReason::InvalidResponse,
                            None,
                            &answers_read,
                        );
                    };
                    route
                }
            }
            DecisionStep::Combine { weights, routes } => {
                let Some(answers) = request.answers else {
                    return TreeWalkOutcome::NeedsAnswers {
                        step: current,
                        path_taken: path,
                    };
                };
                let value = weights.iter().try_fold(0.0, |sum, (question, weight)| {
                    let value = normalized_answer(question, request.questions, answers)?;
                    if let Some(answer) = answers.get(question) {
                        answers_read.insert(question.clone(), answer.clone());
                    }
                    Some(sum + weight * value)
                });
                let Some(value) = value else {
                    return incomplete(
                        RulebookIncompleteReason::InvalidResponse,
                        None,
                        &answers_read,
                    );
                };
                combined_value = Some(value);
                binary_route(value, routes).unwrap_or(&routes.uncertain)
            }
        };
        match route {
            DecisionRoute::Step(next) => current = next.clone(),
            _ => {
                return TreeWalkOutcome::Ended {
                    route: route.clone(),
                    step: current,
                    path_taken: path,
                    answers_read,
                    combined_value,
                };
            }
        }
    }
}
