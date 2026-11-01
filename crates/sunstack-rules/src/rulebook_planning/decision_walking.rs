use crate::{
    BinaryRoutes, DecisionMechanicalTool, DecisionRoute, DecisionStep, RulebookRule, Severity,
    StepName, rulebook_planning::rulebook_admission::normalized_answer,
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
    ProviderFailure {
        failure_class: sunstack_decisions::ProviderFailureClass,
        #[serde(skip_serializing_if = "Option::is_none")]
        http_status: Option<u16>,
    },
    InvalidRequest,
    InvalidResponse,
    RequestLimitReached,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RulebookResult {
    Pass {
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
    },
    Finding {
        severity: Severity,
        message: String,
        line: usize,
        field: Option<FieldName>,
        path_taken: Vec<StepName>,
        answers: Option<NamedAnswerMap>,
        value: Option<f64>,
        answers_read: NamedAnswerMap,
        anchor: Anchor,
    },
    NotApplicable {
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
    },
    Uncertain {
        step: StepName,
        state: sunstack_decisions::SharedEvidenceState,
        answers: NamedAnswerMap,
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
    },
    Incomplete {
        reason: RulebookIncompleteReason,
        field: Option<FieldName>,
        step: Option<StepName>,
        path_taken: Vec<StepName>,
        answers_read: NamedAnswerMap,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Mechanical,
    Judged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    Resolved,
    Default,
}

impl RulebookResult {
    pub fn path_taken(&self) -> &[StepName] {
        match self {
            Self::Pass { path_taken, .. }
            | Self::Finding { path_taken, .. }
            | Self::NotApplicable { path_taken, .. }
            | Self::Uncertain { path_taken, .. }
            | Self::Incomplete { path_taken, .. } => path_taken,
        }
    }

    pub fn answers_read(&self) -> &NamedAnswerMap {
        match self {
            Self::Pass { answers_read, .. }
            | Self::Finding { answers_read, .. }
            | Self::NotApplicable { answers_read, .. }
            | Self::Uncertain { answers_read, .. }
            | Self::Incomplete { answers_read, .. } => answers_read,
        }
    }

    pub fn basis(&self) -> Option<Basis> {
        match self {
            Self::Incomplete { .. } => None,
            Self::Pass { .. }
            | Self::Finding { .. }
            | Self::NotApplicable { .. }
            | Self::Uncertain { .. } => Some(if self.answers_read().is_empty() {
                Basis::Mechanical
            } else {
                Basis::Judged
            }),
        }
    }
}
#[derive(Debug, Clone)]
pub enum WalkOutcome {
    Settled(RulebookResult),
    NeedsAnswers {
        step: StepName,
        path_taken: Vec<StepName>,
    },
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
        #[serde(rename = "cut-fields", skip_serializing_if = "<[FieldName]>::is_empty")]
        cut_fields: &'a [FieldName],
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
        cut_fields: match &piece.model_view {
            ModelView::Bounded { cut, .. } => cut,
            ModelView::TooLarge { .. } => &[],
        },
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
    let mut answers_read = NamedAnswerMap::new();
    let mut combined_value = None;
    loop {
        let Some(step) = rule.decision.steps.get(&current) else {
            return incomplete(
                RulebookIncompleteReason::EvidenceUnavailable,
                None,
                path,
                answers_read,
            );
        };
        path.push(current.clone());
        let incomplete = |reason, field, answers_read: &NamedAnswerMap| {
            WalkOutcome::Settled(RulebookResult::Incomplete {
                reason,
                field,
                step: Some(current.clone()),
                path_taken: path.clone(),
                answers_read: answers_read.clone(),
            })
        };
        if !matches!(step, DecisionStep::Mechanical { .. })
            && let Some(field) = unavailable_question_field(&rule.questions, &piece.fields)
        {
            return incomplete(
                RulebookIncompleteReason::EvidenceUnavailable,
                Some(field),
                &answers_read,
            );
        }
        if !matches!(step, DecisionStep::Mechanical { .. })
            && matches!(piece.model_view, ModelView::TooLarge { .. })
        {
            return incomplete(
                RulebookIncompleteReason::EvidenceTooLarge,
                None,
                &answers_read,
            );
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
                            &answers_read,
                        );
                    }
                }
            }
            DecisionStep::AskNoul { question, routes }
            | DecisionStep::AskScore {
                question, routes, ..
            } => {
                let Some(answers) = answers else {
                    return WalkOutcome::NeedsAnswers {
                        step: current,
                        path_taken: path,
                    };
                };
                let Some(value) = normalized_answer(question, &rule.questions, answers) else {
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
                let Some(answers) = answers else {
                    return WalkOutcome::NeedsAnswers {
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
                let Some(answers) = answers else {
                    return WalkOutcome::NeedsAnswers {
                        step: current,
                        path_taken: path,
                    };
                };
                let value = weights.iter().try_fold(0.0, |sum, (question, weight)| {
                    let value = normalized_answer(question, &rule.questions, answers)?;
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
            DecisionRoute::Pass => {
                return WalkOutcome::Settled(RulebookResult::Pass {
                    path_taken: path,
                    answers_read,
                });
            }
            DecisionRoute::NotApplicable => {
                return WalkOutcome::Settled(RulebookResult::NotApplicable {
                    path_taken: path,
                    answers_read,
                });
            }
            DecisionRoute::Uncertain => {
                return uncertain(
                    rule,
                    piece,
                    current,
                    answers.unwrap_or(&NamedAnswerMap::new()),
                    path,
                    answers_read,
                );
            }
            DecisionRoute::Finding(finding) => {
                let mut line = finding
                    .at
                    .as_ref()
                    .and_then(|field| piece.fields.get(field))
                    .and_then(|field| field.spans().first())
                    .map_or(piece.location.first_line, |span| span.start_line);
                let mut anchor = Anchor::Default;
                if let Some(locator) = &finding.locate
                    && let Some(
                        answer @ JudgmentAnswer::Choice {
                            selected_label,
                            probabilities,
                            ..
                        },
                    ) = answers.and_then(|answers| answers.get(&locator.name))
                {
                    answers_read.insert(locator.name.clone(), answer.clone());
                    if probabilities
                        .get(selected_label)
                        .is_some_and(|probability| *probability >= locator.min_probability)
                        && let Some(selected_line) = selected_label
                            .as_str()
                            .strip_prefix('L')
                            .and_then(|label| label.parse::<usize>().ok())
                        && (piece.location.first_line..=piece.location.last_line)
                            .contains(&selected_line)
                        && finding
                            .at
                            .as_ref()
                            .and_then(|field| piece.fields.get(field))
                            .is_some_and(|field| {
                                field.spans().iter().any(|span| {
                                    span.source == piece.location.span.source
                                        && selected_line >= span.start_line
                                        && (selected_line < span.end_line
                                            || (selected_line == span.end_line
                                                && span.end_column > 1))
                                })
                            })
                    {
                        line = selected_line;
                        anchor = Anchor::Resolved;
                    }
                }
                return WalkOutcome::Settled(RulebookResult::Finding {
                    severity: finding.severity.unwrap_or(rule.severity),
                    message: finding.finding.clone(),
                    line,
                    field: finding.at.clone(),
                    path_taken: path,
                    answers: answers.cloned(),
                    value: combined_value,
                    answers_read,
                    anchor,
                });
            }
        }
    }
}
fn incomplete(
    reason: RulebookIncompleteReason,
    field: Option<FieldName>,
    path_taken: Vec<StepName>,
    answers_read: NamedAnswerMap,
) -> WalkOutcome {
    WalkOutcome::Settled(RulebookResult::Incomplete {
        reason,
        field,
        step: None,
        path_taken,
        answers_read,
    })
}
fn uncertain(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    step: StepName,
    answers: &NamedAnswerMap,
    path_taken: Vec<StepName>,
    answers_read: NamedAnswerMap,
) -> WalkOutcome {
    match piece_state(rule, piece) {
        Ok(state) => WalkOutcome::Settled(RulebookResult::Uncertain {
            step,
            state,
            answers: answers.clone(),
            path_taken,
            answers_read,
        }),
        Err(_) => WalkOutcome::Settled(RulebookResult::Incomplete {
            reason: RulebookIncompleteReason::EvidenceUnavailable,
            field: None,
            step: Some(step),
            path_taken,
            answers_read,
        }),
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
mod tests;

/// The shared referenced-field guard for rule and evidence-selection questions.
pub fn unavailable_question_field(
    questions: &sunstack_decisions::NamedQuestionMap,
    fields: &BTreeMap<FieldName, FieldValue>,
) -> Option<FieldName> {
    crate::rulebook_planning::rulebook_admission::question_field_references(questions)
        .into_iter()
        .find(|field| matches!(fields.get(field), Some(FieldValue::Unavailable { .. })))
}
