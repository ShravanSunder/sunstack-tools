//! Tagged transport outcomes for the general-rule SDK.
use super::sdk_contracts::SdkFailure;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralRuleCall<TOutcome> {
    Received { outcome: TOutcome },
    Failed { failure: SdkFailure },
}

use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::{
    AttemptSettings, BackendSelection, CancellationToken, JevOpenRouterBackend,
    RequestBudgetSettings,
};
use sunstack_evidence::{
    EvidenceDiscovery, InvocationLimits, PackageSourceBinding, PayloadSchemaPath,
    RepositoryBinding, ReturnedCursor, SourceName,
};
use sunstack_rules::{
    BindingGroup, BoundRuleSet, CapturedRuleSet, DiagnosticList, ExecutionSettings, InputBinding,
    InputDeclaration, InvocationDefinitions, InvocationOrigin, RuleDiagnosticCode,
    RuleInvocationInput, RuleInvocationOutcome, RulePreviewOutcome, capture_rule_set,
    validate_rule_settings,
};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleRunProfile {
    pub definition_root: PathBuf,
    pub repositories: Vec<RepositoryBinding>,
    pub discovery: EvidenceDiscovery,
    pub package_bindings: Vec<PackageSourceBinding>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub cursors: Vec<ReturnedCursor>,
    pub execution: ExecutionSettings,
    pub limits: InvocationLimits,
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSetLoadRequest {
    pub path: PathBuf,
    pub input_contract: BTreeMap<SourceName, InputDeclaration>,
    pub profile: RuleRunProfile,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundRuleRun {
    pub inputs: BTreeMap<SourceName, InputBinding>,
    pub origin: InvocationOrigin,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleSetLoadStatus {
    Ready,
    Invalid { diagnostics: DiagnosticList },
    Cancelled,
    Failed { failure: SdkFailure },
}

#[derive(Clone)]
pub struct AdmittedRuleSet {
    path: PathBuf,
    profile: RuleRunProfile,
    capture: CapturedRuleSet,
}

impl AdmittedRuleSet {
    pub async fn load(
        request: RuleSetLoadRequest,
        cancellation: CancellationToken,
    ) -> Result<Self, DiagnosticList> {
        let RuleSetLoadRequest {
            path,
            input_contract,
            profile,
        } = request;
        validate_rule_settings(
            &profile.definition_root,
            &profile.attempts,
            &profile.request_budget,
        )?;
        JevOpenRouterBackend::for_preparation(profile.backend.clone()).map_err(|error| {
            DiagnosticList::from(sunstack_decisions::DiagnosticList::from(error))
        })?;
        let capture = capture_rule_set(
            profile.definition_root.clone(),
            path.clone(),
            input_contract,
            profile.limits.max_file_bytes.get(),
            cancellation,
        )
        .await?;
        Ok(Self {
            path,
            profile,
            capture,
        })
    }

    fn bind(&self, binding: BoundRuleRun) -> RuleInvocationInput {
        let RuleRunProfile {
            definition_root,
            repositories,
            discovery,
            package_bindings,
            payload_schemas,
            cursors,
            execution,
            limits,
            backend,
            attempts,
            request_budget,
        } = self.profile.clone();
        RuleInvocationInput {
            definition_root,
            definitions: InvocationDefinitions::RuleSets {
                rule_sets: vec![BoundRuleSet {
                    path: self.path.clone(),
                    binding_groups: vec![BindingGroup {
                        bindings: binding.inputs,
                    }],
                }],
            },
            repositories,
            discovery,
            package_bindings,
            payload_schemas,
            cursors,
            execution,
            limits,
            backend,
            attempts,
            request_budget,
            origin: binding.origin,
        }
    }

    pub async fn evaluate(
        &self,
        binding: BoundRuleRun,
        cancellation: CancellationToken,
    ) -> RuleInvocationOutcome {
        crate::host_services::general_rules::evaluate_captured(
            self.bind(binding),
            self.capture.clone(),
            cancellation,
        )
        .await
    }

    pub async fn preview(
        &self,
        binding: BoundRuleRun,
        cancellation: CancellationToken,
    ) -> RulePreviewOutcome {
        crate::host_services::general_rules::preview_captured(
            self.bind(binding),
            self.capture.clone(),
            cancellation,
        )
        .await
    }
}

pub fn invalid_sdk_request() -> DiagnosticList {
    DiagnosticList::rule(RuleDiagnosticCode::MalformedJson)
}
