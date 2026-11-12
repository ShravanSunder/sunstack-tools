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
        .map_err(|_| backend_refusal(bundle, "decision_model endpoint or credential is invalid"))
}

fn backend_refusal(bundle: &DefinitionBundle, message: &str) -> LintInvalid {
    let file = bundle
        .root
        .text
        .as_ref()
        .and(bundle.root.origin.file_path())
        .map_or_else(|| "sunstack.lint.json".into(), std::path::Path::to_path_buf);
    LintInvalid::at(file, LintInvalidCode::Syntax, message).in_definition(&bundle.root.origin)
}

pub(crate) fn prepare_encoding_backend(
    bundle: &DefinitionBundle,
) -> Result<Arc<dyn DecisionBackend>, LintInvalid> {
    JevOpenRouterBackend::for_preparation(backend_selection(&bundle.settings.decision_model))
        .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
        .map_err(|_| backend_refusal(bundle, "decision_model endpoint is invalid"))
}
