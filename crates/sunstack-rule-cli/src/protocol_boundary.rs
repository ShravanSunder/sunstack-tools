use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sunstack_decisions::{
    DecisionRunOutcome, RunJudgmentsInput, judgment_validation::decode_run_input,
};
use sunstack_evidence::{
    DiagnosticList, EvidencePlanInput, EvidencePlanOutcome, decode_evidence_plan_input,
};
use sunstack_rules::{
    RuleInvocationInput, RuleInvocationOutcome, RulePreparationOutcome, decode_rule_invocation,
};
use tokio::io::AsyncReadExt;

pub const MAX_REQUEST_BYTES: u64 = 8_388_608;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope {
    protocol_version: u64,
    operation: String,
    input: Box<RawValue>,
}
pub enum OperationInput {
    PrepareEvidence(Box<EvidencePlanInput>),
    RunJudgments(Box<RunJudgmentsInput>),
    PrepareRules(Box<RuleInvocationInput>),
    EvaluateRules(Box<RuleInvocationInput>),
    Rejected(OperationResult),
}
#[derive(Serialize)]
#[serde(tag = "operation", content = "outcome", rename_all = "snake_case")]
pub enum OperationResult {
    PrepareEvidence(EvidencePlanOutcome),
    RunJudgments(DecisionRunOutcome),
    PrepareRules(Box<RulePreparationOutcome>),
    EvaluateRules(Box<RuleInvocationOutcome>),
}
impl OperationResult {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::PrepareEvidence(outcome) => match outcome {
                EvidencePlanOutcome::Prepared { .. } => 0,
                EvidencePlanOutcome::Refused { .. } => 2,
                EvidencePlanOutcome::Incomplete { .. } => 3,
                EvidencePlanOutcome::Cancelled { .. } => 4,
            },
            Self::RunJudgments(outcome) => outcome.exit_code(),
            Self::PrepareRules(outcome) => outcome.exit_code(),
            Self::EvaluateRules(outcome) => outcome.exit_code(),
        }
    }
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgramResponse {
    ProtocolError {
        protocol_version: u8,
        diagnostics: ProtocolDiagnostics,
    },
    OperationResult {
        protocol_version: u8,
        result: OperationResult,
    },
}
impl ProgramResponse {
    pub fn protocol_error(code: ProtocolCode) -> Self {
        Self::ProtocolError {
            protocol_version: 1,
            diagnostics: ProtocolDiagnostics {
                items: vec![ProtocolDiagnostic {
                    origin: ProtocolOrigin::Protocol,
                    code,
                    location: ProtocolLocation::None,
                    guidance: code.guidance(),
                    retry_advice: ProtocolRetry::RepairRequest,
                }],
                omitted_count: 0,
            },
        }
    }
    pub fn operation_result(result: OperationResult) -> Self {
        Self::OperationResult {
            protocol_version: 1,
            result,
        }
    }
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolCode {
    MalformedJson,
    UnknownField,
    DuplicateField,
    UnsupportedProtocolVersion,
    UnsupportedOperation,
    InvalidFraming,
    InputLimit,
}
impl ProtocolCode {
    fn guidance(self) -> &'static str {
        match self {
            Self::MalformedJson => "Supply one complete JSON request.",
            Self::UnknownField => "Remove fields outside the versioned request envelope.",
            Self::DuplicateField => "Supply each request field once.",
            Self::UnsupportedProtocolVersion => "Use protocol version 1.",
            Self::UnsupportedOperation => "Select a supported program operation.",
            Self::InvalidFraming => "Supply exactly one JSON request object on stdin.",
            Self::InputLimit => {
                "Keep the raw stdin request at or below 8,388,608 bytes; bind evidence by path."
            }
        }
    }
}
#[derive(Serialize)]
pub struct ProtocolDiagnostics {
    items: Vec<ProtocolDiagnostic>,
    omitted_count: usize,
}
#[derive(Serialize)]
struct ProtocolDiagnostic {
    origin: ProtocolOrigin,
    code: ProtocolCode,
    location: ProtocolLocation,
    guidance: &'static str,
    retry_advice: ProtocolRetry,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ProtocolOrigin {
    Protocol,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProtocolLocation {
    None,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProtocolRetry {
    RepairRequest,
}

pub async fn read_frame() -> Result<Vec<u8>, ProtocolCode> {
    // Read one excess byte so an open producer cannot make an over-limit input
    // wait for EOF. No JSON parse runs until this bound has been checked.
    let mut reader = tokio::io::stdin().take(MAX_REQUEST_BYTES + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| ProtocolCode::InvalidFraming)?;
    if reader.limit() == 0 {
        return Err(ProtocolCode::InputLimit);
    }
    Ok(bytes)
}
pub fn decode_request(bytes: &[u8]) -> Result<OperationInput, ProtocolCode> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let raw =
        Box::<RawValue>::deserialize(&mut deserializer).map_err(|_| ProtocolCode::MalformedJson)?;
    deserializer
        .end()
        .map_err(|_| ProtocolCode::InvalidFraming)?;
    if !raw.get().starts_with('{') {
        return Err(ProtocolCode::InvalidFraming);
    }
    let envelope: RequestEnvelope = serde_json::from_str(raw.get()).map_err(|error| {
        // Classify decoder-owned errors; never copy received keys or values.
        let message = error.to_string();
        if message.starts_with("duplicate field") {
            ProtocolCode::DuplicateField
        } else if message.starts_with("unknown field") {
            ProtocolCode::UnknownField
        } else {
            ProtocolCode::InvalidFraming
        }
    })?;
    if envelope.protocol_version != 1 {
        return Err(ProtocolCode::UnsupportedProtocolVersion);
    }
    match envelope.operation.as_str() {
        "prepare_evidence" => Ok(
            match decode_evidence_plan_input(envelope.input.get().as_bytes()) {
                Ok(input) => OperationInput::PrepareEvidence(Box::new(input)),
                Err(diagnostic) => OperationInput::Rejected(OperationResult::PrepareEvidence(
                    EvidencePlanOutcome::Refused {
                        diagnostics: DiagnosticList {
                            items: vec![*diagnostic],
                            omitted_count: 0,
                        },
                    },
                )),
            },
        ),
        "run_judgments" => Ok(match decode_run_input(envelope.input.get().as_bytes()) {
            Ok(input) => OperationInput::RunJudgments(Box::new(input)),
            Err(diagnostic) => OperationInput::Rejected(OperationResult::RunJudgments(
                DecisionRunOutcome::InvalidRequest {
                    diagnostics: diagnostic.into(),
                },
            )),
        }),
        "prepare_rules" => Ok(
            match decode_rule_invocation(envelope.input.get().as_bytes()) {
                Ok(input) => OperationInput::PrepareRules(Box::new(input)),
                Err(diagnostics) => OperationInput::Rejected(OperationResult::PrepareRules(
                    Box::new(RulePreparationOutcome::Invalid { diagnostics }),
                )),
            },
        ),
        "evaluate_rules" => Ok(
            match decode_rule_invocation(envelope.input.get().as_bytes()) {
                Ok(input) => OperationInput::EvaluateRules(Box::new(input)),
                Err(diagnostics) => OperationInput::Rejected(OperationResult::EvaluateRules(
                    Box::new(RuleInvocationOutcome::Invalid { diagnostics }),
                )),
            },
        ),
        _ => Err(ProtocolCode::UnsupportedOperation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sunstack_rules::{InvocationMeasurements, RuleDiagnosticCode};

    #[test]
    fn internal_preparation_failures_keep_native_diagnostics_measurements_and_exit_three() {
        // These native invariant/fatal outcomes have no public-input fixture.
        // This checks the serialization boundary, not process reachability.
        let diagnostics =
            sunstack_rules::DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError);
        let measurements = InvocationMeasurements {
            files_read: 7,
            ..InvocationMeasurements::default()
        };
        for (operation, result) in [
            (
                "prepare_rules",
                OperationResult::PrepareRules(Box::new(
                    RulePreparationOutcome::PreparationFailed {
                        diagnostics: diagnostics.clone(),
                        measurements: measurements.clone(),
                    },
                )),
            ),
            (
                "evaluate_rules",
                OperationResult::EvaluateRules(Box::new(
                    RuleInvocationOutcome::PreparationFailed {
                        diagnostics: diagnostics.clone(),
                        measurements: measurements.clone(),
                    },
                )),
            ),
        ] {
            assert_eq!(result.exit_code(), 3);
            let wire = serde_json::to_value(ProgramResponse::operation_result(result));
            assert!(wire.is_ok());
            let expected = json!({
                "kind":"operation_result", "protocol_version":1,
                "result":{"operation":operation,"outcome":{
                    "kind":"preparation_failed", "diagnostics":diagnostics,
                    "measurements":measurements
                }}
            });
            assert_eq!(wire.ok(), Some(expected));
        }
    }
}
