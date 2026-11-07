use super::filter_plan::*;
use crate::{DispatchRefusal, ModelRequestDispatch, RulebookIncompleteReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{
    BackendSelection, ChoiceLabel, DecisionRunOutcome, ModelName, StructuredJudgmentRequest,
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
        label: ChoiceLabel,
        path: Vec<crate::StepName>,
        answers: sunstack_decisions::NamedAnswerMap,
    },
    Unsure {
        step: crate::StepName,
        path: Vec<crate::StepName>,
        answers: sunstack_decisions::NamedAnswerMap,
    },
    Incomplete {
        reason: FilterIncomplete,
        #[serde(skip_serializing_if = "Option::is_none")]
        field: Option<sunstack_evidence::FieldName>,
        path: Vec<crate::StepName>,
        answers: sunstack_decisions::NamedAnswerMap,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterAnswer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reads_sha256: Option<String>,
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
    pub requests: Vec<FilterRequestRecord>,
    pub not_sent: Vec<UnsentFilterRequest>,
    pub pieces: BTreeMap<(FilterName, PathBuf), EvidencePiece>,
}

struct PreparedFilterRequest {
    reads_sha256: String,
    request: StructuredJudgmentRequest,
    piece: EvidencePiece,
    input_cut: bool,
}
struct FilterRequestFailure {
    reason: FilterIncomplete,
    field: Option<sunstack_evidence::FieldName>,
}
fn filter_piece_for(
    plan: &FilterPlan,
    folder: &FilterFolder,
    pieces: &[EvidencePiece],
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    default_max_chars: usize,
) -> Result<EvidencePiece, FilterRequestFailure> {
    let failure = |reason| FilterRequestFailure {
        reason,
        field: None,
    };
    let index = match folder.reads {
        ReadsPiece::One(index) => index,
        ReadsPiece::None => return Err(failure(FilterIncomplete::Undescribed)),
        ReadsPiece::Ambiguous => return Err(failure(FilterIncomplete::Ambiguous)),
        ReadsPiece::Unavailable(reason) => {
            return Err(failure(FilterIncomplete::ReadsUnavailable { reason }));
        }
    };
    let mut piece = pieces
        .get(index)
        .ok_or_else(|| failure(FilterIncomplete::Undescribed))?
        .clone();
    piece
        .fields
        .retain(|field, _| plan.reads.fields.contains(field));
    let max_chars = kinds
        .get(&plan.reads.kind)
        .and_then(|kind| kind.max_chars)
        .map_or(default_max_chars, |limit| limit.get() as usize);
    piece.model_view = project_model_view(&piece.fields, max_chars);
    Ok(piece)
}
fn request_for(
    plan: &FilterPlan,
    piece: EvidencePiece,
) -> Result<PreparedFilterRequest, FilterRequestFailure> {
    let failure = |reason| FilterRequestFailure {
        reason,
        field: None,
    };
    let mut fields = BTreeMap::new();
    for field in &plan.reads.fields {
        let value = match piece.fields.get(field) {
            Some(FieldValue::Unavailable { reason }) => {
                return Err(FilterRequestFailure {
                    reason: FilterIncomplete::ReadsUnavailable { reason: *reason },
                    field: Some(field.clone()),
                });
            }
            Some(FieldValue::Absent) | None => {
                return Err(FilterRequestFailure {
                    reason: FilterIncomplete::ReadsAbsent,
                    field: Some(field.clone()),
                });
            }
            Some(value) => value.clone(),
        };
        fields.insert(field.clone(), value);
    }
    let ModelView::Bounded {
        fields: unprojected,
        ..
    } = project_model_view(&fields, usize::MAX)
    else {
        return Err(failure(FilterIncomplete::ReadsTooLarge));
    };
    let reads_sha256 = serde_json::to_vec(&unprojected)
        .map(|bytes| sunstack_evidence::content_digest(&bytes))
        .map_err(|_| {
            failure(FilterIncomplete::Request {
                reason: RulebookIncompleteReason::InvalidRequest,
            })
        })?;
    let ModelView::Bounded { fields, cut } = &piece.model_view else {
        return Err(failure(FilterIncomplete::ReadsTooLarge));
    };
    let paths = crate::rulebook_planning::rulebook_admission::question_state_paths(&plan.questions);
    let visible = fields
        .iter()
        .filter(|(name, _)| {
            paths
                .iter()
                .any(|path| path.split('.').next() == Some(name.as_str()))
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    #[derive(Serialize)]
    struct FilterState<'a> {
        #[serde(flatten)]
        fields: &'a BTreeMap<sunstack_evidence::FieldName, sunstack_evidence::ModelFieldValue>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        reference: &'a crate::ReferenceMaterial,
        #[serde(
            rename = "cut-fields",
            skip_serializing_if = "<[sunstack_evidence::FieldName]>::is_empty"
        )]
        cut: &'a [sunstack_evidence::FieldName],
        #[serde(rename = "unavailable-fields")]
        unavailable: BTreeMap<sunstack_evidence::FieldName, FieldUnavailable>,
    }
    let input_cut = !cut.is_empty();
    let state = serde_json::to_value(FilterState {
        fields: &visible,
        reference: &plan.reference,
        cut,
        unavailable: BTreeMap::new(),
    })
    .and_then(serde_json::from_value)
    .map_err(|_| {
        failure(FilterIncomplete::Request {
            reason: RulebookIncompleteReason::InvalidRequest,
        })
    })?;
    Ok(PreparedFilterRequest {
        reads_sha256,
        request: StructuredJudgmentRequest {
            questions: plan.questions.clone(),
            state,
        },
        piece,
        input_cut,
    })
}

fn request_failure(refusal: DispatchRefusal) -> FilterIncomplete {
    FilterIncomplete::Request {
        reason: match refusal {
            DispatchRefusal::RequestTooLarge => return FilterIncomplete::ReadsTooLarge,
            DispatchRefusal::Cancelled => RulebookIncompleteReason::Cancelled,
            DispatchRefusal::RequestLimitReached => RulebookIncompleteReason::RequestLimitReached,
            DispatchRefusal::ChoiceOptionsExceeded(error) => {
                RulebookIncompleteReason::ChoiceOptionsExceeded {
                    options: error.options,
                    limit: error.limit,
                }
            }
            DispatchRefusal::InvalidRequest => RulebookIncompleteReason::InvalidRequest,
        },
    }
}
fn incomplete_state(
    reason: FilterIncomplete,
    field: Option<sunstack_evidence::FieldName>,
) -> FilterAnswerState {
    FilterAnswerState::Incomplete {
        reason,
        field,
        path: vec![],
        answers: Default::default(),
    }
}
fn walk_filter(
    plan: &FilterPlan,
    piece: &EvidencePiece,
    index: &sunstack_evidence::FileMatchIndex,
    answers: &sunstack_decisions::NamedAnswerMap,
) -> FilterAnswerState {
    use crate::rulebook_planning::tree_walking::{TreeWalkOutcome, TreeWalkRequest, walk_tree};
    match walk_tree(TreeWalkRequest {
        decision: &plan.decision,
        questions: &plan.questions,
        piece,
        index,
        answers: Some(answers),
    }) {
        TreeWalkOutcome::Ended {
            route: crate::DecisionRoute::Label(label),
            path_taken,
            answers_read,
            ..
        } => FilterAnswerState::Judged {
            label,
            path: path_taken,
            answers: answers_read,
        },
        TreeWalkOutcome::Ended {
            route: crate::DecisionRoute::Uncertain,
            step,
            path_taken,
            answers_read,
            ..
        } => FilterAnswerState::Unsure {
            step,
            path: path_taken,
            answers: answers_read,
        },
        TreeWalkOutcome::Incomplete {
            reason,
            field,
            path_taken,
            answers_read,
            ..
        } => FilterAnswerState::Incomplete {
            reason: FilterIncomplete::Request { reason },
            field,
            path: path_taken,
            answers: answers_read,
        },
        _ => incomplete_state(
            FilterIncomplete::Request {
                reason: RulebookIncompleteReason::InvalidRequest,
            },
            None,
        ),
    }
}
#[derive(Debug, Clone)]
pub struct UnsentFilterRequest {
    pub filter: FilterName,
    pub folder: PathBuf,
    pub request: StructuredJudgmentRequest,
    pub reason: FilterIncomplete,
}
#[derive(Debug, Clone)]
pub struct FilterRequestRecord {
    pub filter: FilterName,
    pub folder: PathBuf,
    pub request: StructuredJudgmentRequest,
    pub outcome: DecisionRunOutcome,
}
pub async fn ask_filters(
    plans: &FilterRegistry,
    folders: &[FilterFolder],
    pieces: &[EvidencePiece],
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    matches: &BTreeMap<PathBuf, sunstack_evidence::FileMatchIndex>,
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
            reads_sha256: None,
            filter: folder.filter.clone(),
            folder: folder.folder.clone(),
            input_cut: false,
            requested_model: model.clone(),
            resolved_model: None,
            state: incomplete_state(
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidResponse,
                },
                None,
            ),
        };
        let prepared =
            filter_piece_for(plan, folder, pieces, kinds, default_max_chars).and_then(|piece| {
                result.pieces.insert(
                    (folder.filter.clone(), folder.folder.clone()),
                    piece.clone(),
                );
                request_for(plan, piece)
            });
        match prepared {
            Err(failure) => answer.state = incomplete_state(failure.reason, failure.field),
            Ok(prepared) => {
                answer.reads_sha256 = Some(prepared.reads_sha256);
                answer.input_cut = prepared.input_cut;
                match dispatcher.submit(prepared.request.clone()).await {
                    Ok(id) => {
                        pending.insert(id, (position, prepared.piece));
                    }
                    Err(refusal) => {
                        let reason = request_failure(refusal);
                        result.not_sent.push(UnsentFilterRequest {
                            filter: folder.filter.clone(),
                            folder: folder.folder.clone(),
                            request: prepared.request,
                            reason: reason.clone(),
                        });
                        answer.state = incomplete_state(reason, None);
                    }
                }
            }
        }
        result.entries.push(answer);
    }
    for record in dispatcher.drain().await {
        let Some((position, piece)) = pending.remove(&record.id) else {
            continue;
        };
        let answer = &mut result.entries[position];
        let Some(plan) = plans.get(&answer.filter) else {
            continue;
        };
        answer.state = match &record.outcome {
            DecisionRunOutcome::Completed {
                answers, receipt, ..
            } => {
                answer.resolved_model = Some(receipt.resolved_model.clone());
                let empty = sunstack_evidence::FileMatchIndex::default();
                walk_filter(
                    plan,
                    &piece,
                    matches.get(&piece.location.file).unwrap_or(&empty),
                    answers,
                )
            }
            DecisionRunOutcome::ProviderFailure {
                failure_class,
                http_status,
                ..
            } => incomplete_state(
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::ProviderFailure {
                        failure_class: *failure_class,
                        http_status: *http_status,
                    },
                },
                None,
            ),
            DecisionRunOutcome::InvalidRequest { .. } => incomplete_state(
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidRequest,
                },
                None,
            ),
            DecisionRunOutcome::InvalidResponse { .. } => incomplete_state(
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::InvalidResponse,
                },
                None,
            ),
            DecisionRunOutcome::Cancelled { .. } => incomplete_state(
                FilterIncomplete::Request {
                    reason: RulebookIncompleteReason::Cancelled,
                },
                None,
            ),
        };
        result.requests.push(FilterRequestRecord {
            filter: answer.filter.clone(),
            folder: answer.folder.clone(),
            request: record.request,
            outcome: record.outcome,
        });
    }
    result
}

#[cfg(test)]
#[path = "filter_run_test.rs"]
mod tests;
