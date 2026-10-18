use crate::protocol_boundary::{OperationInput, OperationResult};
use std::sync::Arc;
use sunstack_decisions::{
    BackendPreparation, CancellationToken, DecisionRunOutcome, JevOpenRouterBackend,
};
use sunstack_evidence::{AdapterRegistry, EvidencePlanOutcome};
use sunstack_rules::{
    RuleExecutionContext, RuleInvocationInput, RuleInvocationOutcome, RulePreparationOutcome,
};

fn rule_context(
    input: &RuleInvocationInput,
    cancellation: &CancellationToken,
) -> Result<RuleExecutionContext, sunstack_rules::DiagnosticList> {
    let credential = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
    let backend = match JevOpenRouterBackend::prepare(input.backend.clone(), &credential) {
        BackendPreparation::Ready { backend } => backend,
        BackendPreparation::Refused { diagnostics } => return Err(diagnostics.into()),
    };
    let adapters = AdapterRegistry::builtins()
        .map_err(|error| sunstack_rules::DiagnosticList::from(error.diagnostics()))?;
    Ok(RuleExecutionContext {
        backend: Arc::new(backend),
        adapters,
        cancellation: cancellation.clone(),
    })
}

pub async fn execute_operation(
    input: OperationInput,
    cancellation: &CancellationToken,
) -> OperationResult {
    match input {
        OperationInput::Rejected(result) => result,
        OperationInput::CheckRules(input) => OperationResult::CheckRules(Box::new(
            sunstack_rules::check_rules(*input, cancellation.clone()).await,
        )),
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
        OperationInput::PrepareRules(input) => {
            let outcome = match rule_context(&input, cancellation) {
                Ok(context) => sunstack_rules::prepare_rules(*input, context).await,
                Err(diagnostics) => RulePreparationOutcome::Invalid { diagnostics },
            };
            OperationResult::PrepareRules(Box::new(outcome))
        }
        OperationInput::EvaluateRules(input) => {
            let outcome = match rule_context(&input, cancellation) {
                Ok(context) => sunstack_rules::evaluate_rules(*input, context).await,
                Err(diagnostics) => RuleInvocationOutcome::Invalid { diagnostics },
            };
            OperationResult::EvaluateRules(Box::new(outcome))
        }
    }
}
