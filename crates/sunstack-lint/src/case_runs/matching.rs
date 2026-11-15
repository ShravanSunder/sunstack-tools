use super::contracts::*;
use crate::{LintReport, RulePieceEnding};
use std::path::Path;
use sunstack_rules::{FilterAnswerState, FilterSkipReason};

fn incomplete(reason: impl ToString) -> CaseOutcome {
    CaseOutcome::Incomplete {
        reason: reason.to_string(),
    }
}
fn archived_lines(
    snapshot: &super::snapshots::RevisionSnapshot,
    file: &Path,
    lines: [usize; 2],
) -> Result<String, String> {
    let source = snapshot
        .sources
        .get(file)
        .ok_or_else(|| "case file was not captured".to_string())?
        .as_ref()
        .map_err(Clone::clone)?;
    let collected = source.split_inclusive('\n').collect::<Vec<_>>();
    if lines[0] == 0 || lines[0] > lines[1] || lines[1] > collected.len() {
        return Err("case lines are outside archived source".into());
    }
    Ok(collected[lines[0] - 1..lines[1]].concat())
}
pub(super) fn score(
    case: &LintCase,
    snapshot: &super::snapshots::RevisionSnapshot,
    report: &LintReport,
) -> (CaseOutcome, Option<usize>) {
    match &case.subject {
        CaseSubject::Rule {
            rule,
            file,
            lines,
            text_sha256,
            expected,
        } => {
            let text = match archived_lines(snapshot, file, *lines) {
                Ok(text) => text,
                Err(error) => return (incomplete(error), None),
            };
            if sunstack_evidence::content_digest(text.as_bytes()) != *text_sha256 {
                return (CaseOutcome::Stale, None);
            }
            let matches = report
                .results
                .iter()
                .filter(|result| {
                    &result.name == rule
                        && &result.file == file
                        && result.first_line == lines[0]
                        && result.last_line == lines[1]
                })
                .collect::<Vec<_>>();
            let result = match matches.as_slice() {
                [] => {
                    if let Some(skip) = report
                        .filter_skips
                        .iter()
                        .find(|skip| &skip.rule == rule && &skip.file == file)
                    {
                        let exact_match_count = skip
                            .locations
                            .iter()
                            .filter(|location| {
                                location.first_line == lines[0] && location.last_line == lines[1]
                            })
                            .count();
                        if exact_match_count == 0 {
                            return (CaseOutcome::NoMatchingPiece, None);
                        }
                        if exact_match_count > 1 {
                            return (CaseOutcome::MoreThanOneMatchingPiece, None);
                        }
                        return (
                            match skip.why {
                                FilterSkipReason::Unsure => CaseOutcome::Unsure,
                                _ => incomplete("filter did not produce a label"),
                            },
                            Some(skip.id),
                        );
                    }
                    return (CaseOutcome::NoMatchingPiece, None);
                }
                [result] => *result,
                _ => return (CaseOutcome::MoreThanOneMatchingPiece, None),
            };
            let outcome = match (&result.ending, expected) {
                (RulePieceEnding::NotApplicable { reason: Some(_) }, _) => {
                    CaseOutcome::FilterExcluded
                }
                (RulePieceEnding::Finding, ExpectedRuleOutcome::Warn)
                | (RulePieceEnding::Pass, ExpectedRuleOutcome::Pass) => CaseOutcome::Right,
                (RulePieceEnding::Finding, ExpectedRuleOutcome::Pass) => CaseOutcome::FalseWarning,
                (
                    RulePieceEnding::Pass | RulePieceEnding::NotApplicable { .. },
                    ExpectedRuleOutcome::Warn,
                ) => CaseOutcome::Missed,
                (RulePieceEnding::NotApplicable { .. }, ExpectedRuleOutcome::Pass) => {
                    CaseOutcome::Right
                }
                (RulePieceEnding::Uncertain, _) => CaseOutcome::Unsure,
                (RulePieceEnding::Incomplete { gap }, _) => incomplete(format!("{gap:?}")),
                (RulePieceEnding::Unavailable { reason }, _) => CaseOutcome::Unavailable {
                    reason: reason.clone(),
                },
            };
            (outcome, Some(result.id))
        }
        CaseSubject::Filter {
            filter,
            folder,
            reads_sha256,
            expected_label,
        } => {
            let Some(result) = report
                .filter_answers
                .iter()
                .find(|result| &result.filter == filter && &result.folder == folder)
            else {
                return (incomplete("filter folder was not evaluated"), None);
            };
            let outcome = if result
                .reads_sha256
                .as_ref()
                .is_some_and(|actual| actual != reads_sha256)
            {
                CaseOutcome::Stale
            } else {
                match &result.state {
                    FilterAnswerState::Judged { label, .. } if label == expected_label => {
                        CaseOutcome::Right
                    }
                    FilterAnswerState::Judged { .. } => CaseOutcome::WrongLabel,
                    FilterAnswerState::Unsure { .. } => CaseOutcome::Unsure,
                    FilterAnswerState::Unavailable { reason, .. } => CaseOutcome::Unavailable {
                        reason: reason.clone(),
                    },
                    FilterAnswerState::Incomplete { reason, .. } => {
                        incomplete(format!("{reason:?}"))
                    }
                }
            };
            (outcome, Some(result.id))
        }
    }
}

#[cfg(test)]
#[path = "matching_test.rs"]
mod tests;

pub(super) fn prepare(
    case: &LintCase,
    snapshot: &super::snapshots::RevisionSnapshot,
    report: &LintReport,
    dependency: Option<&(sunstack_evidence::EvidenceKindName, std::path::PathBuf)>,
) -> (CaseOutcome, Option<usize>) {
    use crate::{RequestOwner, SavedRequestOutcome};
    if let CaseSubject::Rule {
        file,
        lines,
        text_sha256,
        ..
    } = &case.subject
    {
        let text = match archived_lines(snapshot, file, *lines) {
            Ok(text) => text,
            Err(error) => return (incomplete(error), None),
        };
        if sunstack_evidence::content_digest(text.as_bytes()) != *text_sha256 {
            return (CaseOutcome::Stale, None);
        }
    }
    let prepared = report
        .requests
        .iter()
        .filter(|request| matches!(request.outcome, SavedRequestOutcome::Prepared { .. }))
        .filter(|request| match (&case.subject, &request.owner) {
            (
                CaseSubject::Rule {
                    rule, file, lines, ..
                },
                RequestOwner::Rule {
                    rule: actual,
                    piece,
                },
            ) => {
                rule == actual
                    && file == &piece.file
                    && lines[0] == piece.first_line
                    && lines[1] == piece.last_line
            }
            (
                CaseSubject::Filter { filter, folder, .. },
                RequestOwner::Filter {
                    filter: actual,
                    folder: actual_folder,
                },
            ) => filter == actual && folder == actual_folder,
            _ => false,
        })
        .collect::<Vec<_>>();
    match prepared.as_slice() {
        [request] => {
            if let CaseSubject::Filter { reads_sha256, .. } = &case.subject {
                let Some(evidence) = report.piece_evidence.get(&request.id) else {
                    return (incomplete("prepared filter evidence is missing"), None);
                };
                match sunstack_rules::filter_reads_hash(&evidence.piece.fields) {
                    Ok(hash) if &hash == reads_sha256 => {}
                    Ok(_) => return (CaseOutcome::Stale, None),
                    Err(reason) => return (incomplete(format!("{reason:?}")), None),
                }
            }
            return (CaseOutcome::Prepared, Some(request.id));
        }
        [] => {}
        _ => return (CaseOutcome::MoreThanOneMatchingPiece, None),
    }
    let (outcome, id) = score(case, snapshot, report);
    if id.is_none()
        && !matches!(outcome, CaseOutcome::MoreThanOneMatchingPiece)
        && let Some((kind, file)) = dependency
        && let Some(wait) = report.preparation.as_ref().and_then(|preparation| {
            preparation
                .needs_answer
                .iter()
                .find(|entry| &entry.kind == kind && &entry.file == file)
        })
    {
        return (
            CaseOutcome::NotJudged {
                reason: format!(
                    "needs_answer: {} request {}",
                    wait.waits_on.selection_kind.as_str(),
                    wait.waits_on.request_id
                ),
            },
            None,
        );
    }
    let outcome = match outcome {
        CaseOutcome::Right
        | CaseOutcome::FalseWarning
        | CaseOutcome::Missed
        | CaseOutcome::WrongLabel => CaseOutcome::Prepared,
        CaseOutcome::Unsure => CaseOutcome::NotJudged {
            reason: "mechanical route is uncertain".into(),
        },
        other => other,
    };
    (outcome, id)
}

// Execution owns identity admission; scoring must not turn a refused binding into a match.
pub(super) fn selection_outcome(
    case: &LintCase,
    report: &LintReport,
) -> Option<(CaseOutcome, Option<usize>)> {
    let Some(selection) = report.case_selection.as_ref() else {
        return Some((incomplete("case selection accounting is unavailable"), None));
    };
    let Some(binding) = selection
        .bindings
        .iter()
        .find(|entry| entry.binding.id == case.id)
    else {
        return Some((
            incomplete("case binding is missing from selection accounting"),
            None,
        ));
    };
    let outcome = match &binding.outcome {
        crate::CaseBindingStatus::Selected { .. } => return None,
        crate::CaseBindingStatus::Unmatched => CaseOutcome::NoMatchingPiece,
        crate::CaseBindingStatus::Ambiguous { .. } => CaseOutcome::MoreThanOneMatchingPiece,
        crate::CaseBindingStatus::Stale => CaseOutcome::Stale,
        crate::CaseBindingStatus::SelectionPending {
            evidence_kind,
            file,
        } => CaseOutcome::SelectionPending {
            evidence_kind: evidence_kind.clone(),
            file: file.clone(),
        },
        crate::CaseBindingStatus::Incomplete { reason } => incomplete(format!("{reason:?}")),
        crate::CaseBindingStatus::Unavailable { reason } => CaseOutcome::Unavailable {
            reason: reason.clone(),
        },
        crate::CaseBindingStatus::Cancelled => CaseOutcome::Cancelled,
    };
    let result_id = if matches!(
        outcome,
        CaseOutcome::Incomplete { .. }
            | CaseOutcome::Unavailable { .. }
            | CaseOutcome::Cancelled
            | CaseOutcome::SelectionPending { .. }
    ) {
        unique_result_id(case, report)
    } else {
        None
    };
    Some((outcome, result_id))
}

// A settled execution status can retain evidence, but only an actual unique result can supply its ID.
fn unique_result_id(case: &LintCase, report: &LintReport) -> Option<usize> {
    let mut matching_ids = match &case.subject {
        CaseSubject::Rule {
            rule, file, lines, ..
        } => report
            .results
            .iter()
            .filter(|result| {
                &result.name == rule
                    && &result.file == file
                    && [result.first_line, result.last_line] == *lines
            })
            .map(|result| result.id)
            .collect::<Vec<_>>(),
        CaseSubject::Filter { filter, folder, .. } => report
            .filter_answers
            .iter()
            .filter(|result| &result.filter == filter && &result.folder == folder)
            .map(|result| result.id)
            .collect::<Vec<_>>(),
    }
    .into_iter();
    let result_id = matching_ids.next()?;
    matching_ids.next().is_none().then_some(result_id)
}

pub(super) fn selected_result(
    case: &LintCase,
    snapshot: &super::snapshots::RevisionSnapshot,
    report: &LintReport,
    dependency: Option<&(sunstack_evidence::EvidenceKindName, std::path::PathBuf)>,
    preparing: bool,
) -> (CaseOutcome, Option<usize>) {
    let (outcome, id) = if preparing {
        prepare(case, snapshot, report, dependency)
    } else {
        score(case, snapshot, report)
    };
    // Binding uniqueness survives cancellation; an unfinished execution is not an unmatched case.
    let unfinished = matches!(
        outcome,
        CaseOutcome::Incomplete { .. } | CaseOutcome::NoMatchingPiece
    ) || (id.is_none() && matches!(outcome, CaseOutcome::NotJudged { .. }));
    if report.status == crate::LintStatus::Cancelled && unfinished {
        (CaseOutcome::Cancelled, id)
    } else {
        (outcome, id)
    }
}
