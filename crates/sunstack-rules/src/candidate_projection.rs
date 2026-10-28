use crate::{candidate_inputs::CandidateMaterial, *};
use sunstack_decisions::{DecisionRunOutcome, JudgmentAnswer};

pub(crate) fn project_candidate_usage(report: &RuleInvocationReport) -> RunUsage {
    let mut usage = RunUsage {
        requests: report.measurements.model_requests,
        attempts: report.measurements.inference_attempts,
        ..RunUsage::default()
    };
    let mut known_cost = report.measurements.model_requests > 0;
    let mut cost = 0.0;
    for request in &report.requests {
        if let RequestStatus::Sent {
            outcome: DecisionRunOutcome::Completed { receipt, .. },
        } = &request.status
        {
            usage.input_tokens = usage
                .input_tokens
                .saturating_add(receipt.usage.input_tokens);
            usage.output_tokens = usage
                .output_tokens
                .saturating_add(receipt.usage.output_tokens);
            if let Some(value) = receipt.usage.cost_usd {
                cost += value;
            } else {
                known_cost = false;
            }
        } else if matches!(request.status, RequestStatus::Sent { .. }) {
            known_cost = false;
        }
    }
    usage.cost_usd = if known_cost { Some(cost) } else { None };
    usage
}
pub(crate) fn project_candidate(
    material: CandidateMaterial,
    group: usize,
    set: &crate::invocation_admission::LoadedRuleSet,
    report: Option<&RuleInvocationReport>,
    router: Option<&crate::candidate_definitions::CandidateRouter>,
) -> Result<CandidateEntry, DiagnosticList> {
    let mut answers = Vec::new();
    let mut references = std::collections::BTreeMap::new();
    let definition = set
        .rules
        .iter()
        .find(|rule| rule.definition.rule_id == material.rule.rule_id)
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    if let Some(report) = report {
        for request in &report.requests {
            if !matches!(&request.owner, RequestOwner::RuleSetGroup { rule_set, group: position, .. } if *rule_set == material.rule.rule_set && *position == group)
                && !matches!(&request.owner, RequestOwner::Router {group:position} if router.is_some() && *position==group)
            {
                continue;
            }
            for (position, question) in request.questions.iter().enumerate() {
                if request
                    .discarded
                    .iter()
                    .any(|discarded| discarded.question_name == question.request_name)
                {
                    continue;
                }
                let (question_ref, threshold, floor) = match &question.owner {
                    QuestionOwner::Verdict { .. } => (
                        QuestionRef::Verdict {},
                        definition.definition.verdict.threshold(),
                        definition.definition.verdict.uncertainty_floor(),
                    ),
                    QuestionOwner::Informational { name, .. } => (
                        QuestionRef::Informational { name: name.clone() },
                        None,
                        match &definition.definition.verdict {
                            VerdictDeclaration::Composite {
                                dimensions,
                                uncertain_below,
                                ..
                            } if dimensions
                                .iter()
                                .any(|dimension| dimension.question == *name) =>
                            {
                                *uncertain_below
                            }
                            _ => None,
                        },
                    ),
                    QuestionOwner::RuleDecision { entry, .. } => {
                        let entries = crate::candidate_gates::rule_entries(&definition.definition);
                        (
                            crate::candidate_gates::reference(entry, &definition.definition),
                            decision_threshold(&entries, entry.position),
                            decision_floor(&entries, entry.position),
                        )
                    }
                    QuestionOwner::RuleSetDecision { entry } => (
                        QuestionRef::RuleSetGate {
                            position: entry.position,
                        },
                        decision_threshold(&set.definition.applicability, entry.position),
                        decision_floor(&set.definition.applicability, entry.position),
                    ),
                    QuestionOwner::Router { .. } => (QuestionRef::Router {}, None, None),
                };
                let answer_id = AnswerId(
                    sunstack_evidence::DeclarationId::new(format!(
                        "a{}_{}",
                        request.index, position
                    ))
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
                );
                let answer = match &request.status {
                    RequestStatus::Sent {
                        outcome: DecisionRunOutcome::Completed { answers, .. },
                    } => answers.get(&question.request_name).cloned(),
                    _ => None,
                };
                let settlement = if answer
                    .as_ref()
                    .is_some_and(|answer| below_floor(answer, floor))
                {
                    QuestionSettlement::Uncertain
                } else {
                    match (&answer, threshold) {
                        (Some(JudgmentAnswer::Noul { yes_probability }), Some(threshold))
                            if *yes_probability < threshold =>
                        {
                            QuestionSettlement::DidNotPass
                        }
                        (Some(_), _) => QuestionSettlement::Passed,
                        (None, _) => QuestionSettlement::NotSent,
                    }
                };
                references.insert(
                    (request.index, question.request_name.clone()),
                    answer_id.clone(),
                );
                answers.push(AnswerRecord {
                    answer_id,
                    question: question_ref,
                    page: None,
                    answer,
                    settlement,
                });
            }
        }
    }
    if let Some(report) = report {
        for evaluated in report
            .groups
            .iter()
            .filter(|result| result.rule_set == material.rule.rule_set && result.position == group)
            .flat_map(|result| &result.entries)
        {
            if matches!(&evaluated.key.owner, EntryOwner::Rule { rule_id, .. } if *rule_id != material.rule.rule_id)
            {
                continue;
            }
            if let EntryResult::Condition { outcome, .. } = &evaluated.result {
                let owner = if matches!(evaluated.key.owner, EntryOwner::RuleSet { .. }) {
                    "set"
                } else {
                    "rule"
                };
                let answer_id = AnswerId(
                    sunstack_evidence::DeclarationId::new(format!(
                        "local_{owner}_{}",
                        evaluated.key.position
                    ))
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
                );
                let settlement = match outcome {
                    ConditionOutcome::Passed => QuestionSettlement::Passed,
                    ConditionOutcome::DidNotPass => QuestionSettlement::DidNotPass,
                    ConditionOutcome::Unresolved { .. } => QuestionSettlement::Unresolved,
                };
                answers.push(AnswerRecord {
                    answer_id,
                    question: crate::candidate_gates::reference(
                        &evaluated.key,
                        &definition.definition,
                    ),
                    page: None,
                    answer: None,
                    settlement,
                });
            }
        }
    }
    let lookup = |reference: &JudgmentReference| {
        references
            .get(&(reference.request_index, reference.question_name.clone()))
            .cloned()
    };
    let occurrence = report.and_then(|report| {
        report.occurrences.iter().find(|occurrence| {
            let key = occurrence.key();
            key.rule_set == material.rule.rule_set
                && key.group == group
                && key.rule_id == material.rule.rule_id
        })
    });
    let applicability_stop = match occurrence {
        Some(RuleOccurrenceResult::NotApplicable {
            stopped_at, basis, ..
        }) => Some((stopped_at, basis)),
        _ => report.and_then(|report| {
            report
                .rule_evaluations
                .iter()
                .find_map(|evaluation| match evaluation {
                    RuleEvaluationResult::NotApplicable {
                        key,
                        stopped_at,
                        basis,
                    } if key.rule_set == material.rule.rule_set
                        && key.group == group
                        && key.rule_id == material.rule.rule_id =>
                    {
                        Some((stopped_at, basis))
                    }
                    _ => None,
                })
        }),
    };
    let uncertain_gate = report.and_then(|report| {
        report
            .rule_evaluations
            .iter()
            .find_map(|evaluation| match evaluation {
                RuleEvaluationResult::Uncertain { key, at, answer }
                    if key.rule_set == material.rule.rule_set
                        && key.group == group
                        && key.rule_id == material.rule.rule_id =>
                {
                    Some((at, answer))
                }
                _ => None,
            })
    });
    let routing = report.and_then(|report| {
        crate::candidate_routing::router_outcome(
            router,
            group,
            &material.rule.rule_set,
            report,
            &mut answers,
        )
    });
    let outcome = if let Some(outcome) = routing {
        answers.retain(|answer| matches!(answer.question, QuestionRef::Router {}));
        outcome
    } else if let Some((at, answer)) = uncertain_gate {
        CandidateOutcome::Uncertain {
            at: crate::candidate_gates::reference(at, &definition.definition),
            answers: lookup(answer).into_iter().collect(),
        }
    } else if let Some((stopped_at, basis)) = applicability_stop {
        let at = crate::candidate_gates::reference(stopped_at, &definition.definition);
        let answer = match basis {
            EntryBasis::Decision { answer, .. } => lookup(answer),
            EntryBasis::Condition { .. } => answers
                .iter()
                .find(|answer| {
                    answer.question == at
                        && matches!(answer.settlement, QuestionSettlement::DidNotPass)
                })
                .map(|answer| answer.answer_id.clone()),
        };
        match answer {
            Some(answer) => CandidateOutcome::Dropped { at, answer },
            None => CandidateOutcome::NotProcessed {
                reason: NotProcessedReason::EvaluationFailed,
            },
        }
    } else {
        match occurrence {
            Some(RuleOccurrenceResult::Judged { judgment, .. }) => {
                crate::candidate_verdict_projection::project_verdict(
                    judgment,
                    &mut answers,
                    &references,
                )?
            }
            Some(RuleOccurrenceResult::Uncertain { at, answer, .. }) => {
                CandidateOutcome::Uncertain {
                    at: crate::candidate_gates::reference(at, &definition.definition),
                    answers: lookup(answer).into_iter().collect(),
                }
            }
            Some(RuleOccurrenceResult::NotStarted {
                reason: NotStartedReason::RequestLimit,
                ..
            })
            | Some(RuleOccurrenceResult::Cancelled { .. }) => CandidateOutcome::NotProcessed {
                reason: NotProcessedReason::RunStopped,
            },
            Some(RuleOccurrenceResult::SelectionFailed { .. }) => CandidateOutcome::NotProcessed {
                reason: NotProcessedReason::Refused,
            },
            Some(RuleOccurrenceResult::EvaluationFailed { cause, .. }) => {
                CandidateOutcome::NotProcessed {
                    reason: report.map_or(NotProcessedReason::Refused, |report| {
                        native_failure_reason(cause, report)
                    }),
                }
            }
            Some(
                RuleOccurrenceResult::Checked { .. }
                | RuleOccurrenceResult::NotApplicable { .. }
                | RuleOccurrenceResult::OutsideChangeScope { .. }
                | RuleOccurrenceResult::NotStarted { .. },
            ) => CandidateOutcome::NotProcessed {
                reason: NotProcessedReason::Refused,
            },
            None => {
                let reason = report
                    .and_then(|report| {
                        report
                            .rule_evaluations
                            .iter()
                            .find_map(|evaluation| match evaluation {
                                RuleEvaluationResult::EvaluationFailed { key, cause }
                                    if key.rule_set == material.rule.rule_set
                                        && key.group == group
                                        && key.rule_id == material.rule.rule_id =>
                                {
                                    Some(native_failure_reason(cause, report))
                                }
                                RuleEvaluationResult::Cancelled { key }
                                    if key.rule_set == material.rule.rule_set
                                        && key.group == group
                                        && key.rule_id == material.rule.rule_id =>
                                {
                                    Some(NotProcessedReason::RunStopped)
                                }
                                _ => None,
                            })
                    })
                    .unwrap_or(NotProcessedReason::Refused);
                CandidateOutcome::NotProcessed { reason }
            }
        }
    };
    Ok(CandidateEntry {
        candidate_id: material.candidate_id,
        rule: material.rule,
        occurrence: material.occurrence,
        outcome,
        answers,
        evidence: vec![],
        privacy_view: material.view,
        elapsed_ms: None,
    })
}
fn native_failure_reason(cause: &Cause, report: &RuleInvocationReport) -> NotProcessedReason {
    match cause {
        Cause::Request { .. } => NotProcessedReason::EvaluationFailed,
        Cause::Budget { .. } => NotProcessedReason::OverBudget,
        Cause::Evidence { .. } | Cause::Condition { .. } => NotProcessedReason::Refused,
        Cause::Gate { entry } => {
            let failed = report
                .groups
                .iter()
                .flat_map(|group| &group.entries)
                .find(|evaluated| evaluated.key == *entry);
            match failed.map(|entry| &entry.result) {
                Some(EntryResult::Decision {
                    outcome:
                        DecisionEntryOutcome::Failed {
                            cause: Cause::Request { .. },
                        },
                    ..
                }) => NotProcessedReason::EvaluationFailed,
                Some(EntryResult::Decision {
                    outcome:
                        DecisionEntryOutcome::Failed {
                            cause: Cause::Budget { .. },
                        },
                    ..
                }) => NotProcessedReason::OverBudget,
                _ => NotProcessedReason::Refused,
            }
        }
    }
}
fn decision_threshold(entries: &[ApplicabilityEntry], position: usize) -> Option<f64> {
    match entries.get(position) {
        Some(ApplicabilityEntry::Decision { threshold, .. }) => Some(*threshold),
        _ => None,
    }
}

fn decision_floor(entries: &[ApplicabilityEntry], position: usize) -> Option<f64> {
    match entries.get(position) {
        Some(ApplicabilityEntry::Decision {
            uncertain_below, ..
        }) => *uncertain_below,
        _ => None,
    }
}
fn below_floor(answer: &JudgmentAnswer, floor: Option<f64>) -> bool {
    floor.is_some_and(|floor| match answer {
        JudgmentAnswer::Noul { yes_probability } => {
            yes_probability.max(1.0 - *yes_probability) < floor
        }
        JudgmentAnswer::Score { confidence, .. } | JudgmentAnswer::Choice { confidence, .. } => {
            *confidence < floor
        }
    })
}
