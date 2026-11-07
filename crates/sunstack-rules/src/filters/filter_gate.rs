use crate::{
    FilterAnswerState, FilterAnswers, FilterName, NotApplicableReason, RuleId, RulebookRule,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FilterSkipReason {
    Unsure,
    Incomplete,
    NoFolder,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterSkip {
    pub locations: Vec<sunstack_evidence::PieceLocation>,
    pub rule: RuleId,
    pub file: PathBuf,
    pub filter: FilterName,
    pub folder: Option<PathBuf>,
    pub why: FilterSkipReason,
    pub path: Vec<crate::StepName>,
    pub answers: sunstack_decisions::NamedAnswerMap,
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
        locations: vec![],
        rule: rule.name.clone(),
        file: file.into(),
        filter: gate.filter.clone(),
        folder: folder.cloned(),
        why: FilterSkipReason::NoFolder,
        path: vec![],
        answers: BTreeMap::new(),
    };
    let Some(folder) = folder else {
        return GateOutcome::Skip(skip);
    };
    let answer = answers
        .entries
        .iter()
        .find(|answer| answer.filter == gate.filter && &answer.folder == folder);
    match answer.map(|answer| &answer.state) {
        Some(FilterAnswerState::Judged { label, .. }) => {
            if gate.answers.contains(label) {
                GateOutcome::Walk
            } else {
                GateOutcome::Exclude(NotApplicableReason::Filter {
                    filter: gate.filter.clone(),
                    label: label.clone(),
                })
            }
        }
        Some(FilterAnswerState::Unsure { path, answers, .. }) => {
            skip.why = FilterSkipReason::Unsure;
            skip.path = path.clone();
            skip.answers = answers.clone();
            GateOutcome::Skip(skip)
        }
        Some(FilterAnswerState::Incomplete { path, answers, .. }) => {
            skip.path = path.clone();
            skip.answers = answers.clone();
            skip.why = FilterSkipReason::Incomplete;
            GateOutcome::Skip(skip)
        }
        None => {
            skip.why = FilterSkipReason::Incomplete;
            GateOutcome::Skip(skip)
        }
    }
}
