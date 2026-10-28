use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_decisions::{AttemptSettings, BackendSelection, RequestBudgetSettings};
use sunstack_evidence::{
    InvocationLimits, OccurrenceId, PayloadSchemaPath, ProviderRecipe, RepositoryBinding,
    SourceInput,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateRunInput {
    #[schemars(length(min = 1))]
    pub sources: Vec<SourceEntry>,
    pub definitions: CandidateDefinitions,
    pub definition_root: PathBuf,
    #[serde(default)]
    pub recipes: Vec<ProviderRecipe>,
    pub repositories: Vec<RepositoryBinding>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub execution: ExecutionSettings,
    pub limits: InvocationLimits,
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
    #[serde(default = "default_samples_per_pass")]
    pub samples_per_pass: usize,
}
fn default_samples_per_pass() -> usize {
    20
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceEntry {
    pub source: SourceInput,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateDefinitions {
    RuleSets {
        #[schemars(length(min = 1))]
        rule_sets: Vec<PathBuf>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleRef {
    pub rule_set: PathBuf,
    pub rule_id: RuleId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateId {
    pub rule: RuleRef,
    pub occurrence: OccurrenceId,
}

pub fn decode_candidate_run(bytes: &[u8]) -> Result<CandidateRunInput, DiagnosticList> {
    let result = crate::strict_decoding::decode_definition(
        bytes,
        std::path::Path::new("<candidate_run>"),
        1,
    );
    match result {
        Err(mut diagnostics) => {
            // Name a missing contracted field without echoing an untrusted misspelling.
            if serde_json::from_slice::<serde_json::Value>(bytes)
                .ok()
                .is_some_and(|value| value.is_object() && value.get("sources").is_none())
            {
                diagnostics.extend(crate::candidate_inputs::invalid_candidate_field(vec![
                    "sources".into(),
                ]));
            }
            Err(diagnostics)
        }
        admitted => admitted,
    }
}
