use crate::*;

pub(crate) struct QuestionContextFailure {
    pub at: QuestionRef,
    pub pages: u32,
}

pub(crate) fn apply_whole_context_failure(
    entry: &mut CandidateEntry,
    failures: &[QuestionContextFailure],
) {
    if !matches!(
        entry.outcome,
        CandidateOutcome::NotProcessed {
            reason: NotProcessedReason::Refused
        }
    ) {
        return;
    }
    let Some(failure) = failures.iter().min_by_key(|failure| match failure.at {
        QuestionRef::Router {} => (0, 0),
        QuestionRef::RuleSetGate { position } => (1, position),
        QuestionRef::Narrowing {} => (2, 0),
        QuestionRef::RuleGate { position } => (3, position),
        QuestionRef::Verdict {} | QuestionRef::Informational { .. } => (4, 0),
    }) else {
        return;
    };
    entry.outcome = CandidateOutcome::Unresolved {
        at: failure.at.clone(),
        missing: UnresolvedCause::NeedsWholeContext {
            pages: failure.pages,
        },
    };
}
