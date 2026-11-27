use crate::operation_protocol::protocol_boundary::{OperationInput, OperationResult};
use sunstack::host_services::general_rules;
use sunstack_decisions::{
    BackendPreparation, CancellationToken, DecisionRunOutcome, JevOpenRouterBackend,
};
pub async fn execute_operation(
    input: OperationInput,
    cancellation: &CancellationToken,
) -> OperationResult {
    match input {
        OperationInput::Rejected(result) => result,
        OperationInput::CheckRules(input) => OperationResult::CheckRules(Box::new(
            sunstack_rules::check_rules(*input, cancellation.clone()).await,
        )),
        OperationInput::PrepareEvidence(input) => OperationResult::PrepareEvidence(
            general_rules::inspect_evidence(*input, cancellation.clone()).await,
        ),
        OperationInput::RunJudgments(input) => {
            let credential = sunstack::host_services::provider_credentials::resolve().await;
            let outcome = match JevOpenRouterBackend::prepare(
                input.backend.clone(),
                credential.as_deref().unwrap_or_default(),
            ) {
                BackendPreparation::Ready { backend } => {
                    sunstack_decisions::run_judgments(&input, &backend, cancellation).await
                }
                BackendPreparation::Refused { mut diagnostics } => {
                    if credential.is_none() {
                        for diagnostic in &mut diagnostics.items {
                            diagnostic.guidance =
                                sunstack::host_services::provider_credentials::MISSING_CREDENTIAL
                                    .into();
                        }
                    }
                    DecisionRunOutcome::InvalidRequest { diagnostics }
                }
            };
            OperationResult::RunJudgments(outcome)
        }
        OperationInput::PrepareRules(input) => OperationResult::PrepareRules(Box::new(
            general_rules::prepare(*input, cancellation.clone()).await,
        )),
        OperationInput::EvaluateRules(input) => OperationResult::EvaluateRules(Box::new(
            general_rules::evaluate(*input, cancellation.clone()).await,
        )),
    }
}
