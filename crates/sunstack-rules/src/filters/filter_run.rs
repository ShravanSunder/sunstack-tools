use super::filter_plan::*;
use crate::{DispatchRefusal, ModelRequestDispatch, RulebookIncompleteReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{
    BackendSelection, ChoiceLabel, DecisionRunOutcome, JudgmentAnswer, ModelName, QuestionName,
    StructuredJudgmentRequest,
};
use sunstack_evidence::{
    EvidenceKindDefinition, EvidenceKindName, EvidencePiece, FieldUnavailable, FieldValue,
    ModelView, project_model_view,
};

#[derive(Debug, Clone)]
pub enum ReadsPiece {
    One(usize),
    None,
    Ambiguous,
    Unavailable(FieldUnavailable),
}
#[derive(Debug, Clone)]
pub struct FilterFolder {
    pub filter: FilterName,
    pub folder: PathBuf,
    pub members: Vec<PathBuf>,
    pub reads: ReadsPiece,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilterIncomplete {
    Undescribed,
    Ambiguous,
    ReadsAbsent,
    ReadsUnavailable { reason: FieldUnavailable },
    ReadsTooLarge,
    Request { reason: RulebookIncompleteReason },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FilterAnswerState {
    Judged {
        answer: ChoiceLabel,
        probability: f64,
        probabilities: BTreeMap<ChoiceLabel, f64>,
    },
    Unsure {
        answer: ChoiceLabel,
        probability: f64,
        probabilities: BTreeMap<ChoiceLabel, f64>,
    },
    Incomplete {
        reason: FilterIncomplete,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterAnswer {
    pub filter: FilterName,
    pub folder: PathBuf,
    pub input_cut: bool,
    pub requested_model: ModelName,
    pub resolved_model: Option<ModelName>,
    #[serde(flatten)]
    pub state: FilterAnswerState,
}
#[derive(Debug, Clone, Default)]
pub struct FilterAnswers {
    pub entries: Vec<FilterAnswer>,
    pub folders: BTreeMap<(FilterName, PathBuf), PathBuf>,
}

fn request_for(
    plan: &FilterPlan,
    folder: &FilterFolder,
    pieces: &[EvidencePiece],
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    default_max_chars: usize,
) -> Result<(StructuredJudgmentRequest, bool), FilterIncomplete> {
    let index = match folder.reads {
        ReadsPiece::One(index) => index,
        ReadsPiece::None => return Err(FilterIncomplete::Undescribed),
        ReadsPiece::Ambiguous => return Err(FilterIncomplete::Ambiguous),
        ReadsPiece::Unavailable(reason) => {
            return Err(FilterIncomplete::ReadsUnavailable { reason });
        }
    };
    let piece = pieces.get(index).ok_or(FilterIncomplete::Undescribed)?;
    let field = match piece.fields.get(&plan.reads.field) {
        Some(value @ FieldValue::Text { .. }) => value.clone(),
        Some(FieldValue::Unavailable { reason }) => {
            return Err(FilterIncomplete::ReadsUnavailable { reason: *reason });
        }
        Some(FieldValue::Absent) | None => return Err(FilterIncomplete::ReadsAbsent),
        Some(FieldValue::Texts { .. } | FieldValue::Targets { .. }) => {
            return Err(FilterIncomplete::ReadsUnavailable {
                reason: FieldUnavailable::NotAString,
            });
        }
    };
    let max_chars = kinds
        .get(&plan.reads.kind)
        .and_then(|kind| kind.max_chars)
        .map_or(default_max_chars, |limit| limit.get() as usize);
    let view = project_model_view(
        &BTreeMap::from([(plan.reads.field.clone(), field)]),
        max_chars,
    );
    let ModelView::Bounded { fields, cut } = view else {
        return Err(FilterIncomplete::ReadsTooLarge);
    };
    let mut state = serde_json::Map::new();
    state.insert("where".into(), serde_json::json!({"folder":folder.folder}));
    let value = fields
        .get(&plan.reads.field)
        .ok_or(FilterIncomplete::ReadsAbsent)?;
    state.insert(
        plan.reads.field.as_str().into(),
        serde_json::to_value(value).map_err(|_| FilterIncomplete::Request {
            reason: RulebookIncompleteReason::InvalidRequest,
        })?,
    );
    if !cut.is_empty() {
        state.insert("cut-fields".into(), serde_json::json!(cut));
    }
    let question = QuestionName::new("filter").map_err(|_| FilterIncomplete::Request {
        reason: RulebookIncompleteReason::InvalidRequest,
    })?;
    Ok((
        StructuredJudgmentRequest {
            questions: BTreeMap::from([(question, plan.question.clone())]),
            state: serde_json::from_value(serde_json::Value::Object(state)).map_err(|_| {
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidRequest,
                }
            })?,
        },
        !cut.is_empty(),
    ))
}

pub async fn ask_filters(
    plans: &FilterRegistry,
    folders: &[FilterFolder],
    pieces: &[EvidencePiece],
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    default_max_chars: usize,
    dispatcher: &mut ModelRequestDispatch,
) -> FilterAnswers {
    let mut result = FilterAnswers::default();
    let mut pending = BTreeMap::new();
    let BackendSelection::JevOpenrouter { model, .. } = &dispatcher.settings.backend;
    let model = model.clone();
    for folder in folders {
        let Some(plan) = plans.get(&folder.filter) else {
            continue;
        };
        for member in &folder.members {
            result.folders.insert(
                (folder.filter.clone(), member.clone()),
                folder.folder.clone(),
            );
        }
        let position = result.entries.len();
        let mut answer = FilterAnswer {
            filter: folder.filter.clone(),
            folder: folder.folder.clone(),
            input_cut: false,
            requested_model: model.clone(),
            resolved_model: None,
            state: FilterAnswerState::Incomplete {
                reason: FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidResponse,
                },
            },
        };
        match request_for(plan, folder, pieces, kinds, default_max_chars) {
            Err(reason) => answer.state = FilterAnswerState::Incomplete { reason },
            Ok((request, cut)) => {
                answer.input_cut = cut;
                match dispatcher.submit(request).await {
                    Ok(id) => {
                        pending.insert(id, (position, plan.min_probability));
                    }
                    Err(refusal) => {
                        answer.state = FilterAnswerState::Incomplete {
                            reason: match refusal {
                                DispatchRefusal::RequestTooLarge => FilterIncomplete::ReadsTooLarge,
                                DispatchRefusal::Cancelled => FilterIncomplete::Request {
                                    reason: RulebookIncompleteReason::Cancelled,
                                },
                                DispatchRefusal::RequestLimitReached => FilterIncomplete::Request {
                                    reason: RulebookIncompleteReason::RequestLimitReached,
                                },
                                DispatchRefusal::ChoiceOptionsExceeded(error) => {
                                    FilterIncomplete::Request {
                                        reason: RulebookIncompleteReason::ChoiceOptionsExceeded {
                                            options: error.options,
                                            limit: error.limit,
                                        },
                                    }
                                }
                                DispatchRefusal::InvalidRequest => FilterIncomplete::Request {
                                    reason: RulebookIncompleteReason::InvalidRequest,
                                },
                            },
                        };
                    }
                }
            }
        }
        result.entries.push(answer);
    }
    for record in dispatcher.drain().await {
        let Some((position, threshold)) = pending.remove(&record.id) else {
            continue;
        };
        let answer = &mut result.entries[position];
        answer.state = match record.outcome {
            DecisionRunOutcome::Completed {
                answers, receipt, ..
            } => {
                answer.resolved_model = Some(receipt.resolved_model);
                match answers.values().next() {
                    Some(JudgmentAnswer::Choice {
                        selected_label,
                        probabilities,
                        ..
                    }) => match probabilities.get(selected_label) {
                        Some(probability) if *probability >= threshold => {
                            FilterAnswerState::Judged {
                                answer: selected_label.clone(),
                                probability: *probability,
                                probabilities: probabilities.clone(),
                            }
                        }
                        Some(probability) => FilterAnswerState::Unsure {
                            answer: selected_label.clone(),
                            probability: *probability,
                            probabilities: probabilities.clone(),
                        },
                        None => FilterAnswerState::Incomplete {
                            reason: FilterIncomplete::Request {
                                reason: RulebookIncompleteReason::InvalidResponse,
                            },
                        },
                    },
                    _ => FilterAnswerState::Incomplete {
                        reason: FilterIncomplete::Request {
                            reason: RulebookIncompleteReason::InvalidResponse,
                        },
                    },
                }
            }
            DecisionRunOutcome::ProviderFailure {
                failure_class,
                http_status,
                ..
            } => FilterAnswerState::Incomplete {
                reason: FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::ProviderFailure {
                        failure_class,
                        http_status,
                    },
                },
            },
            DecisionRunOutcome::InvalidRequest { .. } => FilterAnswerState::Incomplete {
                reason: FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidRequest,
                },
            },
            DecisionRunOutcome::InvalidResponse { .. } => FilterAnswerState::Incomplete {
                reason: FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidResponse,
                },
            },
            DecisionRunOutcome::Cancelled { .. } => FilterAnswerState::Incomplete {
                reason: FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::Cancelled,
                },
            },
        };
    }
    result
}
