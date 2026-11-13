use crate::{
    AdmittedSelection, DispatchRefusal, ModelRequestDispatch, SelectionIncompleteReason,
    SelectionInput, SelectionOutcome, unavailable_question_field,
};
use std::collections::BTreeMap;
use sunstack_decisions::{
    DecisionRunOutcome, JudgmentAnswer, JudgmentQuestion, NamedQuestionMap, QuestionName,
    SharedEvidenceState, StructuredEntry, StructuredJson, StructuredJudgmentRequest,
};
use sunstack_evidence::{CandidateId, FieldName, ModelView, SelectionCandidate, SharedView};

#[derive(serde::Serialize)]
struct SelectionViewState<'a> {
    #[serde(flatten)]
    fields: &'a BTreeMap<FieldName, sunstack_evidence::ModelFieldValue>,
    #[serde(rename = "cut-fields", skip_serializing_if = "<[FieldName]>::is_empty")]
    cut_fields: &'a [FieldName],
}

fn incomplete(reason: SelectionIncompleteReason) -> SelectionOutcome {
    SelectionOutcome::Incomplete { reason }
}
fn question_map(selection: &AdmittedSelection) -> NamedQuestionMap {
    QuestionName::new("selection")
        .ok()
        .map(|name| BTreeMap::from([(name, selection.question.clone())]))
        .unwrap_or_default()
}
fn qualify(value: &mut StructuredJson, candidate: &SelectionCandidate, shared: &[FieldName]) {
    match value {
        StructuredJson::Text(name) => {
            *name = if shared.iter().any(|field| field.as_str() == name) {
                format!("shared.{name}")
            } else {
                format!("candidates.{}.{name}", candidate.key.as_str())
            };
        }
        StructuredJson::Array(names) => {
            for name in names {
                qualify(name, candidate, shared);
            }
        }
        _ => {}
    }
}
fn request_for(
    selection: &AdmittedSelection,
    candidates: &[&SelectionCandidate],
    shared: &SharedView,
) -> Result<StructuredJudgmentRequest, SelectionIncompleteReason> {
    let mut questions = NamedQuestionMap::new();
    let mut views = serde_json::Map::new();
    for candidate in candidates {
        let mut question = selection.question.clone();
        if let JudgmentQuestion::Noul {
            instructions: StructuredEntry::Object(instructions),
            ..
        } = &mut question
        {
            for key in ["check", "against", "context"] {
                if let Some(value) = instructions.get_mut(key) {
                    qualify(value, candidate, &selection.shared);
                }
            }
        }
        let name = QuestionName::new(candidate.key.as_str())
            .map_err(|_| SelectionIncompleteReason::InvalidRequest)?;
        questions.insert(name, question);
        let ModelView::Bounded { fields, cut } = &candidate.selection_view else {
            return Err(SelectionIncompleteReason::EvidenceTooLarge);
        };
        views.insert(
            candidate.key.as_str().into(),
            serde_json::to_value(SelectionViewState {
                fields,
                cut_fields: cut,
            })
            .map_err(|_| SelectionIncompleteReason::InvalidRequest)?,
        );
    }
    let ModelView::Bounded { fields, cut } = &shared.view else {
        return Err(SelectionIncompleteReason::EvidenceTooLarge);
    };
    let shared_state = SelectionViewState {
        fields,
        cut_fields: cut,
    };
    let state: SharedEvidenceState = serde_json::from_value(
        serde_json::json!({"where":{"file":shared.file},"candidates":views,"shared":shared_state}),
    )
    .map_err(|_| SelectionIncompleteReason::InvalidRequest)?;
    Ok(StructuredJudgmentRequest { questions, state })
}
async fn send_group(
    selection: &AdmittedSelection,
    candidates: &[&SelectionCandidate],
    shared: &SharedView,
    dispatcher: &mut ModelRequestDispatch,
    outcomes: &mut BTreeMap<CandidateId, SelectionOutcome>,
    pending: &mut BTreeMap<usize, Vec<(CandidateId, String)>>,
) {
    let request = match request_for(selection, candidates, shared) {
        Ok(request) => request,
        Err(reason) => {
            for candidate in candidates {
                outcomes.insert(candidate.id.clone(), incomplete(reason.clone()));
            }
            return;
        }
    };
    if let Some((id, recorded)) = dispatcher.reuse_selection(&selection.kind, &request) {
        dispatcher
            .selection_requests
            .push(crate::SelectionRequestOwner {
                request_id: id,
                selection_kind: selection.kind.clone(),
                file: shared.file.clone(),
                candidates: candidates
                    .iter()
                    .map(|candidate| (*candidate).clone())
                    .collect(),
                shared: shared.clone(),
            });
        for candidate in candidates {
            outcomes.insert(
                candidate.id.clone(),
                outcome_for(selection, candidate.key.as_str(), &recorded),
            );
        }
        return;
    }
    match dispatcher.submit(request).await {
        Ok(id) => {
            dispatcher
                .selection_requests
                .push(crate::SelectionRequestOwner {
                    request_id: id,
                    selection_kind: selection.kind.clone(),
                    file: shared.file.clone(),
                    candidates: candidates
                        .iter()
                        .map(|candidate| (*candidate).clone())
                        .collect(),
                    shared: shared.clone(),
                });
            if dispatcher.is_preparing() {
                for candidate in candidates {
                    outcomes.insert(
                        candidate.id.clone(),
                        SelectionOutcome::Prepared { request_id: id },
                    );
                }
                return;
            }
            pending.insert(
                id,
                candidates
                    .iter()
                    .map(|candidate| (candidate.id.clone(), candidate.key.as_str().to_owned()))
                    .collect(),
            );
        }
        Err(refusal) => {
            let reason = match refusal {
                DispatchRefusal::AdmissionRefused { diagnostics } => {
                    SelectionIncompleteReason::AdmissionRefused { diagnostics }
                }
                DispatchRefusal::Cancelled => SelectionIncompleteReason::Cancelled,
                DispatchRefusal::RequestLimitReached => {
                    SelectionIncompleteReason::RequestLimitReached
                }
                DispatchRefusal::ChoiceOptionsExceeded(_) | DispatchRefusal::InvalidRequest => {
                    SelectionIncompleteReason::InvalidRequest
                }
            };
            for candidate in candidates {
                outcomes.insert(candidate.id.clone(), incomplete(reason.clone()));
            }
        }
    }
}
/// Settle only candidates in the requested files; absent entries remain not_asked.
pub async fn select_candidates(
    input: &SelectionInput,
    dispatcher: &mut ModelRequestDispatch,
) -> BTreeMap<CandidateId, SelectionOutcome> {
    let mut outcomes = BTreeMap::new();
    let mut pending = BTreeMap::new();
    let questions = question_map(&input.selection);
    for shared in &input.shared {
        if !input.files.contains(&shared.file) {
            continue;
        }
        let mut group = Vec::new();
        for candidate in input
            .candidates
            .iter()
            .filter(|candidate| candidate.location.file == shared.file)
        {
            outcomes.insert(
                candidate.id.clone(),
                incomplete(SelectionIncompleteReason::InvalidResponse),
            );
            let mut fields = candidate.fields.clone();
            fields.extend(shared.fields.clone());
            if dispatcher.is_preparing()
                && fields
                    .values()
                    .any(sunstack_evidence::FieldValue::is_selection_pending)
            {
                outcomes.insert(candidate.id.clone(), SelectionOutcome::Deferred);
                continue;
            }
            if let Some(field) = unavailable_question_field(&questions, &fields) {
                outcomes.insert(
                    candidate.id.clone(),
                    incomplete(SelectionIncompleteReason::EvidenceUnavailable { field }),
                );
                continue;
            }
            if matches!(candidate.selection_view, ModelView::TooLarge { .. })
                || matches!(shared.view, ModelView::TooLarge { .. })
            {
                outcomes.insert(
                    candidate.id.clone(),
                    incomplete(SelectionIncompleteReason::EvidenceTooLarge),
                );
                continue;
            }
            let single = request_for(&input.selection, &[candidate], shared)
                .and_then(|request| dispatcher.assess_selection(&request));
            if let Err(reason) = single {
                outcomes.insert(candidate.id.clone(), incomplete(reason));
                continue;
            }
            let mut trial = group.clone();
            trial.push(candidate);
            let fits = request_for(&input.selection, &trial, shared)
                .and_then(|request| dispatcher.assess_selection(&request))
                .is_ok();
            if !fits && !group.is_empty() {
                send_group(
                    &input.selection,
                    &group,
                    shared,
                    dispatcher,
                    &mut outcomes,
                    &mut pending,
                )
                .await;
                group.clear();
            }
            group.push(candidate);
        }
        if !group.is_empty() {
            send_group(
                &input.selection,
                &group,
                shared,
                dispatcher,
                &mut outcomes,
                &mut pending,
            )
            .await;
        }
    }
    for record in dispatcher.drain().await {
        let Some(candidates) = pending.remove(&record.id) else {
            continue;
        };
        for (id, key) in candidates {
            let outcome = outcome_for(&input.selection, &key, &record.outcome);
            outcomes.insert(id, outcome);
        }
    }
    outcomes
}

fn outcome_for(
    selection: &AdmittedSelection,
    key: &str,
    record: &DecisionRunOutcome,
) -> SelectionOutcome {
    match record {
        DecisionRunOutcome::Completed { answers, .. } => {
            let answer = QuestionName::new(key)
                .ok()
                .and_then(|name| answers.get(&name));
            match answer {
                Some(JudgmentAnswer::Noul { yes_probability }) => {
                    let yes = *yes_probability;
                    if yes >= selection.yes_at_or_above {
                        SelectionOutcome::Selected { yes }
                    } else if yes <= selection.no_at_or_below {
                        SelectionOutcome::NotSelected { yes }
                    } else {
                        SelectionOutcome::Uncertain { yes }
                    }
                }
                _ => incomplete(SelectionIncompleteReason::InvalidResponse),
            }
        }
        DecisionRunOutcome::Cancelled { .. } => incomplete(SelectionIncompleteReason::Cancelled),
        DecisionRunOutcome::InvalidRequest { .. } => {
            incomplete(SelectionIncompleteReason::InvalidRequest)
        }
        DecisionRunOutcome::InvalidResponse { .. } => {
            incomplete(SelectionIncompleteReason::InvalidResponse)
        }
        DecisionRunOutcome::ProviderFailure {
            failure_class,
            http_status,
            ..
        } => incomplete(SelectionIncompleteReason::ProviderFailure {
            failure_class: *failure_class,
            http_status: *http_status,
        }),
    }
}

#[cfg(test)]
#[path = "selection_run_test.rs"]
mod tests;
