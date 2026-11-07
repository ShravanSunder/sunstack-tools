use super::contracts::*;
use crate::configuration_loading::config_validation::CheckedRulebook;
use std::collections::BTreeSet;
use sunstack_decisions::{JudgmentAnswer, NamedAnswerMap, NamedQuestionMap};
use sunstack_rules::{DecisionGraph, DecisionStep, StepName};

fn step_bounds(step: &DecisionStep) -> Vec<(BarBound, f64)> {
    match step {
        DecisionStep::AskNoul { routes, .. }
        | DecisionStep::AskScore { routes, .. }
        | DecisionStep::Combine { routes, .. } => vec![
            (BarBound::YesAtOrAbove, routes.yes_at_or_above),
            (BarBound::NoAtOrBelow, routes.no_at_or_below),
        ],
        DecisionStep::AskChoice {
            min_probability, ..
        } => vec![(BarBound::MinProbability, *min_probability)],
        DecisionStep::Mechanical { .. } => vec![],
    }
}
fn graphs(checked: &CheckedRulebook) -> Vec<(&str, &DecisionGraph, &NamedQuestionMap)> {
    checked
        .assembly
        .rules
        .iter()
        .map(|rule| (rule.name.as_str(), &rule.decision, &rule.questions))
        .chain(
            checked
                .filters
                .values()
                .map(|filter| (filter.name.as_str(), &filter.decision, &filter.questions)),
        )
        .collect()
}
pub(super) fn assess(file: &CaseFile, checked: &CheckedRulebook) -> Vec<BarAssessment> {
    let ids = file
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut assessments = Vec::new();
    for (owner, graph, _) in graphs(checked) {
        for (step, definition) in &graph.steps {
            for (bound, value) in step_bounds(definition) {
                let recorded = file
                    .bars
                    .iter()
                    .filter(|bar| bar.owner == owner && &bar.step == step && bar.bound == bound)
                    .collect::<Vec<_>>();
                let reason = match recorded.as_slice() {
                    [] => Some("bar has no recorded development cases".into()),
                    [bar] if bar.value != value => Some("bar changed: unassessed".into()),
                    [bar]
                        if bar.set_from.is_empty()
                            || bar.set_from.iter().any(|id| !ids.contains(id.as_str())) =>
                    {
                        Some("bar refers to no cases or an unknown case".into())
                    }
                    [_] => None,
                    _ => Some("bar has multiple records".into()),
                };
                assessments.push(BarAssessment {
                    owner: owner.into(),
                    step: step.clone(),
                    bound,
                    value: recorded.first().map(|bar| bar.value),
                    set_from: recorded
                        .first()
                        .map_or_else(Vec::new, |bar| bar.set_from.clone()),
                    current: Some(value),
                    state: if reason.is_some() {
                        BarState::Unassessed
                    } else {
                        BarState::Assessed
                    },
                    reason,
                });
            }
        }
    }
    for bar in &file.bars {
        if !assessments.iter().any(|assessment| {
            assessment.owner == bar.owner
                && assessment.step == bar.step
                && assessment.bound == bar.bound
        }) {
            assessments.push(BarAssessment {
                owner: bar.owner.clone(),
                step: bar.step.clone(),
                bound: bar.bound,
                value: Some(bar.value),
                set_from: bar.set_from.clone(),
                current: None,
                state: BarState::Unassessed,
                reason: Some("recorded bar is absent from today's tree".into()),
            });
        }
    }
    assessments
}
fn observed(
    step: &DecisionStep,
    questions: &NamedQuestionMap,
    answers: &NamedAnswerMap,
) -> Option<f64> {
    match step {
        DecisionStep::AskNoul { question, .. } | DecisionStep::AskScore { question, .. } => {
            sunstack_rules::normalized_answer(question, questions, answers)
        }
        DecisionStep::AskChoice { question, .. } => match answers.get(question)? {
            JudgmentAnswer::Choice {
                selected_label,
                probabilities,
                ..
            } => probabilities.get(selected_label).copied(),
            _ => None,
        },
        DecisionStep::Combine { weights, .. } => {
            weights.iter().try_fold(0.0, |sum, (question, weight)| {
                Some(
                    sum + weight * sunstack_rules::normalized_answer(question, questions, answers)?,
                )
            })
        }
        DecisionStep::Mechanical { .. } => None,
    }
}
pub(super) fn crossings(
    owner: &str,
    path: &[StepName],
    answers: &NamedAnswerMap,
    checked: &CheckedRulebook,
) -> Vec<BarCrossing> {
    let Some((_, graph, questions)) = graphs(checked)
        .into_iter()
        .find(|(name, _, _)| name == &owner)
    else {
        return vec![];
    };
    path.iter()
        .filter_map(|name| graph.steps.get(name).map(|step| (name, step)))
        .flat_map(|(name, step)| {
            let observed = observed(step, questions, answers);
            step_bounds(step)
                .into_iter()
                .map(move |(bound, bar)| BarCrossing {
                    step: name.clone(),
                    bound,
                    bar,
                    observed,
                    crossed: observed.map(|value| match bound {
                        BarBound::NoAtOrBelow => value <= bar,
                        BarBound::YesAtOrAbove | BarBound::MinProbability => value >= bar,
                    }),
                })
        })
        .collect()
}
