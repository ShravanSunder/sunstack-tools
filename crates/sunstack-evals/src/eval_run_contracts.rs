use crate::EvalReport;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_evidence::{
    EvidenceDiscovery, InvocationLimits, ProviderRecipe, RepositoryBinding, SourceName,
};
use sunstack_rules::{
    AttemptSettings, BackendSelection, DiagnosticList, EvidenceBinding, ExecutionSettings,
    RequestBudgetSettings,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvalRunInput {
    #[schemars(length(min = 1))]
    pub case_set: PathBuf,
    #[serde(default)]
    pub evidence: BTreeMap<SourceName, EvidenceBinding>,
    #[serde(default)]
    pub recipes: Vec<ProviderRecipe>,
    #[serde(default)]
    pub repositories: Vec<RepositoryBinding>,
    #[serde(default = "no_discovery")]
    pub discovery: EvidenceDiscovery,
    pub limits: InvocationLimits,
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
    pub execution: ExecutionSettings,
}
fn no_discovery() -> EvidenceDiscovery {
    EvidenceDiscovery::None {}
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvalRunOutcome {
    EvalReport { report: EvalReport },
    Invalid { diagnostics: DiagnosticList },
    Incomplete { report: EvalReport },
    Cancelled { report: EvalReport },
}
impl EvalRunOutcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::EvalReport { .. } => 0,
            Self::Invalid { .. } => 2,
            Self::Incomplete { .. } => 3,
            Self::Cancelled { .. } => 4,
        }
    }
}
