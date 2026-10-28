use crate::BackendConfig;
use std::sync::Arc;
use sunstack_decisions::{BackendSelection, DecisionBackend, JevOpenRouterBackend};
use sunstack_rules::{DiagnosticList, RuleBackendFactory, RuleDiagnostic, RuleDiagnosticCode};

pub(crate) struct LintBackendFactory {
    pub selection: BackendSelection,
    pub configured: Option<BackendConfig>,
    pub api_key: Option<String>,
}
fn missing(message: &str) -> DiagnosticList {
    let mut diagnostic = RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration);
    diagnostic.guidance = message.into();
    diagnostic.into()
}
impl RuleBackendFactory for LintBackendFactory {
    fn selection(&self) -> &BackendSelection {
        &self.selection
    }
    fn prepare(self: Arc<Self>) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        if self.configured.is_none() {
            return Err(missing("Lint backend configuration is missing."));
        }
        let key = self.api_key.as_deref().filter(|key| !key.trim().is_empty())
            .ok_or_else(|| missing("OpenRouter credential is missing. Set OPENROUTER_API_KEY or run sunstack credentials save first."))?;
        JevOpenRouterBackend::new(self.selection.clone(), key)
            .map(|backend| -> Arc<dyn DecisionBackend> { Arc::new(backend) })
            .map_err(|diagnostic| sunstack_decisions::DiagnosticList::from(diagnostic).into())
    }
}
