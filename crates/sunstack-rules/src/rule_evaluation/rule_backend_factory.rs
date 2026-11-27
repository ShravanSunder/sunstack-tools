use crate::{DiagnosticList, RuleDiagnosticCode};
use std::{future::Future, pin::Pin, sync::Arc};
use sunstack_decisions::{BackendSelection, DecisionBackend};

pub type RuleBackendPreparation =
    Pin<Box<dyn Future<Output = Result<Arc<dyn DecisionBackend>, DiagnosticList>> + Send>>;

/// Selection is credential-free; construction waits for reachable model work.
pub trait RuleBackendFactory: Send + Sync {
    fn selection(&self) -> &BackendSelection;
    fn prepare(self: Arc<Self>) -> RuleBackendPreparation;
}

impl<Backend: DecisionBackend + 'static> RuleBackendFactory for Backend {
    fn selection(&self) -> &BackendSelection {
        DecisionBackend::selection(self)
    }
    fn prepare(self: Arc<Self>) -> RuleBackendPreparation {
        let backend: Arc<dyn DecisionBackend> = self;
        Box::pin(std::future::ready(Ok(backend)))
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
    pub(crate) async fn resolve(&mut self) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        if let Some(prepared) = &self.prepared {
            return prepared.clone();
        }
        let prepared = self.factory.clone().prepare().await.and_then(|backend| {
            if backend.selection() != self.factory.selection() {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::InvalidConfiguration,
                ));
            }
            Ok(backend)
        });
        self.prepared = Some(prepared.clone());
        prepared
    }
}
