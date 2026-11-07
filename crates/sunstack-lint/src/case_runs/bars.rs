use super::contracts::*;
use crate::configuration_loading::config_validation::CheckedRulebook;
use std::collections::BTreeMap;
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BarOwner<'a> {
    Rule(&'a sunstack_rules::RuleId),
    Filter(&'a sunstack_rules::FilterName),
}
impl<'a> BarOwner<'a> {
    fn name(self) -> &'a str {
        match self {
            Self::Rule(rule) => rule.as_str(),
            Self::Filter(filter) => filter.as_str(),
        }
    }
    fn owns(self, case: &LintCase) -> bool {
        match (self, &case.subject) {
            (Self::Rule(owner), CaseSubject::Rule { rule, .. }) => owner == rule,
            (Self::Filter(owner), CaseSubject::Filter { filter, .. }) => owner == filter,
            _ => false,
        }
    }
}
fn graphs(checked: &CheckedRulebook) -> Vec<(BarOwner<'_>, &DecisionGraph, &NamedQuestionMap)> {
    checked
        .assembly
        .rules
        .iter()
        .map(|rule| (BarOwner::Rule(&rule.name), &rule.decision, &rule.questions))
        .chain(checked.filters.values().map(|filter| {
            (
                BarOwner::Filter(&filter.name),
                &filter.decision,
                &filter.questions,
            )
        }))
        .collect()
}
fn provenance_reason(
    bar: &RecordedBar,
    owner: BarOwner<'_>,
    cases: &BTreeMap<&str, &LintCase>,
) -> Option<String> {
    if bar.set_from.is_empty() {
        return Some("bar has no recorded development cases".into());
    }
    bar.set_from
        .iter()
        .find_map(|id| match cases.get(id.as_str()) {
            None => Some(format!("bar references unknown case {id}")),
            Some(case) if case.set != "development" => {
                Some(format!("case {id} is not a development case"))
            }
            Some(case) if !owner.owns(case) => {
                Some(format!("case {id} belongs to a different owner"))
            }
            Some(_) => None,
        })
}
pub(super) fn assess(file: &CaseFile, checked: &CheckedRulebook) -> Vec<BarAssessment> {
    let cases = file
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let mut assessments = Vec::new();
    for (owner, graph, _) in graphs(checked) {
        for (step, definition) in &graph.steps {
            for (bound, value) in step_bounds(definition) {
                let recorded = file
                    .bars
                    .iter()
                    .filter(|bar| {
                        bar.owner == owner.name() && &bar.step == step && bar.bound == bound
                    })
                    .collect::<Vec<_>>();
                let reason = match recorded.as_slice() {
                    [] => Some("bar has no recorded development cases".into()),
                    [bar] if bar.value != value => Some("bar changed: unassessed".into()),
                    [bar] => provenance_reason(bar, owner, &cases),
                    _ => Some("bar has multiple records".into()),
                };
                assessments.push(BarAssessment {
                    owner: owner.name().into(),
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
    owner: BarOwner<'_>,
    path: &[StepName],
    answers: &NamedAnswerMap,
    checked: &CheckedRulebook,
) -> Vec<BarCrossing> {
    let Some((_, graph, questions)) = graphs(checked)
        .into_iter()
        .find(|(name, _, _)| *name == owner)
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

#[cfg(test)]
#[path = "bars_test.rs"]
mod tests;
