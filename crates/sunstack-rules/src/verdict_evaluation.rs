use crate::*;
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentAnswer, JudgmentQuestion, QuestionName};
#[derive(Debug, Clone)]
pub struct ReferencedAnswer {
    pub answer: JudgmentAnswer,
    pub reference: JudgmentReference,
}
#[derive(Debug, Clone)]
pub struct VerdictAnswers {
    pub noul_verdict: Option<ReferencedAnswer>,
    pub informational: BTreeMap<QuestionName, ReferencedAnswer>,
}
/// Verdict policy is applied only after a completed request; native answers remain in that request.
pub fn evaluate_verdict(
    rule: &RuleDefinition,
    answers: &VerdictAnswers,
) -> Result<VerdictOutcome, DiagnosticList> {
    let invalid = || DiagnosticList::rule(RuleDiagnosticCode::InvalidComposite);
    let (value, basis) = match &rule.verdict {
        VerdictDeclaration::Mechanical { .. } => return Err(invalid()),
        VerdictDeclaration::Noul { .. } => {
            let answer = answers.noul_verdict.as_ref().ok_or_else(invalid)?;
            let JudgmentAnswer::Noul { yes_probability } = &answer.answer else {
                return Err(invalid());
            };
            (
                *yes_probability,
                VerdictBasis::Noul {
                    answer: answer.reference.clone(),
                },
            )
        }
        VerdictDeclaration::Composite { dimensions, .. } => {
            let mut results = Vec::new();
            let mut value = 0.0;
            for dimension in dimensions {
                let answer = answers
                    .informational
                    .get(&dimension.question)
                    .ok_or_else(invalid)?;
                let normalized_value =
                    match (&answer.answer, rule.questions.get(&dimension.question)) {
                        (
                            JudgmentAnswer::Noul { yes_probability },
                            Some(JudgmentQuestion::Noul { .. }),
                        ) => *yes_probability,
                        (
                            JudgmentAnswer::Score { score, .. },
                            Some(JudgmentQuestion::Score { criteria, .. }),
                        ) if criteria.len() > 1 => score / (criteria.len() - 1) as f64,
                        _ => return Err(invalid()),
                    };
                if !normalized_value.is_finite() || !(0.0..=1.0).contains(&normalized_value) {
                    return Err(invalid());
                }
                value += normalized_value * dimension.weight;
                results.push(DimensionResult {
                    question: dimension.question.clone(),
                    weight: dimension.weight,
                    normalized_value,
                    answer: answer.reference.clone(),
                });
            }
            (
                value,
                VerdictBasis::Composite {
                    dimensions: results,
                },
            )
        }
    };
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(invalid());
    }
    let threshold = rule.verdict.threshold().ok_or_else(invalid)?;
    let result = if value >= threshold {
        VerdictResult::Pass
    } else {
        let activation = match (rule.severity, rule.verdict.band()) {
            (Severity::Warn, _) => Activation::Warn,
            (Severity::Error, Some(band)) if value > band => Activation::WarnAboveBand { band },
            (Severity::Error, _) => Activation::Error,
        };
        VerdictResult::Fail { activation }
    };
    Ok(VerdictOutcome {
        basis,
        value,
        threshold,
        result,
    })
}
