use super::*;
use crate::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EvaluationIndex(pub usize);

pub(crate) struct PlannedRuleEvaluation {
    pub key: RuleEvaluationKey,
    pub rule: Arc<RuleDefinition>,
    pub group: GroupIndex,
    pub entries: Vec<EntryIndex>,
    pub scope: ScopeEvaluation,
    pub authored_order: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum ExpansionState {
    OutsideChangeScope,
    Waiting,
    Expanded {
        children: Vec<OccurrenceIndex>,
    },
    ZeroMatches {
        source: sunstack_evidence::SourceRef,
    },
    SelectionFailed {
        diagnostics: DiagnosticList,
    },
    Stopped {
        entry: EntryIndex,
    },
    Failed {
        cause: FailureCause,
    },
    Cancelled,
}

pub(crate) fn occurrence_order(occurrence: &PlannedOccurrence) -> (usize, usize, usize, usize) {
    let (unit, matched, visible) = match occurrence.key.target.as_ref() {
        RuleOccurrenceTarget::MarkdownUnit { range, .. } => (range.start, 0, 0),
        RuleOccurrenceTarget::CallSite {
            unit_range,
            match_range,
            visible_range,
            ..
        } => (unit_range.start, match_range.start, visible_range.start),
        _ => (0, 0, 0),
    };
    (occurrence.authored_order, unit, matched, visible)
}
