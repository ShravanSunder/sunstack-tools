use crate::{
    FilterAnswerState, FilterAnswers, FilterName, NotApplicableReason, RuleId, RulebookRule,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use sunstack_decisions::ChoiceLabel;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FilterSkipReason {
    Unsure,
    Incomplete,
    NoFolder,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterSkip {
    pub rule: RuleId,
    pub file: PathBuf,
    pub filter: FilterName,
    pub folder: Option<PathBuf>,
    pub why: FilterSkipReason,
    pub probabilities: Option<BTreeMap<ChoiceLabel, f64>>,
}
pub enum GateOutcome {
    Walk,
    Exclude(NotApplicableReason),
    Skip(FilterSkip),
}
pub fn gate_for(rule: &RulebookRule, answers: &FilterAnswers, file: &Path) -> GateOutcome {
    let Some(gate) = &rule.gate else {
        return GateOutcome::Walk;
    };
    let folder = answers.folders.get(&(gate.filter.clone(), file.to_owned()));
    let mut skip = FilterSkip {
        rule: rule.name.clone(),
        file: file.into(),
        filter: gate.filter.clone(),
        folder: folder.cloned(),
        why: FilterSkipReason::NoFolder,
        probabilities: None,
    };
    let Some(folder) = folder else {
        return GateOutcome::Skip(skip);
    };
    let answer = answers
        .entries
        .iter()
        .find(|answer| answer.filter == gate.filter && &answer.folder == folder);
    match answer.map(|answer| &answer.state) {
        Some(FilterAnswerState::Judged {
            answer,
            probability,
            ..
        }) => {
            if gate.answers.contains(answer) {
                GateOutcome::Walk
            } else {
                GateOutcome::Exclude(NotApplicableReason::Filter {
                    filter: gate.filter.clone(),
                    answer: answer.clone(),
                    probability: *probability,
                })
            }
        }
        Some(FilterAnswerState::Unsure { probabilities, .. }) => {
            skip.why = FilterSkipReason::Unsure;
            skip.probabilities = Some(probabilities.clone());
            GateOutcome::Skip(skip)
        }
        Some(FilterAnswerState::Incomplete { .. }) | None => {
            skip.why = FilterSkipReason::Incomplete;
            GateOutcome::Skip(skip)
        }
    }
}
