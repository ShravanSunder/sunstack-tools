//! Direct evidence inspection through the shared Rust acquisition and listing path.
use super::{
    CredentialSource,
    definition_capture::{CaptureFailure, capture_configuration, refusal},
    report_projection::{CancelledStatus, IncompleteStatus, project_diagnostic},
    sdk_contracts::{
        AfterDispatchReason, Configuration, Diagnostic, DiagnosticCode, Scope, SdkFailure,
    },
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::{
    DefinitionOrigin, DefinitionSnapshot, EvidenceKindName, EvidencePiece, FileCount,
};
use sunstack_lint::{
    EffectiveLimits, ExclusionCounts, IncompletePart, LintReport, LintStatus, RunStage,
    RunStageStopped, ScopeChoice, ScopeReport, SelectionEntry, UsageSummary,
};

/// The data part of an inspection request; cancellation is passed separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct InspectRequest {
    /// Absolute checked root, separate from the definition base directory.
    pub root: PathBuf,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub evidence_kind: Option<EvidenceKindName>,
}

/// Evidence-work states; none describes whether source code violates a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum InspectionStatus {
    Clean,
    Uncertain,
    Incomplete,
    Cancelled,
}

/// A completed listing can be clean or have unresolved selection answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum InspectedStatus {
    Clean,
    Uncertain,
}

/// Original selected evidence and the exact coverage facts that produced the ending.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct InspectionReport<TInspectionStatus> {
    pub version: String,
    pub configuration: DefinitionOrigin,
    pub sources: Vec<DefinitionSnapshot>,
    pub scope: ScopeReport,
    pub status: TInspectionStatus,
    pub pieces: Vec<EvidencePiece>,
    pub file_counts: Vec<FileCount>,
    pub selections: Vec<SelectionEntry>,
    pub usage: UsageSummary,
    pub limits: EffectiveLimits,
    pub exclusions: ExclusionCounts,
    pub diagnostics: Vec<Diagnostic>,
    pub incomplete: Vec<IncompletePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum InspectionOutcome {
    Inspected {
        report: InspectionReport<InspectedStatus>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
    Unavailable {
        diagnostics: Vec<Diagnostic>,
    },
    Incomplete {
        report: InspectionReport<IncompleteStatus>,
    },
    Cancelled {
        report: Option<InspectionReport<CancelledStatus>>,
    },
    Failed {
        failure: SdkFailure,
        report: Option<InspectionReport<InspectionStatus>>,
    },
}

fn project_report<TInspectionStatus>(
    report: LintReport,
    status: TInspectionStatus,
    configuration: &DefinitionOrigin,
    sources: &[DefinitionSnapshot],
) -> InspectionReport<TInspectionStatus> {
    InspectionReport {
        version: report.version,
        configuration: configuration.clone(),
        sources: sources.to_vec(),
        scope: report.scope,
        status,
        pieces: report
            .evidence_listing
            .map(|listing| listing.pieces)
            .unwrap_or_default(),
        file_counts: report.file_counts,
        selections: report.selections,
        usage: report.usage,
        limits: report.limits,
        exclusions: report.exclusions,
        diagnostics: report.diagnostics.iter().map(project_diagnostic).collect(),
        incomplete: report.incomplete,
    }
}

fn project_inspection(
    report: LintReport,
    configuration: &DefinitionOrigin,
    sources: &[DefinitionSnapshot],
) -> InspectionOutcome {
    match report.status {
        LintStatus::Clean => InspectionOutcome::Inspected {
            report: project_report(report, InspectedStatus::Clean, configuration, sources),
        },
        LintStatus::Uncertain => InspectionOutcome::Inspected {
            report: project_report(report, InspectedStatus::Uncertain, configuration, sources),
        },
        LintStatus::Invalid => InspectionOutcome::Invalid {
            diagnostics: report.diagnostics.iter().map(project_diagnostic).collect(),
        },
        LintStatus::Incomplete => InspectionOutcome::Incomplete {
            report: project_report(report, IncompleteStatus::Incomplete, configuration, sources),
        },
        LintStatus::Cancelled => InspectionOutcome::Cancelled {
            report: Some(project_report(
                report,
                CancelledStatus::Cancelled,
                configuration,
                sources,
            )),
        },
        LintStatus::Prepared | LintStatus::NothingSelected | LintStatus::Findings => {
            InspectionOutcome::Failed {
                failure: SdkFailure::after_dispatch(
                    AfterDispatchReason::NativeReplyInvalid,
                    "evidence inspection returned a rule-evaluation or preparation status",
                ),
                report: None,
            }
        }
    }
}

fn project_stopped_inspection(
    stopped: RunStageStopped,
    configuration: &DefinitionOrigin,
    sources: &[DefinitionSnapshot],
) -> InspectionOutcome {
    let report = match stopped.stage() {
        RunStage::Admission => None,
        _ => Some(project_report(
            stopped.into_partial_report(),
            InspectionStatus::Incomplete,
            configuration,
            sources,
        )),
    };
    InspectionOutcome::Failed {
        failure: SdkFailure::after_dispatch(
            AfterDispatchReason::NativeCallFailed,
            "an inspection stage stopped unexpectedly",
        ),
        report,
    }
}

/// Inspect source evidence without executing rule questions, gates or decisions.
pub async fn inspect(
    configuration: Configuration,
    request: InspectRequest,
    cancellation: CancellationToken,
) -> InspectionOutcome {
    inspect_with_credential(configuration, request, cancellation, CredentialSource::Host).await
}

/// Inspect with an explicit credential source; static kinds never resolve credentials.
pub async fn inspect_with_credential(
    configuration: Configuration,
    request: InspectRequest,
    cancellation: CancellationToken,
    credential: CredentialSource,
) -> InspectionOutcome {
    if cancellation.is_cancelled() {
        return InspectionOutcome::Cancelled { report: None };
    }
    if !request.root.is_absolute() {
        return InspectionOutcome::Invalid {
            diagnostics: vec![refusal(
                DiagnosticCode::UnsupportedInputs,
                None,
                vec!["root".into()],
                "root must be an absolute path",
            )],
        };
    }
    let bundle =
        match tokio::task::spawn_blocking(move || capture_configuration(&configuration)).await {
            Ok(Ok(bundle)) => bundle,
            Ok(Err(CaptureFailure::Invalid(diagnostics))) => {
                return InspectionOutcome::Invalid { diagnostics };
            }
            Ok(Err(CaptureFailure::Unavailable(diagnostics))) => {
                return InspectionOutcome::Unavailable { diagnostics };
            }
            Err(_) => {
                return InspectionOutcome::Failed {
                    failure: SdkFailure::after_dispatch(
                        AfterDispatchReason::NativeCallFailed,
                        "inspection definition capture stopped unexpectedly",
                    ),
                    report: None,
                };
            }
        };
    if cancellation.is_cancelled() {
        return InspectionOutcome::Cancelled { report: None };
    }
    let model_selection = bundle
        .settings
        .kinds
        .values()
        .any(|kind| kind.select.is_some());
    let credential = if model_selection {
        match credential {
            CredentialSource::Host => crate::host_services::provider_credentials::resolve().await,
            CredentialSource::Supplied(credential) => credential,
        }
    } else {
        None
    };
    let configuration_origin = bundle.root.origin.clone();
    let sources = bundle.sources.clone();
    let scope = match request.scope {
        Scope::Default => ScopeChoice::Default,
        Scope::All => ScopeChoice::All,
        Scope::Since { rev } => ScopeChoice::Since { rev },
    };
    match sunstack_lint::inspect_definitions(
        bundle,
        request.root,
        request.paths,
        scope,
        request.evidence_kind,
        cancellation,
        credential,
    )
    .await
    {
        Ok(report) => project_inspection(report, &configuration_origin, &sources),
        Err(stopped) => project_stopped_inspection(stopped, &configuration_origin, &sources),
    }
}
