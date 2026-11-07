use super::contracts::*;
use crate::{LintReport, PieceOutcome};
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
            let outcome = if result.reason.is_some() {
                CaseOutcome::FilterExcluded
            } else {
                match (&result.outcome, expected) {
                    (PieceOutcome::Finding, ExpectedRuleOutcome::Warn)
                    | (PieceOutcome::Pass, ExpectedRuleOutcome::Pass) => CaseOutcome::Right,
                    (PieceOutcome::Finding, ExpectedRuleOutcome::Pass) => CaseOutcome::FalseWarning,
                    (
                        PieceOutcome::Pass | PieceOutcome::NotApplicable,
                        ExpectedRuleOutcome::Warn,
                    ) => CaseOutcome::Missed,
                    (PieceOutcome::NotApplicable, ExpectedRuleOutcome::Pass) => CaseOutcome::Right,
                    (PieceOutcome::Uncertain, _) => CaseOutcome::Unsure,
                    (PieceOutcome::Incomplete, _) => incomplete(format!("{:?}", result.gap)),
                }
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
