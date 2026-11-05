use crate::{LintInvalid, LintInvalidCode, LintLimits};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use sunstack_decisions::{AccountingPolicy, AttemptSettings, RequestBudgetSettings};
use sunstack_evidence::{DiscoveryLimits, EvidenceDiscovery, InvocationLimits};
use sunstack_rules::{ExecutionSettings, Speculation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EffectiveLimits {
    pub max_locate_lines: u32,
    pub evidence_max_chars: u32,
    pub request_max_chars: u32,
    pub max_files: u32,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_matches: u32,
    pub max_model_requests: u32,
    pub max_input_tokens_per_request: u32,
    pub attempt_timeout_seconds: u32,
    pub max_concurrent_requests: u32,
    pub max_transcript_bytes: u64,
    pub max_request_bytes: u64,
    pub max_questions_per_request: u32,
    pub retry_count: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub max_response_bytes: u64,
}
impl Default for EffectiveLimits {
    fn default() -> Self {
        Self {
            max_locate_lines: 80,
            evidence_max_chars: 6000,
            request_max_chars: 20000,
            max_files: 2000,
            max_file_bytes: 1_048_576,
            max_total_bytes: 67_108_864,
            max_matches: 10_000,
            max_model_requests: 200,
            max_input_tokens_per_request: 32_000,
            attempt_timeout_seconds: 60,
            max_concurrent_requests: 4,
            max_transcript_bytes: 8_388_608,
            max_request_bytes: 262_144,
            max_questions_per_request: 32,
            retry_count: 2,
            initial_backoff_ms: 500,
            max_backoff_ms: 8000,
            max_response_bytes: 1_048_576,
        }
    }
}
impl From<&LintLimits> for EffectiveLimits {
    fn from(limits: &LintLimits) -> Self {
        let mut effective = Self::default();
        if let Some(value) = limits.max_locate_lines {
            effective.max_locate_lines = value.get();
        }
        if let Some(value) = limits.evidence_max_chars {
            effective.evidence_max_chars = value.get();
        }
        if let Some(value) = limits.request_max_chars {
            effective.request_max_chars = value.get();
        }
        if let Some(value) = limits.max_files {
            effective.max_files = value.get();
        }
        if let Some(value) = limits.max_file_bytes {
            effective.max_file_bytes = value.get();
        }
        if let Some(value) = limits.max_total_bytes {
            effective.max_total_bytes = value.get();
        }
        if let Some(value) = limits.max_matches {
            effective.max_matches = value.get();
        }
        if let Some(value) = limits.max_model_requests {
            effective.max_model_requests = value.get();
        }
        if let Some(value) = limits.max_input_tokens_per_request {
            effective.max_input_tokens_per_request = value.get();
        }
        if let Some(value) = limits.attempt_timeout_seconds {
            effective.attempt_timeout_seconds = value.get();
        }
        if let Some(value) = limits.max_concurrent_requests {
            effective.max_concurrent_requests = value.get();
        }
        effective
    }
}
pub(crate) fn limit_number(value: u64) -> Result<NonZeroUsize, LintInvalid> {
    usize::try_from(value)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or_else(|| {
            LintInvalid::at(
                "sunstack.lint.json",
                LintInvalidCode::Syntax,
                "A limit must be positive and fit this platform.",
            )
        })
}
impl EffectiveLimits {
    pub fn invocation_limits(&self) -> Result<InvocationLimits, LintInvalid> {
        Ok(InvocationLimits {
            max_files: limit_number(u64::from(self.max_files))?,
            max_file_bytes: limit_number(self.max_file_bytes)?,
            max_total_bytes: limit_number(self.max_total_bytes)?,
            max_matches: limit_number(u64::from(self.max_matches))?,
            max_transcript_bytes: limit_number(self.max_transcript_bytes)?,
            include_ignored: false,
        })
    }
    pub fn request_budget(&self) -> Result<RequestBudgetSettings, LintInvalid> {
        Ok(RequestBudgetSettings {
            max_input_tokens: u64::from(self.max_input_tokens_per_request),
            max_request_bytes: limit_number(self.max_request_bytes)?.get(),
            max_questions: limit_number(u64::from(self.max_questions_per_request))?.get(),
            max_requests: limit_number(u64::from(self.max_model_requests))?.get(),
            safety_margin_tokens: 0,
            accounting_policy: AccountingPolicy::AllowEstimated,
        })
    }
    pub fn attempts(&self) -> Result<AttemptSettings, LintInvalid> {
        Ok(AttemptSettings {
            attempt_timeout_ms: u64::from(self.attempt_timeout_seconds) * 1000,
            retry_count: self.retry_count,
            initial_backoff_ms: self.initial_backoff_ms,
            max_backoff_ms: self.max_backoff_ms,
            max_response_bytes: limit_number(self.max_response_bytes)?.get(),
        })
    }
    pub fn execution(&self) -> Result<ExecutionSettings, LintInvalid> {
        Ok(ExecutionSettings {
            speculation: Speculation::Off,
            max_concurrent_requests: limit_number(u64::from(self.max_concurrent_requests))?,
        })
    }
    pub fn discovery(&self) -> Result<EvidenceDiscovery, LintInvalid> {
        Ok(EvidenceDiscovery::Bounded {
            limits: DiscoveryLimits {
                max_files: limit_number(u64::from(self.max_files))?,
                max_file_bytes: limit_number(self.max_file_bytes)?,
                max_total_bytes: limit_number(self.max_total_bytes)?,
                max_matches: limit_number(u64::from(self.max_matches))?,
                include_ignored: false,
                max_unit_visits: limit_number(10_000)?,
                max_relationship_candidates: limit_number(10_000)?,
                max_fan_out: limit_number(100)?,
                max_history_commits: limit_number(1000)?,
                max_local_time_ms: limit_number(10_000)?,
            },
        })
    }
}

#[cfg(test)]
#[path = "limit_settings_test.rs"]
mod tests;
