use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sunstack_rules::{QuestionRef, RuleRef, RunUsage};
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvalReport {
    pub rule: RuleRef,
    pub rule_version: ContentIdentity,
    pub case_set: CaseSetIdentity,
    pub split: Split,
    pub filter: FilterScore,
    pub steps: Vec<StepScore>,
    pub end_result: EndScore,
    pub unknown_labels: usize,
    pub cost: RunUsage,
    pub speed: SpeedSummary,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FilterScore {
    pub positive_cases: usize,
    pub kept: usize,
    pub missed: Vec<CaseId>,
    pub unknown: Vec<CaseId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StepScore {
    pub question: QuestionRef,
    pub correct: Vec<CaseId>,
    pub wrong: Vec<CaseId>,
    pub unknown: Vec<CaseId>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndScore {
    pub true_findings: Vec<CaseId>,
    pub false_findings: Vec<CaseId>,
    pub correct_negatives: Vec<CaseId>,
    pub missed_findings: Vec<CaseId>,
    pub inconclusive: Vec<CaseId>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PercentileConvention {
    NearestRank,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpeedSummary {
    pub run_wall_time_ms: u64,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub measured_candidates: usize,
    pub missing_candidates: usize,
    pub percentile: PercentileConvention,
}
