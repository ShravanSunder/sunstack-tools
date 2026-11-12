//! The engine facade SDK adapters call: load, check and run with closed outcomes.
//!
//! Each call owns its inputs, cancellation token and context. Nothing here changes
//! the process working directory or environment, installs signal handlers, or returns
//! credentials to the caller. Check acquires no credential, provider or checked input.
pub mod configuration_loading;
pub mod definition_capture;
pub mod matcher_transport;
pub mod normalized_values;
pub mod report_projection;
pub mod sdk_contracts;

use definition_capture::{CaptureFailure, capture_configuration};
use report_projection::{RunOutcome, project_diagnostic, project_run};
use sdk_contracts::{
    AfterDispatchReason, CONTRACT_REVISION, CheckOutcome, CheckSummary, Configuration,
    ContractInfo, Diagnostic, DiagnosticCode, LoadOptions, LoadOutcome, RunRequest, Scope,
    SdkFailure,
};
use sunstack_decisions::CancellationToken;
use sunstack_lint::{CheckListing, ScopeChoice};

/// This build's contract identity, checked by the SDK before any engine call.
pub fn contract_info(target: &str) -> ContractInfo {
    ContractInfo {
        sdk_release: env!("CARGO_PKG_VERSION").into(),
        contract_revision: CONTRACT_REVISION,
        target: target.into(),
    }
}

fn native_failure(message: &str) -> SdkFailure {
    SdkFailure::after_dispatch(AfterDispatchReason::NativeCallFailed, message)
}

/// Decode transported configuration JSON with the strict Rust contract.
pub fn decode_configuration(text: &str) -> Result<Configuration, Vec<Diagnostic>> {
    let mut decoder = serde_json::Deserializer::from_str(text);
    let decoded: Result<Configuration, _> = serde_path_to_error::deserialize(&mut decoder);
    let decoded = decoded.and_then(|configuration| {
        // Trailing content has no field path.
        decoder.end().map(|()| configuration).map_err(|error| {
            serde_path_to_error::Error::new(serde_path_to_error::Track::new().path(), error)
        })
    });
    decoded.map_err(|error| {
        let message = error.inner().to_string();
        vec![Diagnostic {
            code: if message.starts_with("unknown field") {
                DiagnosticCode::UnknownField
            } else if message.contains("invalid glob") {
                DiagnosticCode::BadGlob
            } else {
                DiagnosticCode::Syntax
            },
            origin: None,
            path: None,
            line: None,
            column: None,
            json_path: error
                .path()
                .iter()
                .filter_map(|segment| match segment {
                    serde_path_to_error::Segment::Map { key } => Some(key.clone()),
                    serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
                    serde_path_to_error::Segment::Enum { variant } => Some(variant.clone()),
                    serde_path_to_error::Segment::Unknown => None,
                })
                .collect(),
            message,
        }]
    })
}

/// Load an existing configuration into captured values. Reads definitions once.
pub async fn load(options: LoadOptions) -> LoadOutcome {
    match tokio::task::spawn_blocking(move || configuration_loading::load_configuration(&options))
        .await
    {
        Ok(Ok(value)) => LoadOutcome::Loaded { value },
        Ok(Err(CaptureFailure::Invalid(diagnostics))) => LoadOutcome::Invalid { diagnostics },
        Ok(Err(CaptureFailure::Unavailable(diagnostics))) => {
            LoadOutcome::Unavailable { diagnostics }
        }
        Err(_) => LoadOutcome::Failed {
            failure: native_failure("configuration loading stopped unexpectedly"),
        },
    }
}

/// Admit a configuration without inference: no provider, credential or checked input.
pub async fn check(configuration: Configuration, cancellation: CancellationToken) -> CheckOutcome {
    if cancellation.is_cancelled() {
        return CheckOutcome::Cancelled {
            diagnostics: vec![],
        };
    }
    let checked = tokio::task::spawn_blocking(move || {
        let bundle = capture_configuration(&configuration)?;
        let listing = sunstack_lint::check_definitions(&bundle);
        Ok::<_, CaptureFailure>((bundle, listing))
    })
    .await;
    match checked {
        Err(_) => CheckOutcome::Failed {
            failure: native_failure("configuration checking stopped unexpectedly"),
        },
        Ok(Err(CaptureFailure::Invalid(diagnostics))) => CheckOutcome::Invalid { diagnostics },
        Ok(Err(CaptureFailure::Unavailable(diagnostics))) => {
            CheckOutcome::Unavailable { diagnostics }
        }
        Ok(Ok((_, Err(diagnostics)))) => CheckOutcome::Invalid {
            diagnostics: diagnostics.iter().map(project_diagnostic).collect(),
        },
        Ok(Ok((bundle, Ok(listing)))) => {
            let count = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
            CheckOutcome::Valid {
                summary: CheckSummary {
                    rulebook_count: count(bundle.rulebooks.len()),
                    active_rule_count: count(
                        listing
                            .iter()
                            .filter(|row| matches!(row, CheckListing::Rule(_)))
                            .count(),
                    ),
                    skipped_rule_count: 0,
                    evidence_kind_count: count(bundle.settings.kinds.len()),
                    filter_count: count(bundle.settings.filters.len()),
                },
                sources: bundle.sources,
            }
        }
    }
}

/// Run a configuration against an explicit absolute checked root.
///
/// The credential is resolved through the shared host resolver.
pub async fn run(
    configuration: Configuration,
    request: RunRequest,
    cancellation: CancellationToken,
) -> RunOutcome {
    run_with_credential(configuration, request, cancellation, CredentialSource::Host).await
}

/// Where a run's provider credential comes from.
pub enum CredentialSource {
    /// The existing host resolver: environment first, then the OS store.
    Host,
    /// A credential supplied by an in-process caller, such as a controlled test.
    Supplied(Option<String>),
}

/// Run with an explicit credential source; the facade's single run path.
pub async fn run_with_credential(
    configuration: Configuration,
    request: RunRequest,
    cancellation: CancellationToken,
    credential: CredentialSource,
) -> RunOutcome {
    if cancellation.is_cancelled() {
        return RunOutcome::Cancelled { report: None };
    }
    if !request.root.is_absolute() {
        return RunOutcome::Invalid {
            diagnostics: vec![definition_capture::refusal(
                DiagnosticCode::UnsupportedInputs,
                None,
                vec!["root".into()],
                "root must be an absolute path",
            )],
            report: None,
        };
    }
    let captured = tokio::task::spawn_blocking(move || capture_configuration(&configuration)).await;
    let bundle = match captured {
        Ok(Ok(bundle)) => bundle,
        Ok(Err(CaptureFailure::Invalid(diagnostics))) => {
            return RunOutcome::Invalid {
                diagnostics,
                report: None,
            };
        }
        Ok(Err(CaptureFailure::Unavailable(diagnostics))) => {
            return RunOutcome::Unavailable { diagnostics };
        }
        Err(_) => {
            return RunOutcome::Failed {
                failure: native_failure("definition capture stopped unexpectedly"),
                report: None,
            };
        }
    };
    if cancellation.is_cancelled() {
        return RunOutcome::Cancelled { report: None };
    }
    let credential = match credential {
        CredentialSource::Host => crate::host_services::provider_credentials::resolve().await,
        CredentialSource::Supplied(credential) => credential,
    };
    let configuration_origin = bundle.root.origin.clone();
    let sources = bundle.sources.clone();
    let scope = match request.scope {
        Scope::Default => ScopeChoice::Default,
        Scope::All => ScopeChoice::All,
        Scope::Since { rev } => ScopeChoice::Since { rev },
    };
    let report = sunstack_lint::run_definitions(
        bundle,
        request.root,
        request.paths,
        scope,
        cancellation,
        credential,
    )
    .await;
    project_run(report, &configuration_origin, &sources)
}
