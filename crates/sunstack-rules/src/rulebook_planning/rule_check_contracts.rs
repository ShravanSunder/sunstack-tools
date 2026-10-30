use crate::{
    DiagnosticList, InvocationDefinitions, InvocationMeasurements, RuleDefinition, RuleId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_evidence::{InvocationLimits, PayloadSchemaPath, RepositoryBinding};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SkippedRule {
    Entry {
        rule_set: PathBuf,
        markdown: PathBuf,
        rules: PathBuf,
        key: String,
        rule_id: Option<RuleId>,
        diagnostics: DiagnosticList,
    },
    Source {
        rule_set: PathBuf,
        source_file: PathBuf,
        markdown: PathBuf,
        rules: PathBuf,
        diagnostics: DiagnosticList,
    },
}
impl SkippedRule {
    pub(crate) fn diagnostics(&self) -> &DiagnosticList {
        match self {
            Self::Entry { diagnostics, .. } | Self::Source { diagnostics, .. } => diagnostics,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleCoverage {
    Full,
    Partial,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckRulesInput {
    #[schemars(length(min = 1))]
    pub definition_root: PathBuf,
    pub definitions: InvocationDefinitions,
    pub repositories: Vec<RepositoryBinding>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub limits: InvocationLimits,
    #[serde(default = "crate::rule_definitions::invocation_contracts::no_discovery")]
    pub discovery: sunstack_evidence::EvidenceDiscovery,
    pub output: CheckRulesOutput,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckRulesOutput {
    Diagnostics,
    AssembledRules,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrintedRuleSource {
    Jsonl { file: PathBuf, line: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrintedRule {
    pub rule_set: PathBuf,
    pub source: PrintedRuleSource,
    pub definition: RuleDefinition,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CheckRulesOutcome {
    Checked {
        checked_rule_count: usize,
        printed_rules: Vec<PrintedRule>,
    },
    Refused {
        diagnostics: DiagnosticList,
        checked_rule_count: usize,
        refused_rule_count: usize,
    },
    CheckFailed {
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
    Cancelled {
        diagnostics: DiagnosticList,
        measurements: InvocationMeasurements,
    },
}
impl CheckRulesOutcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Checked { .. } => 0,
            Self::Refused { .. } => 2,
            Self::CheckFailed { .. } => 3,
            Self::Cancelled { .. } => 4,
        }
    }
}
