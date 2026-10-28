use crate::{candidate_execution_data::QuestionEvidenceFailure, *};
use std::collections::BTreeMap;
use sunstack_evidence::SourceName;

pub(crate) fn apply_evidence_failure(
    entry: &mut CandidateEntry,
    failures: &[QuestionEvidenceFailure],
    unavailable: &BTreeMap<SourceName, UnavailableReason>,
) -> Result<(), DiagnosticList> {
    if !matches!(
        entry.outcome,
        CandidateOutcome::NotProcessed {
            reason: NotProcessedReason::Refused
        }
    ) {
        return Ok(());
    }
    let Some(failure) = failures.iter().min_by_key(|failure| match failure.at {
        QuestionRef::Router {} => (0, 0),
        QuestionRef::RuleSetGate { position } => (1, position),
        QuestionRef::RuleGate { position } => (2, position),
        QuestionRef::Verdict {} | QuestionRef::Informational { .. } => (3, 0),
    }) else {
        return Ok(());
    };
    let missing = match unavailable.get(&failure.input) {
        Some(reason) => UnresolvedCause::EvidenceUnavailable {
            evidence: failure.input.clone(),
            reason: *reason,
        },
        None => match failure.diagnostic {
            Some(diagnostic) => UnresolvedCause::EvidenceNotCaptured {
                evidence: failure.input.clone(),
                diagnostic,
            },
            None => return Ok(()),
        },
    };
    entry.outcome = CandidateOutcome::Unresolved {
        at: failure.at.clone(),
        missing,
    };
    if !entry
        .answers
        .iter()
        .any(|answer| answer.question == failure.at)
    {
        entry.answers.push(AnswerRecord {
            answer_id: AnswerId(
                sunstack_evidence::DeclarationId::new("evidence_unresolved")
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
            ),
            question: failure.at.clone(),
            page: None,
            answer: None,
            settlement: QuestionSettlement::Unresolved,
        });
    }
    Ok(())
}
