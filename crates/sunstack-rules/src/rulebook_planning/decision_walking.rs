use crate::{
    BinaryRoutes, DecisionMechanicalTool, DecisionRoute, RulebookRule, Severity, StepName,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentAnswer, NamedAnswerMap, NamedQuestionMap};
use sunstack_evidence::{
    EvidencePiece, FieldName, FieldValue, FileMatchIndex, MatchRange, ModelView, match_within,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RulebookIncompleteReason {
    ChoiceOptionsExceeded {
        options: usize,
        limit: usize,
    },
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locator_omitted: Option<LocatorOmission>,
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
        reason: NotApplicableReason,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocatorOmission {
    pub locator: sunstack_decisions::QuestionName,
    pub field: FieldName,
    pub reason: LocatorOmissionReason,
    pub candidate_lines: usize,
    pub limit: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocatorOmissionReason {
    LineLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotApplicableReason {
    Route,
    Filter {
        filter: crate::FilterName,
        label: sunstack_decisions::ChoiceLabel,
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
            Self::NotApplicable {
                reason: NotApplicableReason::Filter { .. },
                ..
            } => Some(Basis::Judged),
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
    piece_state_for_questions(rule, piece, &rule.questions)
}

pub(super) fn piece_state_for_questions(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    questions: &NamedQuestionMap,
) -> Result<sunstack_decisions::SharedEvidenceState, serde_json::Error> {
    #[derive(Serialize)]
    struct RequestLocation<'a> {
        file: &'a std::path::Path,
        first_line: usize,
        last_line: usize,
    }
    #[derive(Serialize)]
    struct PieceState<'a> {
        standard: &'a str,
        #[serde(rename = "where")]
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<RequestLocation<'a>>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        reference: &'a crate::ReferenceMaterial,
        #[serde(rename = "unavailable-fields")]
        unavailable_fields: BTreeMap<FieldName, sunstack_evidence::FieldUnavailable>,
        #[serde(flatten)]
        fields: &'a BTreeMap<FieldName, sunstack_evidence::ModelFieldValue>,
        #[serde(rename = "cut-fields", skip_serializing_if = "<[FieldName]>::is_empty")]
        cut_fields: &'a [FieldName],
    }
    let references = crate::rulebook_planning::rulebook_admission::question_state_paths(questions);
    let empty = BTreeMap::new();
    let available_fields = match &piece.model_view {
        ModelView::Bounded { fields, .. } => fields
            .iter()
            .filter(|(name, _)| {
                references
                    .iter()
                    .any(|field| field.as_str().split('.').next() == Some(name.as_str()))
            })
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
        location: references
            .iter()
            .any(|field| field.as_str() == "where" || field.as_str().starts_with("where."))
            .then_some(RequestLocation {
                file: &piece.location.file,
                first_line: piece.location.first_line,
                last_line: piece.location.last_line,
            }),
        reference: &rule.reference,
        unavailable_fields: piece
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                FieldValue::Unavailable { reason } if references.contains(name.as_str()) => {
                    Some((name.clone(), *reason))
                }
                _ => None,
            })
            .collect(),
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
    walk_decision_with_omissions(rule, piece, index, answers, &BTreeMap::new())
}

pub(crate) fn walk_decision_with_omissions(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    index: &FileMatchIndex,
    answers: Option<&NamedAnswerMap>,
    omissions: &BTreeMap<sunstack_decisions::QuestionName, LocatorOmission>,
) -> WalkOutcome {
    use super::tree_walking::{TreeWalkOutcome, TreeWalkRequest, walk_tree};
    let outcome = walk_tree(TreeWalkRequest {
        decision: &rule.decision,
        questions: &rule.questions,
        piece,
        index,
        answers,
    });
    let (route, current, path, mut answers_read, combined_value) = match outcome {
        TreeWalkOutcome::NeedsAnswers { step, path_taken } => {
            return WalkOutcome::NeedsAnswers { step, path_taken };
        }
        TreeWalkOutcome::Incomplete {
            reason,
            field,
            step,
            path_taken,
            answers_read,
        } => {
            return WalkOutcome::Settled(RulebookResult::Incomplete {
                reason,
                field,
                step,
                path_taken,
                answers_read,
            });
        }
        TreeWalkOutcome::Ended {
            route,
            step,
            path_taken,
            answers_read,
            combined_value,
        } => (route, step, path_taken, answers_read, combined_value),
    };
    match &route {
        DecisionRoute::Step(_) | DecisionRoute::Label(_) => incomplete(
            RulebookIncompleteReason::InvalidRequest,
            None,
            path,
            answers_read,
        ),
        DecisionRoute::Pass => WalkOutcome::Settled(RulebookResult::Pass {
            path_taken: path,
            answers_read,
        }),
        DecisionRoute::NotApplicable => WalkOutcome::Settled(RulebookResult::NotApplicable {
            reason: NotApplicableReason::Route,
            path_taken: path,
            answers_read,
        }),
        DecisionRoute::Uncertain => uncertain(
            rule,
            piece,
            current,
            answers.unwrap_or(&NamedAnswerMap::new()),
            path,
            answers_read,
        ),
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
                                        || (selected_line == span.end_line && span.end_column > 1))
                            })
                        })
                {
                    line = selected_line;
                    anchor = Anchor::Resolved;
                }
            }
            WalkOutcome::Settled(RulebookResult::Finding {
                locator_omitted: finding
                    .locate
                    .as_ref()
                    .and_then(|locator| omissions.get(&locator.name))
                    .cloned(),
                severity: finding.severity.unwrap_or(rule.severity),
                message: finding.finding.clone(),
                line,
                field: finding.at.clone(),
                path_taken: path,
                answers: answers.cloned(),
                value: combined_value,
                answers_read,
                anchor,
            })
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
pub(super) fn binary_route(value: f64, routes: &BinaryRoutes) -> Option<&DecisionRoute> {
    if value >= routes.yes_at_or_above {
        Some(&routes.yes)
    } else if value <= routes.no_at_or_below {
        Some(&routes.no)
    } else {
        None
    }
}
pub(super) fn mechanical_check(
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
        | DecisionMechanicalTool::FileExists { field }
        | DecisionMechanicalTool::Complete { field } => field,
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
        DecisionMechanicalTool::Complete { .. } => {
            let bounded =
                matches!(&piece.model_view, ModelView::Bounded {cut, ..} if !cut.contains(field));
            bounded
                && match value {
                    Some(FieldValue::Targets { values }) => values.iter().all(|target| {
                        target.exists
                            && target.candidates.is_empty()
                            && target.opening.is_some()
                            && !target.opening_cut
                    }),
                    Some(
                        FieldValue::Text { .. } | FieldValue::Texts { .. } | FieldValue::Absent,
                    ) => true,
                    Some(FieldValue::Unavailable { .. }) | None => false,
                }
        }
        DecisionMechanicalTool::AstGrep { .. } => false,
    })
}

/// The shared referenced-field guard for rule and evidence-selection questions.
pub fn unavailable_question_field(
    questions: &sunstack_decisions::NamedQuestionMap,
    fields: &BTreeMap<FieldName, FieldValue>,
) -> Option<FieldName> {
    crate::rulebook_planning::rulebook_admission::question_field_references(questions)
        .into_iter()
        .find(|field| matches!(fields.get(field), Some(FieldValue::Unavailable { .. })))
}

#[cfg(test)]
#[path = "decision_walking_test.rs"]
mod tests;
