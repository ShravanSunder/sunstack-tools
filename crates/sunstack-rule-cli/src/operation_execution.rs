use crate::protocol_boundary::{OperationInput, OperationResult};
use sunstack_decisions::{
    BackendPreparation, CancellationToken, DecisionRunOutcome, JevOpenRouterBackend,
};
use sunstack_evidence::{AdapterRegistry, EvidencePlanOutcome};

pub async fn execute_operation(
    input: OperationInput,
    cancellation: &CancellationToken,
) -> OperationResult {
    match input {
        OperationInput::Rejected(result) => result,
        OperationInput::PrepareEvidence(input) => {
            OperationResult::PrepareEvidence(match AdapterRegistry::builtins() {
                Ok(registry) => {
                    sunstack_evidence::prepare_evidence(*input, registry, cancellation.clone())
                        .await
                }
                Err(error) => EvidencePlanOutcome::Refused {
                    diagnostics: error.diagnostics(),
                },
            })
        }
        OperationInput::RunJudgments(input) => {
            let credential = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
            let outcome = match JevOpenRouterBackend::prepare(input.backend.clone(), &credential) {
                BackendPreparation::Ready { backend } => {
                    sunstack_decisions::run_judgments(&input, &backend, cancellation).await
                }
                BackendPreparation::Refused { diagnostics } => {
                    DecisionRunOutcome::InvalidRequest { diagnostics }
                }
            };
            OperationResult::RunJudgments(outcome)
        }
    }
}
