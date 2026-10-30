use crate::{DiagnosticList, RuleDiagnosticCode};
use std::sync::Arc;
use sunstack_decisions::{BackendSelection, DecisionBackend};

/// Selection is credential-free; construction waits for reachable model work.
pub trait RuleBackendFactory: Send + Sync {
    fn selection(&self) -> &BackendSelection;
    fn prepare(self: Arc<Self>) -> Result<Arc<dyn DecisionBackend>, DiagnosticList>;
}

impl<Backend: DecisionBackend + 'static> RuleBackendFactory for Backend {
    fn selection(&self) -> &BackendSelection {
        DecisionBackend::selection(self)
    }
    fn prepare(self: Arc<Self>) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        Ok(self)
    }
}

pub(crate) struct RuleBackendSession {
    factory: Arc<dyn RuleBackendFactory>,
    prepared: Option<Result<Arc<dyn DecisionBackend>, DiagnosticList>>,
}
impl RuleBackendSession {
    pub(crate) fn new(factory: Arc<dyn RuleBackendFactory>) -> Self {
        Self {
            factory,
            prepared: None,
        }
    }
    pub(crate) fn resolve(&mut self) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        self.prepared
            .get_or_insert_with(|| {
                let backend = self.factory.clone().prepare()?;
                if backend.selection() != self.factory.selection() {
                    return Err(DiagnosticList::rule(
                        RuleDiagnosticCode::InvalidConfiguration,
                    ));
                }
                Ok(backend)
            })
            .clone()
    }
}
