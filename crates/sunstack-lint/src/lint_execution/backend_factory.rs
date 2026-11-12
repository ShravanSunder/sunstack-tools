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

pub(crate) fn prepare_encoding_backend(
    config: &RulesConfig,
) -> Result<Arc<dyn DecisionBackend>, LintInvalid> {
    JevOpenRouterBackend::for_preparation(backend_selection(config))
        .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
        .map_err(|_| {
            LintInvalid::at(
                config
                    .source
                    .as_ref()
                    .map(|source| source.location.file.clone())
                    .unwrap_or_else(|| "sunstack.lint.json".into()),
                LintInvalidCode::Syntax,
                "decision_model endpoint is invalid",
            )
        })
}

/// Refuse missing credentials before any evidence acquisition begins.
pub(super) fn prepare_run_backend(
    config: &RulesConfig,
    location: &ConfigLocation,
    model_work: bool,
    preparing: bool,
    api_key: Option<&str>,
) -> Result<Option<Arc<dyn DecisionBackend>>, LintInvalid> {
    let key = api_key.filter(|key| !key.trim().is_empty());
    if model_work && !preparing && key.is_none() {
        return Err(LintInvalid::at(
            &location.file,
            LintInvalidCode::CredentialMissing,
            "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first.",
        ));
    }
    let backend = if model_work && preparing {
        Some(prepare_encoding_backend(config)?)
    } else if model_work {
        match key.map(|key| prepare_backend(config, key)) {
            Some(Ok(backend)) => Some(backend),
            Some(Err(error)) => return Err(error),
            None => None,
        }
    } else {
        None
    };
    Ok(backend)
}
