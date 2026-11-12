//! The one Rust projection from engine reports to SDK run outcomes.
//!
//! Engine facts are reused unchanged; only definition references become origin-aware.
use super::sdk_contracts::{AfterDispatchReason, Diagnostic, SdkFailure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::SharedEvidenceState;
use sunstack_evidence::{DefinitionOrigin, DefinitionSnapshot, FieldName, MarkerId, SourceSpan};
use sunstack_lint::{
    EffectiveLimits, ExclusionCounts, Finding, IdentifiedResult, IncompletePart, JudgmentSummary,
    LintDiagnostic, LintReport, LintStatus, RelatedPlace, RulePieceResult, SavedPieceEvidence,
    SavedRequest, ScopeReport, SelectionEntry, SettingOrigin, SeverityCounts, UncertainResult,
    UsageSummary,
};
use sunstack_rules::{Anchor, Basis, RuleId, RulebookSkippedRule, Severity};

/// Report statuses a completed run can have; completed does not mean clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum CompletedStatus {
    Clean,
    NothingSelected,
    Findings,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum InvalidStatus {
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum IncompleteStatus {
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum CancelledStatus {
    Cancelled,
}

/// Where a related place is: a checked file, or a definition such as the standard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum PlaceLocation {
    Checked { path: PathBuf },
    Definition { origin: DefinitionOrigin },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkRelatedPlace {
    pub label: String,
    pub location: PlaceLocation,
    pub first_line: usize,
    pub last_line: usize,
}

/// A finding whose standard reference is origin-aware.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkFinding {
    pub id: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub locator_omitted: Option<sunstack_rules::LocatorOmission>,
    pub rule: RuleId,
    pub marker: MarkerId,
    /// The checked file.
    pub file: PathBuf,
    pub line: usize,
    pub field: Option<FieldName>,
    pub span: Option<SourceSpan>,
    pub severity: Severity,
    pub severity_origin: SettingOrigin,
    pub message: String,
    pub judgment: Option<JudgmentSummary>,
    pub basis: Basis,
    pub related: Vec<SdkRelatedPlace>,
    pub anchor: Anchor,
}

/// A rule that could not be admitted, with origin-aware definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkSkippedRule {
    pub markdown: DefinitionOrigin,
    pub rules: DefinitionOrigin,
    pub key: String,
    pub rule_id: Option<RuleId>,
    pub diagnostics: Vec<Diagnostic>,
}

/// An engine report: every semantic `LintReport` fact, with origin-aware definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkReport<TStatus> {
    pub version: String,
    /// The configuration's root definition.
    pub configuration: DefinitionOrigin,
    /// Exact definition content this run used.
    pub sources: Vec<DefinitionSnapshot>,
    pub scope: ScopeReport,
    pub status: TStatus,
    pub findings: Vec<SdkFinding>,
    pub results: Vec<RulePieceResult>,
    pub uncertain: Vec<UncertainResult>,
    pub selections: Vec<SelectionEntry>,
    pub file_counts: Vec<sunstack_evidence::FileCount>,
    pub skipped_rules: Vec<SdkSkippedRule>,
    pub counts: SeverityCounts,
    pub exclusions: ExclusionCounts,
    pub incomplete: Vec<IncompletePart>,
    pub usage: UsageSummary,
    pub evidence_seen: BTreeMap<usize, SharedEvidenceState>,
    pub limits: EffectiveLimits,
    pub diagnostics: Vec<Diagnostic>,
    pub requests: Vec<SavedRequest>,
    pub filter_answers: Vec<IdentifiedResult<sunstack_rules::FilterAnswer>>,
    pub filter_skips: Vec<IdentifiedResult<sunstack_rules::FilterSkip>>,
    pub piece_evidence: BTreeMap<usize, SavedPieceEvidence>,
}

pub type CompletedReport = SdkReport<CompletedStatus>;
pub type InvalidReport = SdkReport<InvalidStatus>;
pub type IncompleteReport = SdkReport<IncompleteStatus>;
pub type CancelledReport = SdkReport<CancelledStatus>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum RunOutcome {
    Completed {
        report: SdkReport<CompletedStatus>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
        report: Option<SdkReport<InvalidStatus>>,
    },
    Unavailable {
        diagnostics: Vec<Diagnostic>,
    },
    Incomplete {
        report: SdkReport<IncompleteStatus>,
    },
    Cancelled {
        report: Option<SdkReport<CancelledStatus>>,
    },
    Failed {
        failure: SdkFailure,
        report: Option<SdkReport<LintStatus>>,
    },
}

/// Project a CLI diagnostic: definition origin and checked path stay separate.
pub fn project_diagnostic(diagnostic: &LintDiagnostic) -> Diagnostic {
    Diagnostic {
        code: diagnostic.code.into(),
        origin: diagnostic.definition.clone(),
        path: diagnostic.path.clone().or_else(|| {
            diagnostic
                .definition
                .is_none()
                .then(|| diagnostic.file.clone())
                .flatten()
        }),
        line: diagnostic.line,
        column: diagnostic.column,
        json_path: diagnostic.json_path.clone(),
        message: diagnostic.message.clone(),
    }
}

fn project_place(place: RelatedPlace) -> SdkRelatedPlace {
    SdkRelatedPlace {
        label: place.label,
        location: match place.definition {
            Some(origin) => PlaceLocation::Definition { origin },
            None => PlaceLocation::Checked { path: place.file },
        },
        first_line: place.first_line,
        last_line: place.last_line,
    }
}

fn project_finding(finding: Finding) -> SdkFinding {
    SdkFinding {
        id: finding.id,
        locator_omitted: finding.locator_omitted,
        rule: finding.rule,
        marker: finding.marker,
        file: finding.file,
        line: finding.line,
        field: finding.field,
        span: finding.span,
        severity: finding.severity,
        severity_origin: finding.severity_origin,
        message: finding.message,
        judgment: finding.judgment,
        basis: finding.basis,
        related: finding.related.into_iter().map(project_place).collect(),
        anchor: finding.anchor,
    }
}

fn project_skipped(skipped: RulebookSkippedRule) -> SdkSkippedRule {
    SdkSkippedRule {
        markdown: skipped.markdown,
        rules: skipped.rules,
        key: skipped.key,
        rule_id: skipped.rule_id,
        diagnostics: super::configuration_loading::rule_refusals(skipped.diagnostics),
    }
}

fn project_report<TStatus>(
    report: LintReport,
    status: TStatus,
    configuration: &DefinitionOrigin,
    sources: &[DefinitionSnapshot],
) -> SdkReport<TStatus> {
    SdkReport {
        version: report.version,
        configuration: configuration.clone(),
        sources: sources.to_vec(),
        scope: report.scope,
        status,
        findings: report.findings.into_iter().map(project_finding).collect(),
        results: report.results,
        uncertain: report.uncertain,
        selections: report.selections,
        file_counts: report.file_counts,
        skipped_rules: report
            .skipped_rules
            .into_iter()
            .map(project_skipped)
            .collect(),
        counts: report.counts,
        exclusions: report.exclusions,
        incomplete: report.incomplete,
        usage: report.usage,
        evidence_seen: report.evidence_seen,
        limits: report.limits,
        diagnostics: report.diagnostics.iter().map(project_diagnostic).collect(),
        requests: report.requests,
        filter_answers: report.filter_answers,
        filter_skips: report.filter_skips,
        piece_evidence: report.piece_evidence,
    }
}

/// Map all eight engine statuses onto the closed run outcome.
pub fn project_run(
    report: LintReport,
    configuration: &DefinitionOrigin,
    sources: &[DefinitionSnapshot],
) -> RunOutcome {
    match report.status {
        LintStatus::Clean => RunOutcome::Completed {
            report: project_report(report, CompletedStatus::Clean, configuration, sources),
        },
        LintStatus::NothingSelected => RunOutcome::Completed {
            report: project_report(
                report,
                CompletedStatus::NothingSelected,
                configuration,
                sources,
            ),
        },
        LintStatus::Findings => RunOutcome::Completed {
            report: project_report(report, CompletedStatus::Findings, configuration, sources),
        },
        LintStatus::Uncertain => RunOutcome::Completed {
            report: project_report(report, CompletedStatus::Uncertain, configuration, sources),
        },
        LintStatus::Invalid => {
            let diagnostics = report.diagnostics.iter().map(project_diagnostic).collect();
            RunOutcome::Invalid {
                diagnostics,
                report: Some(project_report(
                    report,
                    InvalidStatus::Invalid,
                    configuration,
                    sources,
                )),
            }
        }
        LintStatus::Incomplete => RunOutcome::Incomplete {
            report: project_report(report, IncompleteStatus::Incomplete, configuration, sources),
        },
        LintStatus::Cancelled => RunOutcome::Cancelled {
            report: Some(project_report(
                report,
                CancelledStatus::Cancelled,
                configuration,
                sources,
            )),
        },
        LintStatus::Prepared => RunOutcome::Failed {
            failure: SdkFailure::after_dispatch(
                AfterDispatchReason::NativeReplyInvalid,
                "an ordinary run returned a prepared report",
            ),
            report: Some(project_report(
                report,
                LintStatus::Prepared,
                configuration,
                sources,
            )),
        },
    }
}
