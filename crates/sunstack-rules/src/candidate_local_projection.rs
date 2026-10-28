use crate::*;
use sunstack_evidence::{DeclarationId, SourceName};

pub(crate) fn project_local_verdict(
    outcome: &LocalSettlement,
    answers: &mut Vec<AnswerRecord>,
    source: &SourceName,
) -> Result<CandidateOutcome, DiagnosticList> {
    match outcome {
        LocalSettlement::Completed { result, .. } => {
            let answer_id = AnswerId(
                DeclarationId::new("local_verdict")
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
            );
            let passed = matches!(result, MechanicalResult::Pass);
            answers.push(AnswerRecord {
                answer_id: answer_id.clone(),
                question: QuestionRef::Verdict {},
                page: None,
                answer: None,
                settlement: if passed {
                    QuestionSettlement::Passed
                } else {
                    QuestionSettlement::DidNotPass
                },
            });
            Ok(if passed {
                CandidateOutcome::Dropped {
                    at: QuestionRef::Verdict {},
                    answer: answer_id,
                }
            } else {
                CandidateOutcome::Finding {
                    path: answers
                        .iter()
                        .filter(|answer| {
                            answer.answer_id == answer_id
                                || (matches!(answer.settlement, QuestionSettlement::Passed)
                                    && !matches!(
                                        answer.question,
                                        QuestionRef::Informational { .. }
                                    ))
                        })
                        .map(|answer| answer.answer_id.clone())
                        .collect(),
                }
            })
        }
        LocalSettlement::Unresolved { diagnostics } => {
            let diagnostic = diagnostics.items.iter().find_map(|item| match item {
                ReportDiagnostic::Evidence(diagnostic) => Some(diagnostic.code),
                _ => None,
            });
            Ok(match diagnostic {
                Some(diagnostic) => CandidateOutcome::Unresolved {
                    at: QuestionRef::Verdict {},
                    missing: UnresolvedCause::EvidenceNotCaptured {
                        evidence: source.clone(),
                        diagnostic,
                    },
                },
                None => CandidateOutcome::NotProcessed {
                    reason: NotProcessedReason::EvaluationFailed,
                },
            })
        }
        LocalSettlement::Cancelled => Ok(CandidateOutcome::NotProcessed {
            reason: NotProcessedReason::RunStopped,
        }),
    }
}
