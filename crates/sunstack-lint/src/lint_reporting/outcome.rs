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
        report.results.push(RulePieceResult {
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
                sunstack_evidence::ModelView::TooLarge { .. } => vec![],
            },
        });
        match pair.result {
            RulebookResult::Pass { .. } => {}
            RulebookResult::NotApplicable { .. } => {
                report.exclusions.not_applicable =
                    report.exclusions.not_applicable.saturating_add(1)
            }
            RulebookResult::Finding {
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
                    reason: match reason {
                        RulebookIncompleteReason::EvidenceUnavailable => {
                            IncompleteReason::EvidenceUnavailable
                        }
                        RulebookIncompleteReason::EvidenceTooLarge => {
                            IncompleteReason::EvidenceTooLarge
                        }
                        RulebookIncompleteReason::RequestTooLarge => {
                            IncompleteReason::RequestTooLarge
                        }
                        RulebookIncompleteReason::ProviderFailure {
                            failure_class,
                            http_status,
                        } => IncompleteReason::ProviderFailure {
                            failure_class,
                            http_status,
                        },
                        RulebookIncompleteReason::InvalidRequest => {
                            IncompleteReason::InvalidRequest
                        }
                        RulebookIncompleteReason::InvalidResponse => {
                            IncompleteReason::InvalidResponse
                        }
                        RulebookIncompleteReason::RequestLimitReached => {
                            IncompleteReason::RequestLimitReached
                        }
                        RulebookIncompleteReason::Cancelled => IncompleteReason::NotStarted,
                    },
                })
            }
        }
    }
    report.status = if cancelled {
        LintStatus::Cancelled
    } else if !report.incomplete.is_empty() {
        LintStatus::Incomplete
    } else if report.counts.error > 0
        || max_warnings.is_some_and(|maximum| report.counts.warn > maximum)
    {
        LintStatus::Findings
    } else if !report.uncertain.is_empty()
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

fn related_places(rule: &RulebookRule, piece: &EvidencePiece) -> Vec<RelatedPlace> {
    use sunstack_evidence::{FieldValue, SourceRef};
    let mut places = vec![RelatedPlace {
        label: "standard".into(),
        file: rule.sentence.rulebook.clone(),
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
