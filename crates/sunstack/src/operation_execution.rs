use crate::protocol_boundary::{OperationInput, OperationResult};
use std::sync::Arc;
use sunstack_decisions::{
    BackendPreparation, CancellationToken, DecisionRunOutcome, JevOpenRouterBackend,
};
use sunstack_evidence::{AdapterRegistry, EvidencePlanOutcome};
use sunstack_rules::{
    RuleBackendFactory, RuleExecutionContext, RuleInvocationOutcome, RulePreparationOutcome,
};

struct JevRuleBackendFactory {
    selection: sunstack_decisions::BackendSelection,
}
impl RuleBackendFactory for JevRuleBackendFactory {
    fn selection(&self) -> &sunstack_decisions::BackendSelection {
        &self.selection
    }
    fn prepare(
        self: Arc<Self>,
    ) -> Result<Arc<dyn sunstack_decisions::DecisionBackend>, sunstack_rules::DiagnosticList> {
        let credential = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        match JevOpenRouterBackend::prepare(self.selection.clone(), &credential) {
            BackendPreparation::Ready { backend } => Ok(Arc::new(backend)),
            BackendPreparation::Refused { diagnostics } => Err(diagnostics.into()),
        }
    }
}

fn rule_context(
    backend: &sunstack_decisions::BackendSelection,
    cancellation: &CancellationToken,
) -> Result<RuleExecutionContext, sunstack_rules::DiagnosticList> {
    let adapters = AdapterRegistry::builtins()
        .map_err(|error| sunstack_rules::DiagnosticList::from(error.diagnostics()))?;
    Ok(RuleExecutionContext {
        backend: Arc::new(JevRuleBackendFactory {
            selection: backend.clone(),
        }),
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
            let outcome = match rule_context(&input.backend, cancellation) {
                Ok(context) => sunstack_rules::prepare_rules(*input, context).await,
                Err(diagnostics) => RulePreparationOutcome::Invalid { diagnostics },
            };
            OperationResult::PrepareRules(Box::new(outcome))
        }
        OperationInput::EvaluateRules(input) => {
            let outcome = match rule_context(&input.backend, cancellation) {
                Ok(context) => sunstack_rules::evaluate_rules(*input, context).await,
                Err(diagnostics) => RuleInvocationOutcome::Invalid { diagnostics },
            };
            OperationResult::EvaluateRules(Box::new(outcome))
        }
        OperationInput::EvaluateCandidates(input) => {
            let outcome = match rule_context(&input.backend, cancellation) {
                Ok(context) => sunstack_rules::evaluate_candidates(*input, context).await,
                Err(diagnostics) => sunstack_rules::CandidateRunOutcome::Invalid { diagnostics },
            };
            OperationResult::EvaluateCandidates(Box::new(outcome))
        }
    }
}
