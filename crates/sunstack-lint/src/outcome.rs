use crate::{config_validation::native_diagnostics, invocation::TargetSettings, *};
use std::collections::BTreeMap;
use sunstack_decisions::DecisionRunOutcome;
use sunstack_evidence::{FrontmatterFieldLocation, MarkdownOccurrence, SourceSpan, TargetLocation};
use sunstack_rules::{
    Activation, GroupKey, InvocationCancellationProgress, JudgmentReference, LocalSettlement,
    MechanicalResult, PolicyEvaluation, QuestionOwner, RequestStatus, RuleEvaluationResult,
    RuleInvocationOutcome, RuleInvocationReport, RuleOccurrenceKey, RuleOccurrenceResult, Severity,
    VerdictResult,
};

fn mechanical_severity(result: MechanicalResult, setting: SeveritySetting) -> Option<Severity> {
    if matches!(result, MechanicalResult::Pass) {
        return None;
    }
    match setting {
        SeveritySetting::Error => Some(Severity::Error),
        SeveritySetting::Warn => Some(Severity::Warn),
        SeveritySetting::Off => None,
    }
}
fn judged_severity(result: &VerdictResult, setting: SeveritySetting) -> Option<Severity> {
    match (result, setting) {
        (VerdictResult::Pass, _) | (_, SeveritySetting::Off) => None,
        (
            VerdictResult::Fail {
                activation: Activation::WarnAboveBand { .. },
            },
            _,
        ) => Some(Severity::Warn),
        (
            VerdictResult::Fail {
                activation: Activation::Error | Activation::Warn,
            },
            SeveritySetting::Warn,
        ) => Some(Severity::Warn),
        (
            VerdictResult::Fail {
                activation: Activation::Error | Activation::Warn,
            },
            SeveritySetting::Error,
        ) => Some(Severity::Error),
    }
}
fn final_status(
    current: LintStatus,
    counts: &SeverityCounts,
    max_warnings: Option<u32>,
    selected: usize,
) -> LintStatus {
    match current {
        LintStatus::Invalid | LintStatus::Cancelled | LintStatus::Incomplete => current,
        LintStatus::Clean | LintStatus::NothingSelected | LintStatus::Findings => {
            if counts.error > 0 || max_warnings.is_some_and(|maximum| counts.warn > maximum) {
                LintStatus::Findings
            } else if selected == 0 {
                LintStatus::NothingSelected
            } else {
                LintStatus::Clean
            }
        }
    }
}
pub(crate) fn target_span(target: &TargetLocation) -> Option<SourceSpan> {
    match target {
        TargetLocation::Paragraph { span, .. } => Some(span.clone()),
        TargetLocation::MarkdownUnit { occurrence } => match occurrence.as_ref() {
            MarkdownOccurrence::Unit { span } => Some(span.clone()),
            MarkdownOccurrence::CallSite { match_span, .. } => Some(match_span.clone()),
        },
        TargetLocation::FrontmatterField { state, .. } => match state.as_ref() {
            FrontmatterFieldLocation::Present { span } => Some(span.clone()),
            FrontmatterFieldLocation::Absent { frontmatter_span } => Some(frontmatter_span.clone()),
            FrontmatterFieldLocation::BlockUnavailable { .. } => None,
        },
        TargetLocation::CodeUnits { .. }
        | TargetLocation::ConversationRecords { .. }
        | TargetLocation::PayloadField { .. }
        | TargetLocation::PositionedRecord { .. } => None,
    }
}
pub fn summarize_usage(report: &RuleInvocationReport) -> UsageSummary {
    let mut usage = UsageSummary {
        model_requests: u32::try_from(report.measurements.model_requests).unwrap_or(u32::MAX),
        ..UsageSummary::default()
    };
    for request in &report.requests {
        if let RequestStatus::Sent {
            outcome: DecisionRunOutcome::Completed { receipt, .. },
        } = &request.status
        {
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
                usage.cost_reported_requests = usage.cost_reported_requests.saturating_add(1);
            } else {
                usage.cost_missing_requests = usage.cost_missing_requests.saturating_add(1);
            }
        }
    }
    usage
}
fn incomplete_reason(
    cause: &sunstack_rules::Cause,
    report: &RuleInvocationReport,
) -> IncompleteReason {
    match cause {
        sunstack_rules::Cause::Request { request_index } => {
            if report.requests.get(*request_index).is_some_and(|request| {
                matches!(
                    request.status,
                    RequestStatus::Sent {
                        outcome: DecisionRunOutcome::ProviderFailure { .. }
                    }
                )
            }) {
                IncompleteReason::ProviderFailure
            } else {
                IncompleteReason::EvaluationFailed
            }
        }
        sunstack_rules::Cause::Budget { .. } => IncompleteReason::RequestLimitReached,
        sunstack_rules::Cause::Evidence { .. }
        | sunstack_rules::Cause::Gate { .. }
        | sunstack_rules::Cause::Condition { .. } => IncompleteReason::EvaluationFailed,
    }
}
fn target_for<'a>(
    key: &RuleOccurrenceKey,
    targets: &'a BTreeMap<GroupKey, TargetSettings>,
) -> Option<&'a TargetSettings> {
    targets.get(&GroupKey {
        rule_set: key.rule_set.clone(),
        group: key.group,
    })
}
fn add_incomplete(
    report: &mut LintReport,
    target: Option<&TargetSettings>,
    rule: Option<&sunstack_rules::RuleId>,
    reason: IncompleteReason,
) {
    report.incomplete.push(IncompletePart {
        rule: target.zip(rule).map(|(target, rule)| RuleSelector::Exact {
            entry: target.entry.clone(),
            rule: rule.clone(),
        }),
        file: target.map(|target| target.file.clone()),
        reason,
    });
}
fn apply_native_report(
    report: &mut LintReport,
    native: &RuleInvocationReport,
    targets: &BTreeMap<GroupKey, TargetSettings>,
) {
    report.usage = summarize_usage(native);
    match native.rule_coverage {
        sunstack_rules::RuleCoverage::Full => {}
        sunstack_rules::RuleCoverage::Partial => {
            add_incomplete(report, None, None, IncompleteReason::CoveragePartial)
        }
        sunstack_rules::RuleCoverage::None => {
            add_incomplete(report, None, None, IncompleteReason::CoverageNone)
        }
    }
    for evaluation in &native.rule_evaluations {
        match evaluation {
            RuleEvaluationResult::Expanded { .. } => {}
            RuleEvaluationResult::ZeroMatches { .. } => {
                report.exclusions.zero_matches = report.exclusions.zero_matches.saturating_add(1)
            }
            RuleEvaluationResult::NotApplicable { .. } => {
                report.exclusions.not_applicable =
                    report.exclusions.not_applicable.saturating_add(1)
            }
            RuleEvaluationResult::OutsideChangeScope { .. } => {
                report.exclusions.outside_change_scope =
                    report.exclusions.outside_change_scope.saturating_add(1)
            }
            RuleEvaluationResult::SelectionFailed { key, .. }
            | RuleEvaluationResult::EvaluationFailed { key, .. }
            | RuleEvaluationResult::Cancelled { key } => {
                let target = targets.get(&GroupKey {
                    rule_set: key.rule_set.clone(),
                    group: key.group,
                });
                let reason = match evaluation {
                    RuleEvaluationResult::SelectionFailed { .. } => {
                        IncompleteReason::SelectionFailed
                    }
                    RuleEvaluationResult::EvaluationFailed { cause, .. } => {
                        incomplete_reason(cause, native)
                    }
                    _ => IncompleteReason::NotStarted,
                };
                add_incomplete(report, target, Some(&key.rule_id), reason);
            }
        }
    }
    for occurrence in &native.occurrences {
        let key = occurrence.key();
        let target = target_for(key, targets);
        let Some(target) = target else {
            add_incomplete(report, None, None, IncompleteReason::EvaluationFailed);
            continue;
        };
        let Some(setting) = target.rules.get(&key.rule_id) else {
            add_incomplete(
                report,
                Some(target),
                Some(&key.rule_id),
                IncompleteReason::EvaluationFailed,
            );
            continue;
        };
        let (severity, span, message, judgment) = match occurrence {
            RuleOccurrenceResult::Checked {
                target: location,
                explanation,
                outcome,
                ..
            } => match outcome {
                LocalSettlement::Completed { result, .. } => (
                    mechanical_severity(*result, setting.setting),
                    target_span(location),
                    explanation.clone(),
                    None,
                ),
                LocalSettlement::Unresolved { .. } => {
                    add_incomplete(
                        report,
                        Some(target),
                        Some(&key.rule_id),
                        IncompleteReason::EvaluationFailed,
                    );
                    continue;
                }
                LocalSettlement::Cancelled => {
                    add_incomplete(
                        report,
                        Some(target),
                        Some(&key.rule_id),
                        IncompleteReason::NotStarted,
                    );
                    continue;
                }
            },
            RuleOccurrenceResult::Judged { judgment, .. } => {
                let request_index = match &judgment.verdict.basis {
                    sunstack_rules::VerdictBasis::Noul { answer } => Some(answer.request_index),
                    sunstack_rules::VerdictBasis::Composite { dimensions } => dimensions
                        .first()
                        .map(|dimension| dimension.answer.request_index),
                };
                let Some(request_index) = request_index else {
                    add_incomplete(
                        report,
                        Some(target),
                        Some(&key.rule_id),
                        IncompleteReason::EvaluationFailed,
                    );
                    continue;
                };
                let Some(request) = native.requests.get(request_index) else {
                    add_incomplete(
                        report,
                        Some(target),
                        Some(&key.rule_id),
                        IncompleteReason::EvaluationFailed,
                    );
                    continue;
                };
                let mut answers = Vec::new();
                let mut missing = false;
                for binding in &request.questions {
                    if matches!(&binding.owner,QuestionOwner::Verdict{key:owner}|QuestionOwner::Informational{key:owner,..} if owner==key)
                    {
                        let reference = JudgmentReference {
                            request_index,
                            question_name: binding.request_name.clone(),
                        };
                        match native.answer(&reference) {
                            Some(answer) => answers.push(ResolvedAnswer {
                                question: binding.request_name.clone(),
                                answer: answer.clone(),
                            }),
                            None => missing = true,
                        }
                    }
                }
                if missing || !report.evidence_seen.contains_key(&request_index) {
                    add_incomplete(
                        report,
                        Some(target),
                        Some(&key.rule_id),
                        IncompleteReason::EvaluationFailed,
                    );
                    continue;
                }
                let summary = JudgmentSummary {
                    request_index,
                    basis: judgment.verdict.basis.clone(),
                    result: judgment.verdict.result.clone(),
                    value: judgment.verdict.value,
                    threshold: judgment.verdict.threshold,
                    answers,
                };
                (
                    judged_severity(&judgment.verdict.result, setting.setting),
                    target_span(&judgment.target),
                    format!(
                        "Rule {} failed ({:.3} < {:.3}).",
                        key.rule_id.as_str(),
                        judgment.verdict.value,
                        judgment.verdict.threshold
                    ),
                    Some(summary),
                )
            }
            RuleOccurrenceResult::NotApplicable { .. } => {
                report.exclusions.not_applicable =
                    report.exclusions.not_applicable.saturating_add(1);
                continue;
            }
            RuleOccurrenceResult::OutsideChangeScope { .. } => {
                report.exclusions.outside_change_scope =
                    report.exclusions.outside_change_scope.saturating_add(1);
                continue;
            }
            RuleOccurrenceResult::SelectionFailed { .. } => {
                add_incomplete(
                    report,
                    Some(target),
                    Some(&key.rule_id),
                    IncompleteReason::SelectionFailed,
                );
                continue;
            }
            RuleOccurrenceResult::EvaluationFailed { cause, .. } => {
                add_incomplete(
                    report,
                    Some(target),
                    Some(&key.rule_id),
                    incomplete_reason(cause, native),
                );
                continue;
            }
            RuleOccurrenceResult::Cancelled { .. } => {
                add_incomplete(
                    report,
                    Some(target),
                    Some(&key.rule_id),
                    IncompleteReason::NotStarted,
                );
                continue;
            }
            RuleOccurrenceResult::NotStarted { reason, .. } => {
                add_incomplete(
                    report,
                    Some(target),
                    Some(&key.rule_id),
                    match reason {
                        sunstack_rules::NotStartedReason::RequestLimit => {
                            IncompleteReason::RequestLimitReached
                        }
                        sunstack_rules::NotStartedReason::CallerCancelled => {
                            IncompleteReason::NotStarted
                        }
                    },
                );
                continue;
            }
        };
        if let Some(severity) = severity {
            match severity {
                Severity::Error => report.counts.error = report.counts.error.saturating_add(1),
                Severity::Warn => report.counts.warn = report.counts.warn.saturating_add(1),
            }
            report.findings.push(Finding {
                rule: RuleSelector::Exact {
                    entry: target.entry.clone(),
                    rule: key.rule_id.clone(),
                },
                file: target.file.clone(),
                span,
                severity,
                severity_origin: setting.decided_by.clone(),
                message,
                judgment,
            });
        }
    }
}
pub(crate) fn project_outcome(
    config: &LintConfig,
    evaluation: PolicyEvaluation,
    mut report: LintReport,
    targets: &BTreeMap<GroupKey, TargetSettings>,
    max_warnings: Option<u32>,
) -> LintReport {
    report.evidence_seen = evaluation.request_evidence;
    let (native, status) = match evaluation.outcome {
        RuleInvocationOutcome::Invalid { diagnostics } => {
            report.diagnostics = native_diagnostics(config, diagnostics);
            (None, LintStatus::Invalid)
        }
        RuleInvocationOutcome::PreparationFailed { measurements, .. } => {
            report.usage.model_requests =
                u32::try_from(measurements.model_requests).unwrap_or(u32::MAX);
            add_incomplete(&mut report, None, None, IncompleteReason::PreparationFailed);
            (None, LintStatus::Incomplete)
        }
        RuleInvocationOutcome::CompleteReport { report } => (Some(report), LintStatus::Clean),
        RuleInvocationOutcome::IncompleteReport { report } => {
            (Some(report), LintStatus::Incomplete)
        }
        RuleInvocationOutcome::Cancelled { progress, .. } => (
            match progress {
                InvocationCancellationProgress::BeforeReportAdmission => None,
                InvocationCancellationProgress::ReportAdmitted { partial_report } => {
                    Some(*partial_report)
                }
            },
            LintStatus::Cancelled,
        ),
    };
    let selected = native
        .as_ref()
        .map_or(0, |native| native.measurements.rules_selected);
    if let Some(native) = native {
        apply_native_report(&mut report, &native, targets);
    }
    let status = if !report.incomplete.is_empty()
        && !matches!(status, LintStatus::Invalid | LintStatus::Cancelled)
    {
        LintStatus::Incomplete
    } else {
        status
    };
    report.status = final_status(status, &report.counts, max_warnings, selected);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_cannot_promote_passes_or_erase_an_authored_band() {
        for (activation, setting, expected) in [
            (
                Activation::Error,
                SeveritySetting::Error,
                Some(Severity::Error),
            ),
            (
                Activation::Error,
                SeveritySetting::Warn,
                Some(Severity::Warn),
            ),
            (
                Activation::Warn,
                SeveritySetting::Error,
                Some(Severity::Error),
            ),
            (
                Activation::Warn,
                SeveritySetting::Warn,
                Some(Severity::Warn),
            ),
            (
                Activation::WarnAboveBand { band: 0.3 },
                SeveritySetting::Error,
                Some(Severity::Warn),
            ),
            (
                Activation::WarnAboveBand { band: 0.3 },
                SeveritySetting::Warn,
                Some(Severity::Warn),
            ),
            (Activation::Error, SeveritySetting::Off, None),
        ] {
            assert_eq!(
                judged_severity(&VerdictResult::Fail { activation }, setting),
                expected
            );
        }
        for setting in [
            SeveritySetting::Error,
            SeveritySetting::Warn,
            SeveritySetting::Off,
        ] {
            assert_eq!(judged_severity(&VerdictResult::Pass, setting), None);
            assert_eq!(mechanical_severity(MechanicalResult::Pass, setting), None);
        }
        assert_eq!(
            mechanical_severity(MechanicalResult::Fail, SeveritySetting::Error),
            Some(Severity::Error)
        );
        assert_eq!(
            mechanical_severity(MechanicalResult::Fail, SeveritySetting::Warn),
            Some(Severity::Warn)
        );
        assert_eq!(
            mechanical_severity(MechanicalResult::Fail, SeveritySetting::Off),
            None
        );
    }
    #[test]
    fn incomplete_and_cancelled_outcomes_precede_findings_and_empty_runs_never_claim_clean() {
        let errors = SeverityCounts { error: 1, warn: 0 };
        let warns = SeverityCounts { error: 0, warn: 1 };
        let none = SeverityCounts::default();
        for (current, counts, maximum, selected, expected) in [
            (LintStatus::Invalid, &errors, None, 1, LintStatus::Invalid),
            (
                LintStatus::Cancelled,
                &errors,
                None,
                1,
                LintStatus::Cancelled,
            ),
            (
                LintStatus::Incomplete,
                &errors,
                None,
                1,
                LintStatus::Incomplete,
            ),
            (LintStatus::Clean, &errors, None, 1, LintStatus::Findings),
            (LintStatus::Clean, &warns, None, 1, LintStatus::Clean),
            (LintStatus::Clean, &warns, Some(0), 1, LintStatus::Findings),
            (LintStatus::Clean, &warns, Some(1), 1, LintStatus::Clean),
            (
                LintStatus::Clean,
                &none,
                None,
                0,
                LintStatus::NothingSelected,
            ),
            (
                LintStatus::NothingSelected,
                &none,
                None,
                0,
                LintStatus::NothingSelected,
            ),
            (LintStatus::Clean, &none, None, 1, LintStatus::Clean),
        ] {
            assert_eq!(final_status(current, counts, maximum, selected), expected);
        }
    }
}
