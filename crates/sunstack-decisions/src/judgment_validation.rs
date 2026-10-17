use crate::{decision_contracts::*, decision_diagnostics::*};
use serde::Deserialize;
use std::collections::BTreeSet;

pub fn validate_request(request: &StructuredJudgmentRequest) -> Result<(), DecisionDiagnostic> {
    if request.questions.is_empty() {
        return Err(DecisionDiagnostic::request(
            DecisionDiagnosticCode::InvalidQuestion,
        ));
    }
    for question in request.questions.values() {
        match question {
            JudgmentQuestion::Choice { criteria, .. } if criteria.is_empty() => {
                return Err(DecisionDiagnostic::request(
                    DecisionDiagnosticCode::InvalidChoice,
                ));
            }
            JudgmentQuestion::Score { criteria, .. } if criteria.is_empty() => {
                return Err(DecisionDiagnostic::request(
                    DecisionDiagnosticCode::InvalidRubric,
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn decode_run_input(bytes: &[u8]) -> Result<RunJudgmentsInput, DecisionDiagnostic> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let decoded = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let message = error.inner().to_string();
        let code = if message.starts_with("unknown field") {
            DecisionDiagnosticCode::UnknownField
        } else if message.starts_with("duplicate field") {
            DecisionDiagnosticCode::DuplicateField
        } else {
            DecisionDiagnosticCode::InvalidQuestion
        };
        let mut diagnostic = DecisionDiagnostic::request(code);
        // Include only schema-owned segments; dynamic author keys never enter diagnostics.
        let mut safe_path = Vec::new();
        for segment in error.path() {
            if let serde_path_to_error::Segment::Map { key } = segment {
                if [
                    "request",
                    "questions",
                    "state",
                    "backend",
                    "attempts",
                    "request_budget",
                    "instructions",
                    "criteria",
                ]
                .contains(&key.as_str())
                {
                    safe_path.push(key.clone());
                } else {
                    break;
                }
            }
        }
        diagnostic.location = DecisionDiagnosticLocation::Field {
            field_path: safe_path.into_iter().take(32).collect(),
        };
        diagnostic
    })?;
    deserializer
        .end()
        .map_err(|_| DecisionDiagnostic::request(DecisionDiagnosticCode::InvalidBody))?;
    Ok(decoded)
}

pub fn validate_response(
    request: &StructuredJudgmentRequest,
    answers: NamedAnswerMap,
    receipt: ExecutionReceipt,
) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
    let failure = |code| Err(DecisionDiagnostic::response(code));
    if answers
        .keys()
        .any(|name| !request.questions.contains_key(name))
    {
        return failure(DecisionDiagnosticCode::UnexpectedAnswer);
    }
    if request
        .questions
        .keys()
        .any(|name| !answers.contains_key(name))
    {
        return failure(DecisionDiagnosticCode::MissingAnswer);
    }
    if receipt
        .usage
        .cost_usd
        .is_some_and(|cost| !cost.is_finite() || cost < 0.0)
    {
        return failure(DecisionDiagnosticCode::InvalidReceipt);
    }
    for (name, question) in &request.questions {
        let Some(answer) = answers.get(name) else {
            return failure(DecisionDiagnosticCode::MissingAnswer);
        };
        match (question, answer) {
            (JudgmentQuestion::Noul { .. }, JudgmentAnswer::Noul { yes_probability }) => {
                if !probability(*yes_probability) {
                    return failure(DecisionDiagnosticCode::InvalidNumber);
                }
            }
            (
                JudgmentQuestion::Choice { criteria, .. },
                JudgmentAnswer::Choice {
                    selected_label,
                    confidence,
                    probabilities,
                },
            ) => {
                if !criteria.contains_key(selected_label)
                    || !criteria.keys().eq(probabilities.keys())
                {
                    return failure(DecisionDiagnosticCode::InvalidChoice);
                }
                if !probability(*confidence)
                    || !probabilities.values().all(|value| probability(*value))
                {
                    return failure(DecisionDiagnosticCode::InvalidNumber);
                }
            }
            (
                JudgmentQuestion::Score { criteria, .. },
                JudgmentAnswer::Score {
                    score,
                    confidence,
                    legend,
                    probabilities,
                },
            ) => {
                let indices: BTreeSet<String> =
                    (0..criteria.len()).map(|index| index.to_string()).collect();
                if legend.keys().cloned().collect::<BTreeSet<_>>() != indices
                    || probabilities.keys().cloned().collect::<BTreeSet<_>>() != indices
                {
                    return failure(DecisionDiagnosticCode::InvalidRubric);
                }
                if !score.is_finite()
                    || *score < 0.0
                    || *score > criteria.len().saturating_sub(1) as f64
                    || !probability(*confidence)
                    || !probabilities.values().all(|value| probability(*value))
                {
                    return failure(DecisionDiagnosticCode::InvalidNumber);
                }
            }
            _ => return failure(DecisionDiagnosticCode::WrongAnswerKind),
        }
    }
    Ok((answers, receipt))
}
fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

// Provider wire names differ from the public canonical contract. This private
// representation exists only at the HTTP response boundary.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum JevAnswer {
    Noul {
        noul: f64,
    },
    Choice {
        choice: ChoiceLabel,
        confidence: f64,
        #[serde(deserialize_with = "unique_map")]
        probabilities: std::collections::BTreeMap<ChoiceLabel, f64>,
    },
    Score {
        score: f64,
        confidence: f64,
        #[serde(deserialize_with = "unique_map")]
        legend: std::collections::BTreeMap<String, StructuredEntry>,
        #[serde(deserialize_with = "unique_map")]
        probabilities: std::collections::BTreeMap<String, f64>,
    },
}
impl From<JevAnswer> for JudgmentAnswer {
    fn from(answer: JevAnswer) -> Self {
        match answer {
            JevAnswer::Noul { noul } => Self::Noul {
                yes_probability: noul,
            },
            JevAnswer::Choice {
                choice,
                confidence,
                probabilities,
            } => Self::Choice {
                selected_label: choice,
                confidence,
                probabilities,
            },
            JevAnswer::Score {
                score,
                confidence,
                legend,
                probabilities,
            } => Self::Score {
                score,
                confidence,
                legend,
                probabilities,
            },
        }
    }
}
#[derive(Deserialize)]
struct JevEnvelope {
    model: ModelName,
    #[serde(deserialize_with = "unique_map")]
    answers: std::collections::BTreeMap<QuestionName, JevAnswer>,
    usage: JevUsage,
}
#[derive(Deserialize)]
struct JevUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default, deserialize_with = "present_cost")]
    cost: Option<f64>,
}
fn present_cost<'de, TDeserializer: serde::Deserializer<'de>>(
    deserializer: TDeserializer,
) -> Result<Option<f64>, TDeserializer::Error> {
    f64::deserialize(deserializer).map(Some)
}

pub(crate) fn decode_jev_response(
    request: &StructuredJudgmentRequest,
    bytes: &[u8],
) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
    // Decode once. Only schema-owned receipt paths affect classification;
    // provider error text and dynamic answer names never leave this boundary.
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope: JevEnvelope = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let receipt_path = matches!(error.path().iter().next(), Some(serde_path_to_error::Segment::Map { key }) if key == "model" || key == "usage");
        let message = error.inner().to_string();
        let missing_receipt = message.starts_with("missing field `model`") || message.starts_with("missing field `usage`");
        let code = if error.inner().is_data() && (receipt_path || missing_receipt) { DecisionDiagnosticCode::InvalidReceipt } else { DecisionDiagnosticCode::InvalidBody };
        DecisionDiagnostic::response(code)
    })?;
    deserializer
        .end()
        .map_err(|_| DecisionDiagnostic::response(DecisionDiagnosticCode::InvalidBody))?;
    let receipt = ExecutionReceipt {
        resolved_model: envelope.model,
        usage: ExecutionUsage {
            input_tokens: envelope.usage.input_tokens,
            output_tokens: envelope.usage.output_tokens,
            cost_usd: envelope.usage.cost,
        },
    };
    let answers = envelope
        .answers
        .into_iter()
        .map(|(name, answer)| (name, answer.into()))
        .collect();
    validate_response(request, answers, receipt)
}
