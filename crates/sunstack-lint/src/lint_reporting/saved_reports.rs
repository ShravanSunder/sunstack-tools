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
    let input_sha256 = matches!(
        &outcome,
        SavedRequestOutcome::NotSent {
            reason: RequestNotSentReason::Rule {
                reason: sunstack_rules::RulebookIncompleteReason::AdmissionRefused { .. }
            } | RequestNotSentReason::Filter {
                reason: sunstack_rules::FilterIncomplete::Request {
                    reason: sunstack_rules::RulebookIncompleteReason::AdmissionRefused { .. }
                }
            }
        }
    )
    .then(|| {
        serde_json::to_vec(&request)
            .ok()
            .map(|bytes| sunstack_evidence::content_digest(&bytes))
    })
    .flatten();
    report.requests.push(SavedRequest {
        id,
        owner,
        requested_model: model.clone(),
        resolved_model,
        input_sha256,
        selection_evidence: None,
        request,
        outcome,
    });
    id
}
pub(crate) fn capture_prepared_requests(
    report: &mut LintReport,
    dispatcher: &sunstack_rules::ModelRequestDispatch,
    run: &[sunstack_rules::PreparedRulebookRequest],
    filters: &FilterAnswers,
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
) -> Result<(), String> {
    let BackendSelection::JevOpenrouter { model, .. } = &dispatcher.settings.backend;
    // Prepared IDs share the show namespace with actual mechanical results.
    let offset = report.results.len()
        + report.filter_answers.len()
        + report.filter_skips.len()
        + report.requests.len()
        + 1;
    for record in &dispatcher.prepared {
        let rule = run
            .iter()
            .find(|pair| pair.request_id == record.id)
            .and_then(|pair| Some((rules.get(pair.rule_index)?, pieces.get(pair.piece_index)?)));
        let filter = filters
            .prepared
            .iter()
            .find(|item| item.request_id == record.id);
        let selection = dispatcher
            .selection_requests
            .iter()
            .find(|item| item.request_id == record.id);
        let (owner, gate, evidence) = if let Some((rule, piece)) = rule {
            (
                RequestOwner::Rule {
                    rule: rule.name.clone(),
                    piece: piece.location.clone(),
                },
                rule.gate.as_ref().map(|gate| PreparedGate {
                    filter: gate.filter.clone(),
                    labels: gate.answers.clone(),
                }),
                Some(piece.clone()),
            )
        } else if let Some(filter) = filter {
            (
                RequestOwner::Filter {
                    filter: filter.filter.clone(),
                    folder: filter.folder.clone(),
                },
                None,
                Some(filter.piece.clone()),
            )
        } else if let Some(selection) = selection {
            (
                RequestOwner::Selection {
                    selection_kind: selection.selection_kind.clone(),
                    file: selection.file.clone(),
                },
                None,
                None,
            )
        } else {
            return Err("prepared request has no owner".into());
        };
        let id = offset + record.id;
        if let Some(piece) = evidence {
            report.piece_evidence.insert(
                id,
                SavedPieceEvidence {
                    piece,
                    matches: Default::default(),
                },
            );
        }
        let reused = dispatcher
            .reused_selections
            .iter()
            .find(|item| item.request_id == record.id);
        let resolved_model = reused.and_then(|record| match &record.outcome {
            DecisionRunOutcome::Completed { receipt, .. } => Some(receipt.resolved_model.clone()),
            _ => None,
        });
        let outcome = reused.map_or_else(
            || SavedRequestOutcome::Prepared {
                status: PreparedRequestStatus::Prepared,
                gate,
            },
            |record| SavedRequestOutcome::FromReport {
                status: RecordedRequestStatus::FromReport,
                source_request_id: record.source_request_id,
                source_revision: record.source_revision.clone(),
                outcome: record.outcome.clone(),
            },
        );
        report.requests.push(SavedRequest {
            id,
            owner,
            requested_model: model.clone(),
            resolved_model,
            input_sha256: Some(sunstack_evidence::content_digest(
                &serde_json::to_vec(&record.request).map_err(|error| error.to_string())?,
            )),
            selection_evidence: selection.map(|selection| SelectionRequestEvidence {
                candidates: selection.candidates.clone(),
                shared: selection.shared.clone(),
            }),
            request: record.request.clone(),
            outcome,
        });
    }
    if let Some(preparation) = &mut report.preparation {
        for dependency in &mut preparation.needs_answer {
            dependency.waits_on.request_id += offset;
        }
    }
    for entry in &mut report.selections {
        if let sunstack_rules::SelectionOutcome::Prepared { request_id } = &mut entry.outcome {
            *request_id += offset;
        }
    }
    Ok(())
}
pub(crate) fn capture_selection_requests(
    report: &mut LintReport,
    dispatcher: &sunstack_rules::ModelRequestDispatch,
) -> Result<(), String> {
    let BackendSelection::JevOpenrouter { model, .. } = &dispatcher.settings.backend;
    let offset = report.results.len()
        + report.filter_answers.len()
        + report.filter_skips.len()
        + report.requests.len()
        + 1;
    for (index, owner) in dispatcher.selection_requests.iter().enumerate() {
        let Some(record) = dispatcher
            .records
            .iter()
            .find(|record| record.id == owner.request_id)
        else {
            continue;
        };
        let resolved_model = match &record.outcome {
            DecisionRunOutcome::Completed { receipt, .. } => Some(receipt.resolved_model.clone()),
            _ => None,
        };
        report.requests.push(SavedRequest {
            id: offset + index,
            owner: RequestOwner::Selection {
                selection_kind: owner.selection_kind.clone(),
                file: owner.file.clone(),
            },
            requested_model: model.clone(),
            resolved_model,
            input_sha256: Some(sunstack_evidence::content_digest(
                &serde_json::to_vec(&record.request).map_err(|error| error.to_string())?,
            )),
            selection_evidence: Some(SelectionRequestEvidence {
                candidates: owner.candidates.clone(),
                shared: owner.shared.clone(),
            }),
            request: record.request.clone(),
            outcome: SavedRequestOutcome::Run {
                outcome: record.outcome.clone(),
            },
        });
    }
    Ok(())
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
                        reason: record.reason.clone(),
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
enum SavedReportKind {
    Lint,
    PreparedCases,
    CompletedCases,
}
impl SavedReportKind {
    fn human_name(&self) -> &'static str {
        match self {
            Self::Lint => "lint report",
            Self::PreparedCases => "prepared cases report",
            Self::CompletedCases => "completed cases report",
        }
    }
}

/// Read an existing lint or cases report without config, source files or credentials.
pub fn show_saved_report(bytes: &[u8], selector: &str) -> Result<serde_json::Value, String> {
    let document: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "malformed saved report JSON".to_string())?;
    // These reports predate a kind tag. Classify their existing top-level sections
    // before deserializing so a cases report never leaks a lint parser's error.
    let kind = if document.get("cases").is_some() || document.get("runs").is_some() {
        if document.get("mode").is_some() {
            SavedReportKind::PreparedCases
        } else {
            SavedReportKind::CompletedCases
        }
    } else if document.get("results").is_some()
        || (document.get("config").is_some() && document.get("status").is_some())
    {
        SavedReportKind::Lint
    } else {
        return Err("unsupported saved report kind; expected a lint report or cases report".into());
    };
    let name = kind.human_name();
    match kind {
        SavedReportKind::Lint => {
            let report: LintReport = serde_json::from_value(document).map_err(|_| {
                format!("malformed {name}; saved report fields are invalid or missing")
            })?;
            let id = selector
                .parse::<usize>()
                .map_err(|_| format!("{name} requires a numeric result id; got {selector}"))?;
            show(&report, id).map_err(|reason| format!("{name}: {reason}"))
        }
        SavedReportKind::PreparedCases | SavedReportKind::CompletedCases => {
            let report: crate::case_runs::CasesReport =
                serde_json::from_value(document).map_err(|_| {
                    format!("malformed {name}; saved report fields are invalid or missing")
                })?;
            crate::case_runs::show_case(&report, selector)
                .map_err(|reason| format!("{name}: {reason}"))
        }
    }
}

/// Read one result using only the saved file, without config discovery, credentials or network.
pub fn show(report: &LintReport, id: usize) -> Result<serde_json::Value, String> {
    if report.preparation.is_some()
        && let Some(request) = report.requests.iter().find(|request| {
            request.id == id
                && matches!(
                    request.outcome,
                    SavedRequestOutcome::Prepared { .. } | SavedRequestOutcome::FromReport { .. }
                )
        })
    {
        return Ok(
            serde_json::json!({"request": request, "evidence": report.piece_evidence.get(&id), "rulebook_files": report.rulebook_files}),
        );
    }
    let (result, requests, finding, rewalkable, gap) =
        if let Some(result) = report.results.iter().find(|result| result.id == id) {
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
                excluded.then_some(
                    "rule excluded by filter; rule was never asked and cannot be re-walked",
                ),
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
        } else if let Some(request) = report.requests.iter().find(|request| {
            request.id == id && matches!(request.owner, RequestOwner::Selection { .. })
        }) {
            return Ok(
                serde_json::json!({"request": request, "rulebook_files": report.rulebook_files}),
            );
        } else {
            let count =
                report.results.len() + report.filter_answers.len() + report.filter_skips.len();
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
