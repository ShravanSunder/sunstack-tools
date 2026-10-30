use crate::*;
use std::sync::Arc;
use sunstack_decisions::{BackendSelection, DecisionBackend, JevOpenRouterBackend};
pub(crate) fn backend_selection(config: &RulesConfig) -> BackendSelection {
    BackendSelection::JevOpenrouter {
        model: config.decision_model.model.clone(),
        endpoint: config
            .decision_model
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://openrouter.ai/api/v1/systemone".into()),
        context_ceiling_tokens: None,
    }
}
pub(crate) fn prepare_backend(
    config: &RulesConfig,
    key: &str,
) -> Result<Arc<dyn DecisionBackend>, LintInvalid> {
    JevOpenRouterBackend::new(backend_selection(config), key)
        .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
        .map_err(|_| {
            LintInvalid::at(
                config
                    .source
                    .as_ref()
                    .map(|source| source.location.file.clone())
                    .unwrap_or_else(|| "sunstack.lint.json".into()),
                LintInvalidCode::Syntax,
                "decision_model endpoint or credential is invalid",
            )
        })
}
