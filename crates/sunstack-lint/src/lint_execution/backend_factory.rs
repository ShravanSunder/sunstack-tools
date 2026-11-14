use crate::{configuration_loading::definition_bundle::DefinitionBundle, *};
use std::sync::Arc;
use sunstack_decisions::{BackendSelection, DecisionBackend, JevOpenRouterBackend};
pub(crate) fn backend_selection(settings: &DecisionModelSettings) -> BackendSelection {
    BackendSelection::JevOpenrouter {
        model: settings.model.clone(),
        endpoint: settings
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://openrouter.ai/api/v1/systemone".into()),
        context_ceiling_tokens: None,
    }
}
pub(crate) fn prepare_backend(
    bundle: &DefinitionBundle,
    key: &str,
) -> Result<Arc<dyn DecisionBackend>, LintInvalid> {
    JevOpenRouterBackend::new(backend_selection(&bundle.settings.decision_model), key)
        .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
        .map_err(|_| {
            bundle.root.invalid(
                LintInvalidCode::Syntax,
                "decision_model endpoint or credential is invalid",
            )
        })
}

pub(crate) fn prepare_encoding_backend(
    bundle: &DefinitionBundle,
) -> Result<Arc<dyn DecisionBackend>, LintInvalid> {
    JevOpenRouterBackend::for_preparation(backend_selection(&bundle.settings.decision_model))
        .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
        .map_err(|_| {
            bundle.root.invalid(
                LintInvalidCode::Syntax,
                "decision_model endpoint is invalid",
            )
        })
}

/// Refuse missing credentials before any evidence acquisition begins.
pub(super) fn prepare_run_backend(
    bundle: &DefinitionBundle,
    model_work: bool,
    preparing: bool,
    api_key: Option<&str>,
) -> Result<Option<Arc<dyn DecisionBackend>>, LintInvalid> {
    let key = api_key.filter(|key| !key.trim().is_empty());
    if model_work && !preparing && key.is_none() {
        return Err(bundle.root.invalid(
            LintInvalidCode::CredentialMissing,
            "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first.",
        ));
    }
    let backend = if model_work && preparing {
        Some(prepare_encoding_backend(bundle)?)
    } else if model_work {
        match key.map(|key| prepare_backend(bundle, key)) {
            Some(Ok(backend)) => Some(backend),
            Some(Err(error)) => return Err(error),
            None => None,
        }
    } else {
        None
    };
    Ok(backend)
}

#[cfg(test)]
#[path = "backend_factory_test.rs"]
mod tests;
