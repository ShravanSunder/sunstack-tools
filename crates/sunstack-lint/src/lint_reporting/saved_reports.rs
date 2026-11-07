use crate::*;
use sunstack_decisions::{BackendSelection, DecisionRunOutcome};
use sunstack_evidence::EvidencePiece;
use sunstack_rules::{FilterAnswers, RulebookRule, RulebookRun};

pub(crate) fn capture_files(paths: &[std::path::PathBuf]) -> std::io::Result<Vec<RulebookFile>> {
    paths
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|path| {
            let content = std::fs::read_to_string(path)?;
            Ok(RulebookFile {
                path: path.clone(),
                sha256: sunstack_evidence::content_digest(content.as_bytes()),
                content,
            })
        })
        .collect()
}
fn push_request(
    report: &mut LintReport,
    owner: RequestOwner,
    model: &sunstack_decisions::ModelName,
    request: sunstack_decisions::StructuredJudgmentRequest,
    outcome: SavedRequestOutcome,
) -> usize {
    let id = report.requests.len() + 1;
    let resolved_model = match &outcome {
        SavedRequestOutcome::Run {
            outcome: DecisionRunOutcome::Completed { receipt, .. },
        } => Some(receipt.resolved_model.clone()),
        _ => None,
    };
    report.requests.push(SavedRequest {
        id,
        owner,
        requested_model: model.clone(),
        resolved_model,
        request,
        outcome,
    });
    id
}
/// Capture complete requests before projection reduces answers to those the walk read.
pub(crate) fn capture_requests(
    report: &mut LintReport,
    run: &RulebookRun,
    filters: &FilterAnswers,
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    backend: &BackendSelection,
) {
    let BackendSelection::JevOpenrouter { model, .. } = backend;
    for record in &filters.requests {
        push_request(
            report,
            RequestOwner::Filter {
                filter: record.filter.clone(),
                folder: record.folder.clone(),
            },
            model,
            record.request.clone(),
            SavedRequestOutcome::Run {
                outcome: record.outcome.clone(),
            },
        );
    }
    for record in &filters.not_sent {
        push_request(
            report,
            RequestOwner::Filter {
                filter: record.filter.clone(),
                folder: record.folder.clone(),
            },
            model,
            record.request.clone(),
            SavedRequestOutcome::NotSent {
                reason: RequestNotSentReason::Filter {
                    reason: record.reason.clone(),
                },
            },
        );
    }
    for (pair_index, request, outcome) in run
        .requests
        .iter()
        .map(|record| {
            (
                record.pair_index,
                record.request.clone(),
                SavedRequestOutcome::Run {
                    outcome: record.outcome.clone(),
                },
            )
        })
        .chain(run.not_sent.iter().map(|record| {
            (
                record.pair_index,
                record.request.clone(),
                SavedRequestOutcome::NotSent {
                    reason: RequestNotSentReason::Rule {
                        reason: record.reason,
                    },
                },
            )
        }))
    {
        let Some(pair) = run.results.get(pair_index) else {
            continue;
        };
        let (Some(rule), Some(piece)) = (rules.get(pair.rule_index), pieces.get(pair.piece_index))
        else {
            continue;
        };
        push_request(
            report,
            RequestOwner::Rule {
                rule: rule.name.clone(),
                piece: piece.location.clone(),
            },
            model,
            request,
            outcome,
        );
    }
}
/// Read one result using only the saved file, without config discovery, credentials or network.
pub fn show(report: &LintReport, id: usize) -> Result<serde_json::Value, String> {
    let (result, requests, finding, rewalkable, gap) = if let Some(result) =
        report.results.iter().find(|result| result.id == id)
    {
        let excluded = matches!(
            result.reason,
            Some(sunstack_rules::NotApplicableReason::Filter { .. })
        );
        (
            serde_json::to_value(result),
            &result.request_ids,
            report.findings.iter().find(|finding| finding.id == id),
            !excluded
                && (result.gap.is_none()
                    || result.request_ids.iter().any(|id| {
                        report.requests.iter().any(|request| {
                            request.id == *id
                                && matches!(request.owner, RequestOwner::Rule { .. })
                                && matches!(
                                    request.outcome,
                                    SavedRequestOutcome::Run {
                                        outcome:
                                            sunstack_decisions::DecisionRunOutcome::Completed { .. }
                                    }
                                )
                        })
                    })),
            excluded
                .then_some("rule excluded by filter; rule was never asked and cannot be re-walked"),
        )
    } else if let Some(result) = report.filter_answers.iter().find(|result| result.id == id) {
        (
            serde_json::to_value(result),
            &result.request_ids,
            None,
            result.request_ids.iter().any(|id| {
                report.requests.iter().any(|request| {
                    request.id == *id
                        && matches!(
                            request.outcome,
                            SavedRequestOutcome::Run {
                                outcome: sunstack_decisions::DecisionRunOutcome::Completed { .. }
                            }
                        )
                })
            }),
            None,
        )
    } else if let Some(result) = report.filter_skips.iter().find(|result| result.id == id) {
        (
            serde_json::to_value(result),
            &result.request_ids,
            None,
            false,
            Some("rule skipped by filter; rule was never asked and cannot be re-walked"),
        )
    } else {
        let count = report.results.len() + report.filter_answers.len() + report.filter_skips.len();
        return Err(if count == 0 {
            format!("unknown result id {id}; report has no results")
        } else {
            format!("unknown result id {id}; valid range: 1..={count}")
        });
    };
    let requests = report
        .requests
        .iter()
        .filter(|request| requests.contains(&request.id))
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"result":result.map_err(|error| error.to_string())?, "finding":finding, "requests":requests, "evidence": report.piece_evidence.get(&id),
        "rulebook_files":report.rulebook_files, "rewalkable":rewalkable, "gap":gap}),
    )
}
