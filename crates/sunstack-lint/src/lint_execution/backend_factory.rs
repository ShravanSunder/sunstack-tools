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

/// Select execution capability after admission and scope; listing retains its credential guard.
pub(super) fn prepare_run_backend(
    bundle: &DefinitionBundle,
    model_work: bool,
    preparing: bool,
    listing: bool,
    api_key: Option<&str>,
) -> Result<sunstack_rules::ModelDispatchPolicy, LintInvalid> {
    let key = api_key.filter(|key| !key.trim().is_empty());
    if listing && model_work && !preparing && key.is_none() {
        return Err(bundle.root.invalid(
            LintInvalidCode::CredentialMissing,
            "OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first.",
        ));
    }
    use sunstack_rules::ModelDispatchPolicy;
    Ok(if !model_work {
        ModelDispatchPolicy::LocalOnly
    } else if preparing {
        ModelDispatchPolicy::Preparation {
            encoder: prepare_encoding_backend(bundle)?,
        }
    } else if let Some(key) = key {
        ModelDispatchPolicy::Execution {
            backend: prepare_backend(bundle, key)?,
        }
    } else {
        let _validated = prepare_encoding_backend(bundle)?;
        ModelDispatchPolicy::CredentialUnavailable
    })
}

#[cfg(test)]
#[path = "backend_factory_test.rs"]
mod tests;
