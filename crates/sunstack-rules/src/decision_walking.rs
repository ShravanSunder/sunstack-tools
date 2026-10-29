use crate::{
    BinaryRoutes, DecisionMechanicalTool, DecisionRoute, DecisionStep, RulebookRule, Severity,
    StepName, rulebook_admission::normalized_answer,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentAnswer, NamedAnswerMap};
use sunstack_evidence::{
    EvidencePiece, FieldName, FieldValue, FileMatchIndex, MatchRange, ModelView, match_within,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RulebookIncompleteReason {
    EvidenceUnavailable,
    EvidenceTooLarge,
    RequestTooLarge,
    ProviderFailure,
    RequestLimitReached,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RulebookResult {
    Pass {
        path_taken: Vec<StepName>,
    },
    Finding {
        severity: Severity,
        message: String,
        line: usize,
        field: Option<FieldName>,
        path_taken: Vec<StepName>,
        answers: Option<NamedAnswerMap>,
        value: Option<f64>,
    },
    NotApplicable {
        path_taken: Vec<StepName>,
    },
    Uncertain {
        step: StepName,
        state: sunstack_decisions::SharedEvidenceState,
        answers: NamedAnswerMap,
        path_taken: Vec<StepName>,
    },
    Incomplete {
        reason: RulebookIncompleteReason,
        field: Option<FieldName>,
    },
}
#[derive(Debug, Clone)]
pub enum WalkOutcome {
    Settled(RulebookResult),
    NeedsAnswers { step: StepName },
}

pub fn piece_state(
    rule: &RulebookRule,
    piece: &EvidencePiece,
) -> Result<sunstack_decisions::SharedEvidenceState, serde_json::Error> {
    #[derive(Serialize)]
    struct PieceState<'a> {
        standard: &'a str,
        #[serde(rename = "where")]
        location: &'a sunstack_evidence::PieceLocation,
        #[serde(flatten)]
        fields: &'a BTreeMap<FieldName, sunstack_evidence::ModelFieldValue>,
    }
    let empty = BTreeMap::new();
    let available_fields = match &piece.model_view {
        ModelView::Bounded { fields, .. } => fields
            .iter()
            .filter(|(name, _)| {
                !matches!(
                    piece.fields.get(*name),
                    Some(FieldValue::Unavailable { .. })
                )
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        ModelView::TooLarge { .. } => empty,
    };
    serde_json::from_value(serde_json::to_value(PieceState {
        standard: &rule.sentence.standard,
        location: &piece.location,
        fields: &available_fields,
    })?)
}

pub fn walk_decision(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    index: &FileMatchIndex,
    answers: Option<&NamedAnswerMap>,
) -> WalkOutcome {
    let mut current = rule.decision.start.clone();
    let mut path = Vec::new();
    let mut combined_value = None;
    loop {
        let Some(step) = rule.decision.steps.get(&current) else {
            return incomplete(RulebookIncompleteReason::EvidenceUnavailable, None);
        };
        path.push(current.clone());
        if !matches!(step, DecisionStep::Mechanical { .. })
            && let Some(field) =
                crate::rulebook_admission::question_field_references(&rule.questions)
                    .into_iter()
                    .find(|field| {
                        matches!(
                            piece.fields.get(field),
                            Some(FieldValue::Unavailable { .. })
                        )
                    })
        {
            return incomplete(RulebookIncompleteReason::EvidenceUnavailable, Some(field));
        }
        if !matches!(step, DecisionStep::Mechanical { .. })
            && matches!(piece.model_view, ModelView::TooLarge { .. })
        {
            return incomplete(RulebookIncompleteReason::EvidenceTooLarge, None);
        }
        let route = match step {
            DecisionStep::Mechanical { tool, yes, no } => {
                match mechanical_check(tool, piece, index) {
                    Ok(true) => yes,
                    Ok(false) => no,
                    Err(field) => {
                        return incomplete(
                            RulebookIncompleteReason::EvidenceUnavailable,
                            Some(field),
                        );
                    }
                }
            }
            DecisionStep::AskNoul { question, routes }
            | DecisionStep::AskScore {
                question, routes, ..
            } => {
                let Some(answers) = answers else {
                    return needs_answers(piece, &current);
                };
                let Some(value) = normalized_answer(question, &rule.questions, answers) else {
                    return incomplete(RulebookIncompleteReason::ProviderFailure, None);
                };
                let Some(route) = binary_route(value, routes) else {
                    return uncertain(rule, piece, current, answers, path);
                };
                route
            }
            DecisionStep::AskChoice {
                question,
                min_probability,
                routes,
            } => {
                let Some(answers) = answers else {
                    return needs_answers(piece, &current);
                };
                let Some(JudgmentAnswer::Choice {
                    selected_label,
                    probabilities,
                    ..
                }) = answers.get(question)
                else {
                    return incomplete(RulebookIncompleteReason::ProviderFailure, None);
                };
                let Some(probability) = probabilities.get(selected_label) else {
                    return incomplete(RulebookIncompleteReason::ProviderFailure, None);
                };
                if probability < min_probability {
                    return uncertain(rule, piece, current, answers, path);
                }
                let Some(route) = routes.get(selected_label) else {
                    return incomplete(RulebookIncompleteReason::ProviderFailure, None);
                };
                route
            }
            DecisionStep::Combine { weights, routes } => {
                let Some(answers) = answers else {
                    return needs_answers(piece, &current);
                };
                let value = weights.iter().try_fold(0.0, |sum, (question, weight)| {
                    normalized_answer(question, &rule.questions, answers)
                        .map(|value| sum + weight * value)
                });
                let Some(value) = value else {
                    return incomplete(RulebookIncompleteReason::ProviderFailure, None);
                };
                combined_value = Some(value);
                let Some(route) = binary_route(value, routes) else {
                    return uncertain(rule, piece, current, answers, path);
                };
                route
            }
        };
        match route {
            DecisionRoute::Step(next) => current = next.clone(),
            DecisionRoute::Pass => {
                return WalkOutcome::Settled(RulebookResult::Pass { path_taken: path });
            }
            DecisionRoute::NotApplicable => {
                return WalkOutcome::Settled(RulebookResult::NotApplicable { path_taken: path });
            }
            DecisionRoute::Uncertain => {
                return uncertain(
                    rule,
                    piece,
                    current,
                    answers.unwrap_or(&NamedAnswerMap::new()),
                    path,
                );
            }
            DecisionRoute::Finding(finding) => {
                let line = finding
                    .at
                    .as_ref()
                    .and_then(|field| piece.fields.get(field))
                    .and_then(|field| field.spans().first())
                    .map_or(piece.location.first_line, |span| span.start_line);
                return WalkOutcome::Settled(RulebookResult::Finding {
                    severity: finding.severity.unwrap_or(rule.severity),
                    message: finding.finding.clone(),
                    line,
                    field: finding.at.clone(),
                    path_taken: path,
                    answers: answers.cloned(),
                    value: combined_value,
                });
            }
        }
    }
}
fn incomplete(reason: RulebookIncompleteReason, field: Option<FieldName>) -> WalkOutcome {
    WalkOutcome::Settled(RulebookResult::Incomplete { reason, field })
}
fn needs_answers(piece: &EvidencePiece, step: &StepName) -> WalkOutcome {
    match piece.model_view {
        ModelView::TooLarge { .. } => incomplete(RulebookIncompleteReason::EvidenceTooLarge, None),
        ModelView::Bounded { .. } => WalkOutcome::NeedsAnswers { step: step.clone() },
    }
}
fn uncertain(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    step: StepName,
    answers: &NamedAnswerMap,
    path_taken: Vec<StepName>,
) -> WalkOutcome {
    match piece_state(rule, piece) {
        Ok(state) => WalkOutcome::Settled(RulebookResult::Uncertain {
            step,
            state,
            answers: answers.clone(),
            path_taken,
        }),
        Err(_) => incomplete(RulebookIncompleteReason::EvidenceUnavailable, None),
    }
}
fn binary_route(value: f64, routes: &BinaryRoutes) -> Option<&DecisionRoute> {
    if value >= routes.yes_at_or_above {
        Some(&routes.yes)
    } else if value <= routes.no_at_or_below {
        Some(&routes.no)
    } else {
        None
    }
}
fn mechanical_check(
    tool: &DecisionMechanicalTool,
    piece: &EvidencePiece,
    index: &FileMatchIndex,
) -> Result<bool, FieldName> {
    let field = match tool {
        DecisionMechanicalTool::AstGrep { rule } => {
            return Ok(index.by_rule.get(rule).into_iter().flatten().any(|span| {
                match_within(
                    span,
                    &MatchRange {
                        start: piece.location.span.byte_start,
                        end: piece.location.span.byte_end,
                    },
                )
            }));
        }
        DecisionMechanicalTool::Regex { field, .. }
        | DecisionMechanicalTool::Contains { field, .. }
        | DecisionMechanicalTool::MaxChars { field, .. }
        | DecisionMechanicalTool::FileExists { field } => field,
    };
    let value = piece.fields.get(field);
    if matches!(value, Some(FieldValue::Unavailable { .. })) {
        return Err(field.clone());
    }
    let texts = match value {
        Some(FieldValue::Text { value, .. }) => vec![value.as_str()],
        Some(FieldValue::Texts { values, .. }) => values.iter().map(String::as_str).collect(),
        _ => vec![],
    };
    Ok(match tool {
        DecisionMechanicalTool::Regex { pattern, .. } => regex::Regex::new(pattern)
            .is_ok_and(|regex| texts.iter().any(|text| regex.is_match(text))),
        DecisionMechanicalTool::Contains { text, .. } => {
            texts.iter().any(|value| value.contains(text))
        }
        DecisionMechanicalTool::MaxChars { max, .. } => {
            texts.first().map_or(0, |text| text.chars().count()) <= *max
        }
        DecisionMechanicalTool::FileExists { .. } => {
            matches!(value, Some(FieldValue::Targets { values }) if !values.is_empty() && values.iter().all(|target| target.exists))
        }
        DecisionMechanicalTool::AstGrep { .. } => false,
    })
}

#[cfg(test)]
#[path = "decision_walking_test.rs"]
mod tests;
