use crate::*;
use sunstack_decisions::{AttemptEvidence, DecisionRunOutcome};
use sunstack_evidence::EvidencePiece;
use sunstack_rules::{RulebookIncompleteReason, RulebookResult, RulebookRule, RulebookRun};

pub(crate) fn project_outcome(
    run: RulebookRun,
    rules: &[RulebookRule],
    pieces: &[EvidencePiece],
    mut report: LintReport,
    max_warnings: Option<u32>,
    cancelled: bool,
) -> LintReport {
    let selected = run.results.len();
    for answer in &report.filter_answers {
        if let sunstack_rules::FilterAnswerState::Incomplete { reason, .. } = &answer.state {
            report.incomplete.push(IncompletePart {
                rule: None,
                file: Some(answer.folder.clone()),
                field: None,
                reason: IncompleteReason::Filter {
                    filter: answer.filter.clone(),
                    reason: reason.clone(),
                },
            });
        }
    }
    for skip in &run.skips {
        if matches!(skip.why, sunstack_rules::FilterSkipReason::NoFolder) {
            report.incomplete.push(IncompletePart {
                rule: Some(skip.rule.clone()),
                file: Some(skip.file.clone()),
                field: None,
                reason: IncompleteReason::NoFilterFolder {
                    filter: skip.filter.clone(),
                },
            });
        }
    }
    report.filter_skips = run
        .skips
        .iter()
        .cloned()
        .map(|value| IdentifiedResult {
            id: 0,
            request_ids: vec![],
            value,
        })
        .collect();
    let acquired = report.usage.checked_files_acquired;
    report.usage = summarize_usage(&run);
    report.usage.checked_files_acquired = acquired;
    for record in &run.requests {
        if was_dispatched(&record.outcome) {
            report
                .evidence_seen
                .insert(record.pair_index, record.request.state.clone());
        }
    }
    for pair in run.results {
        let (Some(rule), Some(piece)) = (rules.get(pair.rule_index), pieces.get(pair.piece_index))
        else {
            continue;
        };
        let (outcome, step) = match &pair.result {
            RulebookResult::Pass { path_taken, .. } => {
                (PieceOutcome::Pass, path_taken.last().cloned())
            }
            RulebookResult::NotApplicable { path_taken, .. } => {
                (PieceOutcome::NotApplicable, path_taken.last().cloned())
            }
            RulebookResult::Finding { path_taken, .. } => {
                (PieceOutcome::Finding, path_taken.last().cloned())
            }
            RulebookResult::Uncertain { step, .. } => (PieceOutcome::Uncertain, Some(step.clone())),
            RulebookResult::Incomplete { step, .. } => (PieceOutcome::Incomplete, step.clone()),
        };
        let basis = pair.result.basis();
        let id = report.results.len() + 1;
        report.piece_evidence.insert(
            id,
            SavedPieceEvidence {
                piece: piece.clone(),
                matches: Default::default(),
            },
        );
        let request_ids = report
            .requests
            .iter()
            .filter_map(|record| match &record.owner {
                RequestOwner::Rule {
                    rule: name,
                    piece: location,
                } if name == &rule.name && location == &piece.location => Some(record.id),
                RequestOwner::Filter { filter, folder }
                    if rule
                        .gate
                        .as_ref()
                        .is_some_and(|gate| &gate.filter == filter)
                        && piece.location.file.starts_with(folder) =>
                {
                    Some(record.id)
                }
                _ => None,
            })
            .collect();
        report.results.push(RulePieceResult {
            text_sha256: None,
            id,
            request_ids,
            gap: match &pair.result {
                RulebookResult::Incomplete { reason, .. } => Some(*reason),
                _ => None,
            },
            locator_omitted: match &pair.result {
                RulebookResult::Finding {
                    locator_omitted, ..
                } => locator_omitted.clone(),
                _ => None,
            },
            reason: match &pair.result {
                RulebookResult::NotApplicable {
                    reason: reason @ sunstack_rules::NotApplicableReason::Filter { .. },
                    ..
                } => Some(reason.clone()),
                _ => None,
            },
            name: rule.name.clone(),
            marker: rule.sentence.marker.clone(),
            file: piece.location.file.clone(),
            first_line: piece.location.first_line,
            last_line: piece.location.last_line,
            outcome,
            step,
            path: pair.result.path_taken().to_vec(),
            answers: pair.result.answers_read().clone(),
            basis,
            cut: match &piece.model_view {
                sunstack_evidence::ModelView::Bounded { cut, .. } => cut.clone(),
                sunstack_evidence::ModelView::TooLarge { .. }
                | sunstack_evidence::ModelView::SelectionPending => vec![],
            },
        });
        match pair.result {
            RulebookResult::Pass { .. } => {}
            RulebookResult::NotApplicable { .. } => {
                report.exclusions.not_applicable =
                    report.exclusions.not_applicable.saturating_add(1)
            }
            RulebookResult::Finding {
                locator_omitted,
                severity,
                message,
                line,
                field,
                path_taken,
                answers,
                value,
                anchor,
                ..
            } => {
                let Some(basis) = basis else {
                    continue;
                };
                match severity {
                    sunstack_rules::Severity::Error => {
                        report.counts.error = report.counts.error.saturating_add(1)
                    }
                    sunstack_rules::Severity::Warn => {
                        report.counts.warn = report.counts.warn.saturating_add(1)
                    }
                }
                let span = field
                    .as_ref()
                    .and_then(|field| piece.fields.get(field))
                    .and_then(|field| field.spans().first())
                    .cloned()
                    .unwrap_or_else(|| piece.location.span.clone());
                report.findings.push(Finding {
                    id,
                    locator_omitted,
                    rule: rule.name.clone(),
                    marker: rule.sentence.marker.clone(),
                    file: piece.location.file.clone(),
                    line,
                    field,
                    span: Some(span),
                    severity,
                    severity_origin: SettingOrigin::Authored,
                    message,
                    judgment: answers.map(|answers| JudgmentSummary {
                        answers,
                        value,
                        path_taken,
                    }),
                    basis,
                    related: related_places(rule, piece),
                    anchor,
                });
            }
            RulebookResult::Uncertain {
                step,
                state,
                answers,
                ..
            } => report.uncertain.push(UncertainResult {
                id,
                rule: rule.name.clone(),
                marker: rule.sentence.marker.clone(),
                location: piece.location.clone(),
                step,
                state,
                answers,
            }),
            RulebookResult::Incomplete { reason, field, .. } => {
                report.incomplete.push(IncompletePart {
                    rule: Some(rule.name.clone()),
                    file: Some(piece.location.file.clone()),
                    field,
                    reason: rulebook_incomplete_reason(reason),
                })
            }
        }
    }
    for (offset, answer) in report.filter_answers.iter_mut().enumerate() {
        answer.id = report.results.len() + offset + 1;
        answer.request_ids = report
            .requests
            .iter()
            .filter_map(|record| match &record.owner {
                RequestOwner::Filter { filter, folder }
                    if filter == &answer.filter && folder == &answer.folder =>
                {
                    Some(record.id)
                }
                _ => None,
            })
            .collect();
    }
    for (offset, skip) in report.filter_skips.iter_mut().enumerate() {
        skip.id = report.results.len() + report.filter_answers.len() + offset + 1;
        skip.request_ids = report
            .requests
            .iter()
            .filter_map(|record| match &record.owner {
                RequestOwner::Filter { filter, folder }
                    if filter == &skip.filter && Some(folder) == skip.folder.as_ref() =>
                {
                    Some(record.id)
                }
                _ => None,
            })
            .collect();
    }
    report.status = if cancelled {
        LintStatus::Cancelled
    } else if !report.incomplete.is_empty() {
        LintStatus::Incomplete
    } else if report.counts.error > 0
        || max_warnings.is_some_and(|maximum| report.counts.warn > maximum)
    {
        LintStatus::Findings
    } else if report.filter_answers.iter().any(|answer| {
        matches!(
            answer.state,
            sunstack_rules::FilterAnswerState::Unsure { .. }
        )
    }) || !report.uncertain.is_empty()
        || report.selections.iter().any(|entry| {
            matches!(
                entry.outcome,
                sunstack_rules::SelectionOutcome::Uncertain { .. }
            )
        })
    {
        LintStatus::Uncertain
    } else if selected == 0 {
        LintStatus::NothingSelected
    } else {
        LintStatus::Clean
    };
    report
}

pub(crate) fn project_selections(
    catalog: &mut sunstack_evidence::EvidenceCatalog,
    report: &mut LintReport,
) {
    report.selections.sort_by(|left, right| {
        (
            &left.location.file,
            &left.kind,
            left.location.span.byte_start,
        )
            .cmp(&(
                &right.location.file,
                &right.kind,
                right.location.span.byte_start,
            ))
    });
    for entry in &report.selections {
        if let Some(count) = catalog
            .file_counts
            .iter_mut()
            .find(|count| count.file == entry.location.file)
            .and_then(|count| count.kinds.get_mut(&entry.kind))
            .and_then(|count| count.selection.as_mut())
        {
            count.asked += 1;
            match entry.outcome {
                sunstack_rules::SelectionOutcome::Prepared { .. }
                | sunstack_rules::SelectionOutcome::Deferred => {
                    count.asked -= 1;
                }
                sunstack_rules::SelectionOutcome::Selected { .. } => count.selected += 1,
                sunstack_rules::SelectionOutcome::NotSelected { .. } => count.not_selected += 1,
                sunstack_rules::SelectionOutcome::Uncertain { .. } => count.uncertain += 1,
                sunstack_rules::SelectionOutcome::Incomplete { .. } => count.incomplete += 1,
            }
        }
    }
    if report.selections.iter().any(|entry| {
        matches!(
            entry.outcome,
            sunstack_rules::SelectionOutcome::Incomplete { .. }
        )
    }) {
        report.incomplete.push(IncompletePart {
            rule: None,
            file: None,
            field: None,
            reason: IncompleteReason::SelectionIncomplete,
        });
    }
}

fn related_places(rule: &RulebookRule, piece: &EvidencePiece) -> Vec<RelatedPlace> {
    use sunstack_evidence::{FieldValue, SourceRef};
    let mut places = vec![RelatedPlace {
        definition: Some(rule.sentence.rulebook.clone()),
        label: "standard".into(),
        file: rule.sentence.rulebook.cli_path(),
        first_line: rule.sentence.line,
        last_line: rule.sentence.line,
    }];
    for name in &rule.related {
        let Some(field) = piece.fields.get(name) else {
            continue;
        };
        if let FieldValue::Targets { values } = field {
            places.extend(values.iter().filter_map(|target| {
                target.opening_last_line.map(|last_line| RelatedPlace {
                    definition: None,
                    label: name.as_str().into(),
                    file: target.path.clone(),
                    first_line: 1,
                    last_line,
                })
            }));
        } else {
            places.extend(field.spans().iter().filter_map(|span| {
                let file = match &span.source {
                    SourceRef::Document { path, .. }
                    | SourceRef::RepositoryFile { path, .. }
                    | SourceRef::Transcript { path, .. } => path.clone(),
                    SourceRef::Payload { .. } => return None,
                };
                Some(RelatedPlace {
                    definition: None,
                    label: name.as_str().into(),
                    file,
                    first_line: span.start_line,
                    last_line: if span.end_column == 1 && span.end_line > span.start_line {
                        span.end_line - 1
                    } else {
                        span.end_line
                    },
                })
            }));
        }
    }
    places
}

fn was_dispatched(outcome: &DecisionRunOutcome) -> bool {
    let attempts = match outcome {
        DecisionRunOutcome::Completed { attempts, .. }
        | DecisionRunOutcome::ProviderFailure { attempts, .. }
        | DecisionRunOutcome::InvalidResponse { attempts, .. }
        | DecisionRunOutcome::Cancelled { attempts, .. } => attempts.as_slice(),
        DecisionRunOutcome::InvalidRequest { .. } => &[],
    };
    attempts
        .iter()
        .any(|attempt| !matches!(attempt.evidence, AttemptEvidence::NotDispatched { .. }))
}
pub fn summarize_usage(run: &RulebookRun) -> UsageSummary {
    let mut usage = UsageSummary::default();
    for record in &run.requests {
        if was_dispatched(&record.outcome) {
            usage.model_requests = usage.model_requests.saturating_add(1);
        }
        if let DecisionRunOutcome::Completed { receipt, .. } = &record.outcome {
            usage.input_tokens = Some(
                usage
                    .input_tokens
                    .unwrap_or(0)
                    .saturating_add(receipt.usage.input_tokens),
            );
            usage.output_tokens = Some(
                usage
                    .output_tokens
                    .unwrap_or(0)
                    .saturating_add(receipt.usage.output_tokens),
            );
            if let Some(cost) = receipt.usage.cost_usd {
                usage.cost_usd = Some(usage.cost_usd.unwrap_or(0.0) + cost);
                usage.cost_reported_requests += 1;
            }
        }
    }
    usage.cost_missing_requests = usage
        .model_requests
        .saturating_sub(usage.cost_reported_requests);
    usage
}

#[cfg(test)]
#[path = "outcome_test.rs"]
mod tests;

pub(crate) fn selection_usage(
    dispatcher: &sunstack_rules::ModelRequestDispatch,
    acquired: usize,
) -> UsageSummary {
    let run = sunstack_rules::RulebookRun {
        prepared: vec![],
        not_sent: vec![],
        skips: vec![],
        results: vec![],
        requests: dispatcher
            .records
            .iter()
            .map(|record| sunstack_rules::RulebookRequestRecord {
                pair_index: record.id,
                request: record.request.clone(),
                outcome: record.outcome.clone(),
            })
            .collect(),
    };
    let mut usage = crate::lint_reporting::outcome::summarize_usage(&run);
    usage.checked_files_acquired = acquired;
    usage
}

pub(crate) fn project_evidence_failures(
    catalog: &sunstack_evidence::EvidenceCatalog,
    report: &mut LintReport,
) {
    for failure in &catalog.failures {
        report.diagnostics.push(LintDiagnostic {
            definition: None,
            json_path: vec![],
            file: Some(failure.file.clone()),
            path: None,
            line: None,
            column: None,
            code: LintInvalidCode::Syntax,
            message: failure.reason.clone(),
        });
        report.incomplete.push(IncompletePart {
            rule: None,
            file: Some(failure.file.clone()),
            field: None,
            reason: IncompleteReason::SelectionFailed,
        });
    }
}

pub(crate) fn rulebook_incomplete_reason(reason: RulebookIncompleteReason) -> IncompleteReason {
    match reason {
        RulebookIncompleteReason::ChoiceOptionsExceeded { options, limit } => {
            IncompleteReason::ChoiceOptionsExceeded { options, limit }
        }
        RulebookIncompleteReason::EvidenceUnavailable => IncompleteReason::EvidenceUnavailable,
        RulebookIncompleteReason::EvidenceTooLarge => IncompleteReason::EvidenceTooLarge,
        RulebookIncompleteReason::RequestTooLarge => IncompleteReason::RequestTooLarge,
        RulebookIncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        } => IncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        },
        RulebookIncompleteReason::InvalidRequest => IncompleteReason::InvalidRequest,
        RulebookIncompleteReason::InvalidResponse => IncompleteReason::InvalidResponse,
        RulebookIncompleteReason::RequestLimitReached => IncompleteReason::RequestLimitReached,
        RulebookIncompleteReason::Cancelled => IncompleteReason::NotStarted,
    }
}
