use crate::{
    candidate_admission::CandidateEvidenceInputs,
    candidate_definitions::CandidateRouter,
    candidate_inputs::{CandidateMaterial, CandidateRecordSelection},
    input_binding::*,
    invocation_admission::*,
    *,
};
use std::sync::Arc;

pub(crate) fn admit_router_candidate(
    router: &CandidateRouter,
    sets: &[Arc<LoadedRuleSet>],
    outside: &CandidateEvidenceInputs,
    material: &CandidateMaterial,
    position: usize,
    groups: &mut Vec<AdmittedGroup>,
    policy: &mut RulePolicy,
) -> Result<AdmittedSources, DiagnosticList> {
    let invalid = || DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring);
    let subject = router
        .targets
        .get(&(
            material.rule.rule_set.clone(),
            material.rule.rule_id.clone(),
        ))
        .ok_or_else(invalid)?;
    let mut sources = outside
        .entries
        .get(&(router.path.clone(), material.entry_position))
        .cloned()
        .ok_or_else(invalid)?;
    for source in sources.values_mut() {
        source.candidate = Some(CandidateSourceView::Outside {
            grants: material.view.grants.iter().copied().collect(),
        });
    }
    let selected = match &material.selected {
        CandidateRecordSelection::Ready { selected } => selected.clone(),
        CandidateRecordSelection::LeftOutForPrivacy => return Err(invalid()),
    };
    sources.insert(
        subject.clone(),
        AdmittedSource {
            id: SourceId::Conversation {
                path: material.path.clone(),
                format: material.format.clone(),
                supplement_identity: None,
            },
            name: material.source_name.clone(),
            spec: sunstack_evidence::SourceSpec::Conversation {
                transcript_path: material.path.clone(),
                format: material.format.clone(),
                final_assistant_text: None,
            },
            candidate: Some(CandidateSourceView::Record(selected)),
        },
    );
    for (label, targets) in &router.definition.routes {
        for target in targets {
            let set = sets
                .iter()
                .find(|set| set.path == target.rule_set)
                .ok_or_else(invalid)?;
            let bound = target
                .inputs
                .iter()
                .filter_map(|(name, router_name)| {
                    sources
                        .get(router_name)
                        .map(|source| (name.clone(), source.clone()))
                })
                .collect();
            groups.push(AdmittedGroup {
                set: set.clone(),
                position,
                binding: GroupBinding::Routed {
                    router_group: position,
                    label: label.clone(),
                },
                sources: bound,
            });
            policy.off.insert(
                GroupKey {
                    rule_set: set.path.clone(),
                    group: position,
                },
                set.rules
                    .iter()
                    .filter(|rule| {
                        set.path != material.rule.rule_set
                            || rule.definition.rule_id != material.rule.rule_id
                    })
                    .map(|rule| rule.definition.rule_id.clone())
                    .collect(),
            );
        }
    }
    Ok(sources)
}

pub(crate) fn router_outcome(
    router: Option<&CandidateRouter>,
    group: usize,
    rule_set: &std::path::Path,
    report: &RuleInvocationReport,
    answers: &mut [AnswerRecord],
) -> Option<CandidateOutcome> {
    let router = router?;
    let RoutingReport::Router { groups } = &report.routing else {
        return None;
    };
    let outcome = &groups.iter().find(|entry| entry.position == group)?.outcome;
    let answer = answers
        .iter_mut()
        .find(|answer| matches!(answer.question, QuestionRef::Router {}));
    match outcome {
        RouterOutcome::UncertainIntent { .. } => {
            let answer = answer?;
            answer.settlement = QuestionSettlement::Uncertain;
            Some(CandidateOutcome::Uncertain {
                at: QuestionRef::Router {},
                answers: vec![answer.answer_id.clone()],
            })
        }
        RouterOutcome::NoRoute { .. } => {
            let answer = answer?;
            answer.settlement = QuestionSettlement::DidNotPass;
            Some(CandidateOutcome::Dropped {
                at: QuestionRef::Router {},
                answer: answer.answer_id.clone(),
            })
        }
        RouterOutcome::Routed { label, .. }
            if !router.definition.routes.get(label).is_some_and(|targets| {
                targets.iter().any(|target| target.rule_set == rule_set)
            }) =>
        {
            let answer = answer?;
            answer.settlement = QuestionSettlement::DidNotPass;
            Some(CandidateOutcome::Dropped {
                at: QuestionRef::Router {},
                answer: answer.answer_id.clone(),
            })
        }
        RouterOutcome::RouterFailed { cause } => Some(CandidateOutcome::NotProcessed {
            reason: match cause {
                Cause::Request { .. } => NotProcessedReason::EvaluationFailed,
                Cause::Budget { .. } => NotProcessedReason::OverBudget,
                _ => NotProcessedReason::Refused,
            },
        }),
        RouterOutcome::NotEvaluated { .. } => Some(CandidateOutcome::NotProcessed {
            reason: NotProcessedReason::RunStopped,
        }),
        _ => None,
    }
}
