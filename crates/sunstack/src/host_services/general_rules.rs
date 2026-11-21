//! Shared host context for native rule operations.
use std::sync::Arc;
use sunstack_decisions::{
    BackendSelection, CancellationToken, DecisionBackend, JevOpenRouterBackend,
};
use sunstack_evidence::{AdapterRegistry, EvidencePlanInput, EvidencePlanOutcome, EvidenceResult};
use sunstack_rules::{
    CapturedRuleSet, DiagnosticList, RuleBackendFactory, RuleBackendPreparation, RuleDiagnostic,
    RuleDiagnosticCode, RuleExecutionContext, RuleInvocationInput, RuleInvocationOutcome,
    RulePreparationOutcome, RulePreviewOutcome,
};

#[derive(Clone, Copy)]
enum BackendUse {
    Evaluation,
    Preparation,
}

struct JevRuleBackendFactory {
    selection: BackendSelection,
    operation: BackendUse,
}

impl RuleBackendFactory for JevRuleBackendFactory {
    fn selection(&self) -> &BackendSelection {
        &self.selection
    }
    fn prepare(self: Arc<Self>) -> RuleBackendPreparation {
        Box::pin(async move {
            let selection = self.selection.clone();
            let backend = match self.operation {
                BackendUse::Preparation => JevOpenRouterBackend::for_preparation(selection),
                BackendUse::Evaluation => {
                    let credential =
                        super::provider_credentials::resolve()
                            .await
                            .ok_or_else(|| {
                                let mut diagnostic =
                                    RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration);
                                diagnostic.guidance =
                                    super::provider_credentials::MISSING_CREDENTIAL.into();
                                DiagnosticList::from(diagnostic)
                            })?;
                    JevOpenRouterBackend::new(selection, &credential)
                }
            }
            .map_err(|diagnostic| {
                DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
            })?;
            let backend: Arc<dyn DecisionBackend> = Arc::new(backend);
            Ok(backend)
        })
    }
}

fn rule_context(
    input: &RuleInvocationInput,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
    operation: BackendUse,
) -> Result<RuleExecutionContext, DiagnosticList> {
    let adapters = adapters.map_err(|error| DiagnosticList::from(error.diagnostics()))?;
    Ok(RuleExecutionContext {
        backend: Arc::new(JevRuleBackendFactory {
            selection: input.backend.clone(),
            operation,
        }),
        adapters,
        cancellation,
    })
}

pub async fn evaluate(
    input: RuleInvocationInput,
    cancellation: CancellationToken,
) -> RuleInvocationOutcome {
    evaluate_with_registry(input, AdapterRegistry::builtins(), cancellation).await
}

pub async fn evaluate_with_registry(
    input: RuleInvocationInput,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
) -> RuleInvocationOutcome {
    match rule_context(&input, adapters, cancellation, BackendUse::Evaluation) {
        Ok(context) => sunstack_rules::evaluate_rules(input, context).await,
        Err(diagnostics) => RuleInvocationOutcome::Invalid { diagnostics },
    }
}

pub async fn prepare(
    input: RuleInvocationInput,
    cancellation: CancellationToken,
) -> RulePreparationOutcome {
    match rule_context(
        &input,
        AdapterRegistry::builtins(),
        cancellation,
        BackendUse::Preparation,
    ) {
        Ok(context) => sunstack_rules::prepare_rules(input, context).await,
        Err(diagnostics) => RulePreparationOutcome::Invalid { diagnostics },
    }
}

pub async fn inspect_evidence(
    input: EvidencePlanInput,
    cancellation: CancellationToken,
) -> EvidencePlanOutcome {
    inspect_evidence_with_registry(input, AdapterRegistry::builtins(), cancellation).await
}

pub async fn inspect_evidence_with_registry(
    input: EvidencePlanInput,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
) -> EvidencePlanOutcome {
    match adapters {
        Ok(registry) => sunstack_evidence::prepare_evidence(input, registry, cancellation).await,
        Err(error) => EvidencePlanOutcome::Refused {
            diagnostics: error.diagnostics(),
        },
    }
}

pub async fn preview(
    input: RuleInvocationInput,
    cancellation: CancellationToken,
) -> RulePreviewOutcome {
    preview_with_registry(input, AdapterRegistry::builtins(), cancellation).await
}

pub async fn preview_with_registry(
    input: RuleInvocationInput,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
) -> RulePreviewOutcome {
    match rule_context(&input, adapters, cancellation, BackendUse::Preparation) {
        Ok(context) => sunstack_rules::preview_rules(input, context, None).await,
        Err(diagnostics) => RulePreviewOutcome::Unavailable {
            outcome: Box::new(RulePreparationOutcome::Invalid { diagnostics }),
        },
    }
}

pub async fn evaluate_captured(
    input: RuleInvocationInput,
    capture: CapturedRuleSet,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
) -> RuleInvocationOutcome {
    match rule_context(&input, adapters, cancellation, BackendUse::Evaluation) {
        Ok(context) => sunstack_rules::evaluate_captured_rules(input, context, capture).await,
        Err(diagnostics) => RuleInvocationOutcome::Invalid { diagnostics },
    }
}

pub async fn preview_captured(
    input: RuleInvocationInput,
    capture: CapturedRuleSet,
    adapters: EvidenceResult<AdapterRegistry>,
    cancellation: CancellationToken,
) -> RulePreviewOutcome {
    match rule_context(&input, adapters, cancellation, BackendUse::Preparation) {
        Ok(context) => sunstack_rules::preview_rules(input, context, Some(capture)).await,
        Err(diagnostics) => RulePreviewOutcome::Unavailable {
            outcome: Box::new(RulePreparationOutcome::Invalid { diagnostics }),
        },
    }
}
