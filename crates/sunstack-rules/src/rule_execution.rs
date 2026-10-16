use crate::applicability_evaluation::{evaluate_conditions, stopped_occurrence};
use crate::{invocation_admission::admit_invocation, request_encoding::*, request_state::*, *};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use sunstack_decisions::{DecisionBackend, DecisionRunOutcome, run_admitted_judgments};
use sunstack_evidence::{AdapterRegistry, EvidenceMeasurements, EvidenceSession};
use tokio::{sync::Semaphore, task::JoinHandle};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
#[derive(Clone)]
pub struct RuleExecutionContext {
    pub backend: Arc<dyn DecisionBackend>,
    pub adapters: AdapterRegistry,
    pub cancellation: CancellationToken,
}
struct PendingRequest {
    index: usize,
    blocks: Vec<RuleRequestBlock>,
    task: JoinHandle<(DecisionRunOutcome, u64)>,
}
fn cancelled_before() -> RuleInvocationOutcome {
    RuleInvocationOutcome::Cancelled {
        progress: InvocationCancellationProgress::BeforeReportAdmission,
        diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
        measurements: InvocationMeasurements::default(),
    }
}
fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
pub async fn evaluate_rules(
    input: RuleInvocationInput,
    context: RuleExecutionContext,
) -> RuleInvocationOutcome {
    if context.cancellation.is_cancelled() {
        return cancelled_before();
    }
    if input.backend != *context.backend.selection() {
        return RuleInvocationOutcome::Invalid {
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration),
        };
    }
    let local_started = Instant::now();
    let admitted = match admit_invocation(&input, &context.adapters, &context.cancellation).await {
        Ok(admitted) => admitted,
        Err(diagnostics) => {
            return if context.cancellation.is_cancelled() {
                cancelled_before()
            } else {
                RuleInvocationOutcome::Invalid { diagnostics }
            };
        }
    };
    if context.cancellation.is_cancelled() {
        return cancelled_before();
    }
    // Execution of gated plans is added with the staged/same-input proof scenarios.
    if admitted.router.is_some()
        || admitted.groups.iter().any(|group| {
            group
                .set
                .definition
                .applicability
                .iter()
                .any(|entry| matches!(entry, ApplicabilityEntry::Decision { .. }))
                || group.set.rules.iter().any(|rule| {
                    rule.definition
                        .applicability
                        .iter()
                        .any(|entry| matches!(entry, ApplicabilityEntry::Decision { .. }))
                })
        })
    {
        return RuleInvocationOutcome::Invalid {
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration),
        };
    }
    let mut session = match EvidenceSession::open(
        admitted.evidence,
        context.adapters.clone(),
        context.cancellation.clone(),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => {
            return RuleInvocationOutcome::Invalid {
                diagnostics: error.diagnostics().into(),
            };
        }
    };
    let mut report = RuleInvocationReport {
        origin: (&input.origin).into(),
        routing: RoutingReport::RuleSets,
        groups: Vec::new(),
        occurrences: Vec::new(),
        requests: Vec::new(),
        captures: Vec::new(),
        cursors: Vec::new(),
        diagnostics: DiagnosticList::default(),
        measurements: InvocationMeasurements::default(),
    };
    report.measurements.local_elapsed_ms = elapsed_ms(local_started);
    let tracker = TaskTracker::new();
    let semaphore = Arc::new(Semaphore::new(
        input.execution.max_concurrent_requests.get(),
    ));
    let mut pending = Vec::new();
    let mut authored_order = BTreeMap::new();
    for group in &admitted.groups {
        for located in &group.set.rules {
            let key = RuleOccurrenceKey {
                rule_set: group.set.path.clone(),
                group: group.position,
                rule_id: located.definition.rule_id.clone(),
            };
            let position = authored_order.len();
            authored_order.insert(key, position);
        }
    }

    for group in admitted.groups {
        let (applicability, entries) = evaluate_conditions(
            &group.set.definition.applicability,
            EntryOwner::RuleSet {
                path: group.set.path.clone(),
                group: group.position,
            },
            &group.sources,
            &input.origin,
            &mut session,
            &context.cancellation,
        )
        .await;
        report.measurements.applicability_entries_evaluated += entries
            .iter()
            .filter(|entry| !matches!(entry.result, EntryResult::NotEvaluated { .. }))
            .count();
        if let ApplicabilityOutcome::ApplicabilityFailed {
            cause: Cause::Condition { diagnostic, .. },
            ..
        } = &applicability
        {
            report.diagnostics.push(diagnostic.clone());
        }
        let stopped = !matches!(applicability, ApplicabilityOutcome::Applicable);
        report.groups.push(GroupResult {
            rule_set: group.set.path.clone(),
            position: group.position,
            binding: group.binding.clone(),
            applicability: applicability.clone(),
            entries,
        });
        if stopped {
            for located in &group.set.rules {
                let key = RuleOccurrenceKey {
                    rule_set: group.set.path.clone(),
                    group: group.position,
                    rule_id: located.definition.rule_id.clone(),
                };
                if let Some(result) = stopped_occurrence(key, &applicability) {
                    report.occurrences.push(result);
                }
            }
            continue;
        }
        if context.cancellation.is_cancelled() {
            for rule in &group.set.rules {
                report.occurrences.push(RuleOccurrenceResult::NotStarted {
                    key: RuleOccurrenceKey {
                        rule_set: group.set.path.clone(),
                        group: group.position,
                        rule_id: rule.definition.rule_id.clone(),
                    },
                    reason: NotStartedReason::CallerCancelled,
                });
            }
            continue;
        }
        let permit = tokio::select! {biased;_=context.cancellation.cancelled()=>None,permit=semaphore.clone().acquire_owned()=>permit.ok()};
        let Some(permit) = permit else {
            for rule in &group.set.rules {
                report.occurrences.push(RuleOccurrenceResult::NotStarted {
                    key: RuleOccurrenceKey {
                        rule_set: group.set.path.clone(),
                        group: group.position,
                        rule_id: rule.definition.rule_id.clone(),
                    },
                    reason: NotStartedReason::CallerCancelled,
                });
            }
            continue;
        };
        let prepared_at = Instant::now();
        let mut blocks = Vec::new();
        let mut refused = DiagnosticList::default();
        for located in &group.set.rules {
            let rule = &located.definition;
            let key = RuleOccurrenceKey {
                rule_set: group.set.path.clone(),
                group: group.position,
                rule_id: rule.rule_id.clone(),
            };
            let (applicability, entries) = evaluate_conditions(
                &rule.applicability,
                EntryOwner::Rule {
                    path: group.set.path.clone(),
                    group: group.position,
                    rule_id: rule.rule_id.clone(),
                },
                &group.sources,
                &input.origin,
                &mut session,
                &context.cancellation,
            )
            .await;
            report.measurements.applicability_entries_evaluated += entries
                .iter()
                .filter(|entry| !matches!(entry.result, EntryResult::NotEvaluated { .. }))
                .count();
            if let ApplicabilityOutcome::ApplicabilityFailed {
                cause: Cause::Condition { diagnostic, .. },
                ..
            } = &applicability
            {
                report.diagnostics.push(diagnostic.clone());
            }
            if let Some(group_result) = report.groups.last_mut() {
                group_result.entries.extend(entries);
            }
            if let Some(result) = stopped_occurrence(key.clone(), &applicability) {
                report.occurrences.push(result);
                continue;
            }
            let evidence =
                select_rule_evidence(rule, &group.sources, group.position, &mut session).await;
            if let Err(diagnostics) = &evidence.target {
                refused.extend(diagnostics.clone());
                report.diagnostics.extend(diagnostics.clone());
                report
                    .occurrences
                    .push(RuleOccurrenceResult::SelectionFailed {
                        key,
                        diagnostics: diagnostics.clone(),
                    });
            } else {
                blocks.push(RuleRequestBlock {
                    key,
                    rule: rule.clone(),
                    evidence,
                    common: group.set.definition.common_ruleset_instructions.clone(),
                });
            }
        }
        let owner = RequestOwner::RuleSetGroup {
            rule_set: group.set.path.clone(),
            group: group.position,
        };
        if blocks.is_empty() && refused.is_empty() {
            drop(permit);
            continue;
        }
        if context.cancellation.is_cancelled() {
            for block in blocks {
                report
                    .occurrences
                    .push(RuleOccurrenceResult::Cancelled { key: block.key });
            }
            drop(permit);
            continue;
        }
        if blocks.is_empty() {
            report.requests.push(RequestRecord {
                index: report.requests.len(),
                owner,
                mode: RequestMode::Rules,
                questions: vec![],
                prerequisites: vec![],
                status: RequestStatus::NotSent {
                    reason: NotSentReason::EvidenceRefused {
                        diagnostics: refused,
                    },
                },
                discarded: vec![],
            });
            drop(permit);
            continue;
        }
        let encoded = encode_rules(&blocks, context.backend.as_ref(), &input.request_budget);
        report.measurements.local_elapsed_ms = report
            .measurements
            .local_elapsed_ms
            .saturating_add(elapsed_ms(prepared_at));
        match encoded {
            Err(diagnostics) => {
                for block in blocks {
                    report
                        .occurrences
                        .push(RuleOccurrenceResult::EvaluationFailed {
                            key: block.key,
                            cause: Cause::Budget {
                                diagnostics: diagnostics.clone(),
                            },
                        });
                }
                report.diagnostics.extend(diagnostics.clone());
                report.requests.push(RequestRecord {
                    index: report.requests.len(),
                    owner,
                    mode: RequestMode::Rules,
                    questions: vec![],
                    prerequisites: vec![],
                    status: RequestStatus::NotSent {
                        reason: NotSentReason::BudgetRefused { diagnostics },
                    },
                    discarded: vec![],
                });
                drop(permit);
            }
            Ok(encoded) => {
                if report.measurements.model_requests >= input.request_budget.max_requests {
                    for block in blocks {
                        report.occurrences.push(RuleOccurrenceResult::NotStarted {
                            key: block.key,
                            reason: NotStartedReason::RequestLimit,
                        });
                    }
                    report.requests.push(RequestRecord {
                        index: report.requests.len(),
                        owner,
                        mode: RequestMode::Rules,
                        questions: encoded.bindings,
                        prerequisites: vec![],
                        status: RequestStatus::NotSent {
                            reason: NotSentReason::RequestLimit,
                        },
                        discarded: vec![],
                    });
                    drop(permit);
                    continue;
                }
                let index = report.requests.len();
                report.requests.push(RequestRecord {
                    index,
                    owner,
                    mode: RequestMode::Rules,
                    questions: encoded.bindings,
                    prerequisites: vec![],
                    status: RequestStatus::NotSent {
                        reason: NotSentReason::Cancelled,
                    },
                    discarded: vec![],
                });
                let backend = context.backend.clone();
                let attempts = input.attempts.clone();
                let cancellation = context.cancellation.clone();
                let task = tracker.spawn(async move {
                    let started = Instant::now();
                    let outcome = run_admitted_judgments(
                        &encoded.request,
                        backend.as_ref(),
                        &encoded.admitted,
                        &attempts,
                        &cancellation,
                    )
                    .await;
                    drop(permit);
                    (outcome, elapsed_ms(started))
                });
                report.measurements.model_requests += 1;
                pending.push(PendingRequest {
                    index,
                    blocks,
                    task,
                });
            }
        }
    }
    tracker.close();
    for request in pending {
        let (outcome, provider_elapsed) = match request.task.await {
            Ok(result) => result,
            Err(_) => {
                report.diagnostics.extend(DiagnosticList::rule(
                    RuleDiagnosticCode::UnexpectedExecutionError,
                ));
                for block in request.blocks {
                    report
                        .occurrences
                        .push(RuleOccurrenceResult::EvaluationFailed {
                            key: block.key,
                            cause: Cause::Request {
                                request_index: request.index,
                            },
                        });
                }
                continue;
            }
        };
        report.measurements.provider_elapsed_ms = report
            .measurements
            .provider_elapsed_ms
            .saturating_add(provider_elapsed);
        report.measurements.inference_attempts += match &outcome {
            DecisionRunOutcome::Completed { attempts, .. }
            | DecisionRunOutcome::ProviderFailure { attempts, .. }
            | DecisionRunOutcome::InvalidResponse { attempts, .. }
            | DecisionRunOutcome::Cancelled { attempts, .. } => attempts.len(),
            DecisionRunOutcome::InvalidRequest { .. } => 0,
        };
        for block in request.blocks {
            report.occurrences.push(reduce_rule(
                block,
                request.index,
                &report.requests[request.index].questions,
                &outcome,
            ));
        }
        report.requests[request.index].status = RequestStatus::Sent { outcome };
    }
    tracker.wait().await;
    let closed = session.close();
    report.captures = closed.captures;
    report.cursors = closed.cursors;
    copy_evidence_measurements(&mut report.measurements, closed.measurements);
    report.occurrences.sort_by_key(|occurrence| {
        authored_order
            .get(occurrence.key())
            .copied()
            .unwrap_or(usize::MAX)
    });
    for occurrence in &report.occurrences {
        match occurrence {
            RuleOccurrenceResult::Judged { .. } => report.measurements.rules_selected += 1,
            RuleOccurrenceResult::NotApplicable { .. }
            | RuleOccurrenceResult::OutsideChangeScope { .. } => {
                report.measurements.rules_skipped += 1
            }
            _ => report.measurements.rules_failed += 1,
        }
    }
    if context.cancellation.is_cancelled() {
        let measurements = report.measurements.clone();
        RuleInvocationOutcome::Cancelled {
            progress: InvocationCancellationProgress::ReportAdmitted {
                partial_report: report,
            },
            diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
            measurements,
        }
    } else if report.is_complete() {
        RuleInvocationOutcome::CompleteReport { report }
    } else {
        RuleInvocationOutcome::IncompleteReport { report }
    }
}
fn reduce_rule(
    block: RuleRequestBlock,
    index: usize,
    bindings: &[QuestionBinding],
    outcome: &DecisionRunOutcome,
) -> RuleOccurrenceResult {
    match outcome {
        DecisionRunOutcome::Completed { answers, .. } => {
            let mut verdict_answers = VerdictAnswers {
                noul_verdict: None,
                informational: BTreeMap::new(),
            };
            let mut references = BTreeMap::new();
            for binding in bindings {
                let Some(answer) = answers.get(&binding.request_name) else {
                    continue;
                };
                let reference = JudgmentReference {
                    request_index: index,
                    question_name: binding.request_name.clone(),
                };
                match &binding.owner {
                    QuestionOwner::Verdict { rule_id } if rule_id == &block.key.rule_id => {
                        verdict_answers.noul_verdict = Some(ReferencedAnswer {
                            answer: answer.clone(),
                            reference,
                        })
                    }
                    QuestionOwner::Informational { rule_id, name }
                        if rule_id == &block.key.rule_id =>
                    {
                        verdict_answers.informational.insert(
                            name.clone(),
                            ReferencedAnswer {
                                answer: answer.clone(),
                                reference: reference.clone(),
                            },
                        );
                        references.insert(name.clone(), reference);
                    }
                    _ => {}
                }
            }
            match (
                evaluate_verdict(&block.rule, &verdict_answers),
                block.evidence.target,
            ) {
                (Ok(verdict), Ok(target)) => RuleOccurrenceResult::Judged {
                    key: block.key.clone(),
                    judgment: RuleJudgment {
                        key: block.key,
                        verdict,
                        declared_severity: block.rule.severity,
                        answers: references,
                        target,
                    },
                },
                _ => RuleOccurrenceResult::EvaluationFailed {
                    key: block.key,
                    cause: Cause::Request {
                        request_index: index,
                    },
                },
            }
        }
        DecisionRunOutcome::Cancelled { .. } => RuleOccurrenceResult::Cancelled { key: block.key },
        _ => RuleOccurrenceResult::EvaluationFailed {
            key: block.key,
            cause: Cause::Request {
                request_index: index,
            },
        },
    }
}
fn copy_evidence_measurements(target: &mut InvocationMeasurements, evidence: EvidenceMeasurements) {
    target.files_considered = evidence.files_considered;
    target.files_read = evidence.files_read;
    target.files_parsed = evidence.files_parsed;
    target.transcript_bytes_read = evidence.transcript_bytes_read;
    target.transcript_bytes_parsed = evidence.transcript_bytes_parsed;
    target.bytes_scanned = evidence.bytes_scanned;
    target.bytes_included = evidence.bytes_included;
}
