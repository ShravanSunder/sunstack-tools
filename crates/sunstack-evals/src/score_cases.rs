use crate::{case_validation::invalid, *};
use std::collections::BTreeSet;
use sunstack_evidence::OccurrenceId;
use sunstack_rules::{
    CandidateEntry, CandidateOutcome, CandidateRunReport, InputLocation, QuestionRef,
    QuestionSettlement, RunUsage, SamplePass,
};

/// Score an immutable verified snapshot, without filesystem, scheduler, clock or model access.
/// Only verdict labels supply final finding truth. Other labels score their named question.
/// Filter recall counts a yes-labeled occurrence once; its first label in file order names a miss.
pub fn score(bundles: &[CandidateRunReport], set: &FrozenCaseSet) -> Result<EvalReport, EvalError> {
    let cost = sum_usage(bundles)?;
    let speed = speed_summary(bundles, set, cost.wall_time_ms);
    let mut report = EvalReport {
        rule: set.manifest.rule.clone(),
        rule_version: set.rule_version.clone(),
        case_set: set.identity.clone(),
        split: set.manifest.split,
        filter: FilterScore::default(),
        steps: vec![],
        end_result: EndScore::default(),
        unknown_labels: 0,
        cost,
        speed,
    };
    let candidates = bundles
        .iter()
        .flat_map(|bundle| &bundle.candidates)
        .filter(|candidate| candidate.rule == set.manifest.rule)
        .collect::<Vec<_>>();
    for candidate in &candidates {
        if candidate.candidate_id.rule != candidate.rule
            || candidate.candidate_id.occurrence != occurrence_identity(&candidate.occurrence)?
        {
            return Err(invalid("candidate_id"));
        }
    }
    let mut positives = BTreeSet::new();
    for case in set
        .labels
        .iter()
        .filter(|case| case.split == set.manifest.split)
    {
        let identity = occurrence_identity(&case.source_anchor)?;
        let matching = candidates
            .iter()
            .copied()
            .filter(|candidate| candidate.candidate_id.occurrence == identity)
            .collect::<Vec<_>>();
        if case.label == Label::Yes && positives.insert(identity.clone()) {
            report.filter.positive_cases += 1;
            if !matching.is_empty() {
                report.filter.kept += 1;
            } else if input_read_in_full(case, set, bundles)? {
                report.filter.missed.push(case.case_id.clone());
            } else {
                report.filter.unknown.push(case.case_id.clone());
            }
        }
        let step_index = match report
            .steps
            .iter()
            .position(|step| same_question(&step.question, &case.question))
        {
            Some(index) => index,
            None => {
                report.steps.push(StepScore {
                    question: case.question.clone(),
                    correct: vec![],
                    wrong: vec![],
                    unknown: vec![],
                });
                report.steps.len() - 1
            }
        };
        if case.label == Label::Unknown {
            report.unknown_labels += 1;
            continue;
        }
        let observed = match matching.as_slice() {
            [candidate] => observed_step(candidate, &case.question),
            _ => None,
        };
        let step = &mut report.steps[step_index];
        match observed {
            Some(value) if value == (case.label == Label::Yes) => {
                step.correct.push(case.case_id.clone())
            }
            Some(_) => step.wrong.push(case.case_id.clone()),
            None => step.unknown.push(case.case_id.clone()),
        }
        match case.question {
            QuestionRef::Verdict {} => score_end(
                case,
                &matching,
                bundles,
                &identity,
                &set.manifest.rule,
                &mut report.end_result,
            ),
            QuestionRef::Router {}
            | QuestionRef::RuleSetGate { .. }
            | QuestionRef::RuleGate { .. } => {}
        }
    }
    Ok(report)
}
fn input_read_in_full(
    case: &LabeledCase,
    set: &FrozenCaseSet,
    bundles: &[CandidateRunReport],
) -> Result<bool, EvalError> {
    let source = set
        .sources
        .iter()
        .find(|source| source.source_id == case.source)
        .ok_or_else(|| invalid("labels.source"))?;
    let identity = source
        .source
        .source_identity()
        .map_err(|_| invalid("sources.source"))?;
    let mut inputs = bundles
        .iter()
        .flat_map(|bundle| &bundle.inputs)
        .filter(|input| input.input_id == identity);
    let Some(input) = inputs.next() else {
        return Ok(false);
    };
    // Repeated or conflicting input reports cannot establish one complete discovery.
    if inputs.next().is_some() {
        return Ok(false);
    }
    Ok(matches!(input.location, InputLocation::Located { .. }) && input.unfit.is_empty())
}
fn occurrence_identity(
    occurrence: &sunstack_evidence::SourceOccurrence,
) -> Result<OccurrenceId, EvalError> {
    occurrence
        .occurrence_identity()
        .map_err(|_| invalid("source_anchor"))
}
fn same_question(left: &QuestionRef, right: &QuestionRef) -> bool {
    match (left, right) {
        (QuestionRef::Router {}, QuestionRef::Router {})
        | (QuestionRef::Verdict {}, QuestionRef::Verdict {}) => true,
        (
            QuestionRef::RuleSetGate { position: left },
            QuestionRef::RuleSetGate { position: right },
        )
        | (QuestionRef::RuleGate { position: left }, QuestionRef::RuleGate { position: right }) => {
            left == right
        }
        _ => false,
    }
}
fn observed_step(candidate: &CandidateEntry, question: &QuestionRef) -> Option<bool> {
    let mut answers = candidate
        .answers
        .iter()
        .filter(|answer| same_question(&answer.question, question));
    let answer = answers.next()?;
    // Several page/informational records are not a settled logical answer. No arbitrary first answer.
    if answers.next().is_some() {
        return None;
    }
    match answer.settlement {
        QuestionSettlement::Passed => Some(true),
        QuestionSettlement::DidNotPass => Some(false),
        QuestionSettlement::Uncertain
        | QuestionSettlement::Unresolved
        | QuestionSettlement::NotSent => None,
    }
}
fn score_end(
    case: &LabeledCase,
    candidates: &[&CandidateEntry],
    bundles: &[CandidateRunReport],
    identity: &OccurrenceId,
    rule: &sunstack_rules::RuleRef,
    end: &mut EndScore,
) {
    let finding = match candidates {
        [candidate] => match candidate.outcome {
            CandidateOutcome::Finding { .. } => Some(true),
            CandidateOutcome::Dropped { .. } => Some(false),
            CandidateOutcome::Uncertain { .. }
            | CandidateOutcome::Unresolved { .. }
            | CandidateOutcome::NotProcessed { .. } => None,
        },
        [] => {
            // A real mechanical rejected sample establishes a clear filter drop. Mere absence does not.
            let rejected = bundles
                .iter()
                .flat_map(|bundle| &bundle.dropped_samples)
                .any(|sample| {
                    &sample.rule == rule
                        && matches!(sample.pass, SamplePass::Mechanical)
                        && sample.occurrence.occurrence_identity().ok().as_ref() == Some(identity)
                });
            rejected.then_some(false)
        }
        _ => None,
    };
    match (case.label, finding) {
        (Label::Yes, Some(true)) => end.true_findings.push(case.case_id.clone()),
        (Label::No, Some(true)) => end.false_findings.push(case.case_id.clone()),
        (Label::Yes, Some(false)) => end.missed_findings.push(case.case_id.clone()),
        (Label::No, Some(false)) => end.correct_negatives.push(case.case_id.clone()),
        (Label::Yes | Label::No, None) => end.inconclusive.push(case.case_id.clone()),
        (Label::Unknown, _) => {}
    }
}
fn sum_usage(bundles: &[CandidateRunReport]) -> Result<RunUsage, EvalError> {
    let mut total = RunUsage {
        cost_usd: (!bundles.is_empty()).then_some(0.0),
        ..RunUsage::default()
    };
    for bundle in bundles {
        let usage = &bundle.usage;
        total.requests = total
            .requests
            .checked_add(usage.requests)
            .ok_or_else(|| invalid("usage.requests"))?;
        total.attempts = total
            .attempts
            .checked_add(usage.attempts)
            .ok_or_else(|| invalid("usage.attempts"))?;
        total.input_tokens = total
            .input_tokens
            .checked_add(usage.input_tokens)
            .ok_or_else(|| invalid("usage.input_tokens"))?;
        total.output_tokens = total
            .output_tokens
            .checked_add(usage.output_tokens)
            .ok_or_else(|| invalid("usage.output_tokens"))?;
        total.wall_time_ms = total
            .wall_time_ms
            .checked_add(usage.wall_time_ms)
            .ok_or_else(|| invalid("usage.wall_time_ms"))?;
        if usage
            .cost_usd
            .is_some_and(|cost| !cost.is_finite() || cost < 0.0)
        {
            return Err(invalid("usage.cost_usd"));
        }
        total.cost_usd = total
            .cost_usd
            .zip(usage.cost_usd)
            .map(|(total, cost)| total + cost);
        if total.cost_usd.is_some_and(|cost| !cost.is_finite()) {
            return Err(invalid("usage.cost_usd"));
        }
    }
    Ok(total)
}
fn speed_summary(
    bundles: &[CandidateRunReport],
    set: &FrozenCaseSet,
    run_wall_time_ms: u64,
) -> SpeedSummary {
    let mut measurements = vec![];
    let mut missing_candidates = 0;
    for candidate in bundles
        .iter()
        .flat_map(|bundle| &bundle.candidates)
        .filter(|candidate| candidate.rule == set.manifest.rule)
    {
        match candidate.elapsed_ms {
            Some(elapsed) => measurements.push(elapsed),
            None => missing_candidates += 1,
        }
    }
    measurements.sort_unstable();
    SpeedSummary {
        run_wall_time_ms,
        p50_ms: nearest_rank(&measurements, 50),
        p95_ms: nearest_rank(&measurements, 95),
        measured_candidates: measurements.len(),
        missing_candidates,
        percentile: PercentileConvention::NearestRank,
    }
}
fn nearest_rank(sorted: &[u64], percentile: usize) -> Option<u64> {
    let rank =
        (sorted.len() / 100) * percentile + ((sorted.len() % 100) * percentile).div_ceil(100);
    sorted.get(rank.checked_sub(1)?).copied()
}
