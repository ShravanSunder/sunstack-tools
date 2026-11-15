use crate::{CaseBindingStatus, CaseExecutionOwner, IncompleteReason, LintReport, PieceOutcome};
use sunstack_rules::{FilterAnswerState, RulebookRule};

/// Binding remains independent of execution; unresolved walks keep their actual ending.
pub(super) fn settle_case_accounting(
    report: &mut LintReport,
    rules: &[RulebookRule],
    cancelled: bool,
) {
    let Some(selection) = &mut report.case_selection else {
        return;
    };
    for binding in &mut selection.bindings {
        let CaseBindingStatus::Selected { owner } = &binding.outcome else {
            continue;
        };
        let (finished, incomplete, dependency) = match owner {
            CaseExecutionOwner::Rule { rule, file, lines } => {
                let result = report.results.iter().find(|result| {
                    &result.name == rule
                        && &result.file == file
                        && result.first_line == lines[0]
                        && result.last_line == lines[1]
                });
                let prepared = report.requests.iter().any(|request| {
                    matches!(
                        &request.owner,
                        crate::RequestOwner::Rule { rule: actual, piece }
                            if actual == rule && &piece.file == file
                                && [piece.first_line, piece.last_line] == *lines
                    ) && matches!(request.outcome, crate::SavedRequestOutcome::Prepared { .. })
                });
                let finished = prepared
                    || result
                        .is_some_and(|result| !matches!(result.outcome, PieceOutcome::Incomplete));
                let dependency = rules
                    .iter()
                    .find(|candidate| &candidate.name == rule)
                    .map(|rule| (rule.kind.clone(), file.clone()));
                (finished, result.is_some(), dependency)
            }
            CaseExecutionOwner::Filter { filter, folder } => {
                let result = report
                    .filter_answers
                    .iter()
                    .find(|answer| &answer.filter == filter && &answer.folder == folder);
                let prepared = report.requests.iter().any(|request| matches!(
                    &request.owner, crate::RequestOwner::Filter { filter: actual, folder: actual_folder }
                        if actual == filter && actual_folder == folder
                ) && matches!(request.outcome, crate::SavedRequestOutcome::Prepared { .. }));
                (
                    prepared
                        || result.is_some_and(|answer| {
                            !matches!(answer.state, FilterAnswerState::Incomplete { .. })
                        }),
                    result.is_some(),
                    None,
                )
            }
        };
        if finished {
            continue;
        }
        if cancelled {
            binding.outcome = CaseBindingStatus::Cancelled;
        } else if let Some((kind, file)) = dependency
            && let Some(wait) = report.preparation.as_ref().and_then(|preparation| {
                preparation
                    .needs_answer
                    .iter()
                    .find(|wait| wait.kind == kind && wait.file == file)
            })
        {
            binding.outcome = CaseBindingStatus::SelectionPending {
                evidence_kind: wait.kind.clone(),
                file: wait.file.clone(),
            };
        } else if incomplete {
            let reason = report.incomplete.iter().find(|part| match owner {
                CaseExecutionOwner::Rule { rule, file, .. } => part.rule.as_ref() == Some(rule) && part.file.as_ref() == Some(file),
                CaseExecutionOwner::Filter { filter, folder } => part.file.as_ref() == Some(folder)
                    && matches!(&part.reason, IncompleteReason::Filter { filter: actual, .. } if actual == filter),
            }).map(|part| part.reason.clone()).unwrap_or(IncompleteReason::EvaluationFailed);
            binding.outcome = CaseBindingStatus::Incomplete { reason };
        }
    }
}
