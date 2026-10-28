use crate::*;
use std::collections::BTreeMap;
use sunstack_decisions::QuestionName;

/// Preserve native operand answers; only the logical verdict carries aggregate truth.
pub(crate) fn project_verdict(
    judgment: &RuleJudgment,
    answers: &mut Vec<AnswerRecord>,
    references: &BTreeMap<(usize, QuestionName), AnswerId>,
) -> Result<CandidateOutcome, DiagnosticList> {
    let lookup = |reference: &JudgmentReference| {
        references
            .get(&(reference.request_index, reference.question_name.clone()))
            .cloned()
    };
    let used = match &judgment.verdict.basis {
        VerdictBasis::Noul { answer } => lookup(answer).into_iter().collect::<Vec<_>>(),
        VerdictBasis::Composite { dimensions } => dimensions
            .iter()
            .filter_map(|dimension| lookup(&dimension.answer))
            .collect(),
    };
    let uncertain = answers.iter().any(|answer| {
        used.contains(&answer.answer_id)
            && matches!(answer.settlement, QuestionSettlement::Uncertain)
    });
    let logical = match &judgment.verdict.basis {
        VerdictBasis::Noul { .. } => used.first().cloned(),
        VerdictBasis::Composite { .. } => {
            let answer_id = AnswerId(
                sunstack_evidence::DeclarationId::new("verdict")
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
            );
            answers.push(AnswerRecord {
                answer_id: answer_id.clone(),
                question: QuestionRef::Verdict {},
                page: None,
                answer: None,
                settlement: if uncertain {
                    QuestionSettlement::Uncertain
                } else {
                    match judgment.verdict.result {
                        VerdictResult::Pass => QuestionSettlement::Passed,
                        VerdictResult::Fail { .. } => QuestionSettlement::DidNotPass,
                    }
                },
            });
            Some(answer_id)
        }
    };
    if uncertain {
        return Ok(CandidateOutcome::Uncertain {
            at: QuestionRef::Verdict {},
            answers: used,
        });
    }
    Ok(match (&judgment.verdict.result, logical) {
        (VerdictResult::Pass, Some(_)) => CandidateOutcome::Finding {
            path: answers
                .iter()
                .filter(|answer| {
                    matches!(answer.settlement, QuestionSettlement::Passed)
                        && !matches!(answer.question, QuestionRef::Informational { .. })
                })
                .map(|answer| answer.answer_id.clone())
                .collect(),
        },
        (VerdictResult::Fail { .. }, Some(answer)) => CandidateOutcome::Dropped {
            at: QuestionRef::Verdict {},
            answer,
        },
        (_, None) => CandidateOutcome::NotProcessed {
            reason: NotProcessedReason::EvaluationFailed,
        },
    })
}
