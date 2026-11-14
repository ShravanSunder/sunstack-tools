//! Node-API adapter: transports JSON to the shared Sunstack facade and back.
//!
//! The adapter owns no policy. It decodes nothing beyond the strict Rust contract,
//! keeps one cancellation token per call and reports a native task failure as a
//! failed outcome with unknown effects rather than throwing.
use napi::{Env, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use sunstack::sdk_host::{
    self,
    document_codec::{
        DeserializationOutcome, DeserializeSdkRequest, DocumentInvalidReason,
        SdkDocumentDiagnostic, deserialize_document,
    },
    inspection_projection::{InspectRequest, InspectionOutcome},
    report_projection::RunOutcome,
    sdk_contracts::{
        AfterDispatchReason, CheckOutcome, LoadOptions, LoadOutcome, RunRequest, SdkFailure,
    },
};
use sunstack_decisions::CancellationToken;

/// This addon's release, contract revision and compiled target, as JSON.
#[napi]
pub fn contract_info() -> String {
    encode(&sdk_host::contract_info(env!("SUNSTACK_SDK_TARGET")))
}

/// One call's cancellation; aborting it never affects another call.
#[napi]
pub struct CallCancellation {
    token: CancellationToken,
}

#[napi]
impl CallCancellation {
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    #[napi]
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

fn encode(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| {
        // The reply could not be encoded: report it as an invalid native reply.
        serde_json::json!({
            "kind": "failed",
            "failure": SdkFailure::after_dispatch(
                AfterDispatchReason::NativeReplyInvalid,
                format!("native reply encoding failed: {error}"),
            ),
            "report": null,
        })
        .to_string()
    })
}

fn task_failure(message: &str) -> SdkFailure {
    SdkFailure::after_dispatch(AfterDispatchReason::NativeCallFailed, message)
}

/// Run a facade future as an owned task so a task failure becomes a typed reply.
async fn owned<TOutcome, TFuture>(
    future: TFuture,
    failed: impl FnOnce(SdkFailure) -> TOutcome,
) -> TOutcome
where
    TOutcome: Send + 'static,
    TFuture: std::future::Future<Output = TOutcome> + Send + 'static,
{
    match tokio::spawn(future).await {
        Ok(outcome) => outcome,
        Err(_) => failed(task_failure("the native call task stopped unexpectedly")),
    }
}

/// Load an existing configuration. Resolves with `LoadOutcome` JSON.
#[napi]
pub fn load(env: &Env, options_json: String) -> napi::Result<PromiseRaw<'_, String>> {
    env.spawn_future(async move {
        let outcome = match serde_json::from_str::<LoadOptions>(&options_json) {
            Ok(options) => {
                owned(sdk_host::load(options), |failure| LoadOutcome::Failed {
                    failure,
                })
                .await
            }
            Err(error) => LoadOutcome::Invalid {
                diagnostics: vec![sunstack::sdk_host::sdk_contracts::Diagnostic {
                    code: sunstack::sdk_host::sdk_contracts::DiagnosticCode::Syntax,
                    origin: None,
                    path: None,
                    line: None,
                    column: None,
                    json_path: vec![],
                    message: error.to_string(),
                }],
            },
        };
        Ok(encode(&outcome))
    })
}

/// Check a configuration without inference. Resolves with `CheckOutcome` JSON.
#[napi]
pub fn check<'env>(
    env: &'env Env,
    configuration_json: String,
    cancellation: &CallCancellation,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    env.spawn_future(async move {
        if token.is_cancelled() {
            return Ok(encode(&CheckOutcome::Cancelled {
                diagnostics: vec![],
            }));
        }
        let outcome = match sdk_host::decode_configuration(&configuration_json) {
            Ok(configuration) => {
                owned(sdk_host::check(configuration, token), |failure| {
                    CheckOutcome::Failed { failure }
                })
                .await
            }
            Err(diagnostics) => CheckOutcome::Invalid { diagnostics },
        };
        Ok(encode(&outcome))
    })
}

/// Run a configuration against an explicit root. Resolves with `RunOutcome` JSON.
#[napi]
pub fn run<'env>(
    env: &'env Env,
    configuration_json: String,
    request_json: String,
    cancellation: &CallCancellation,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    env.spawn_future(async move {
        if token.is_cancelled() {
            return Ok(encode(&RunOutcome::Cancelled { report: None }));
        }
        let decoded =
            sdk_host::decode_configuration(&configuration_json).and_then(|configuration| {
                serde_json::from_str::<RunRequest>(&request_json)
                    .map(|request| (configuration, request))
                    .map_err(|error| {
                        vec![sunstack::sdk_host::sdk_contracts::Diagnostic {
                            code: sunstack::sdk_host::sdk_contracts::DiagnosticCode::Syntax,
                            origin: None,
                            path: None,
                            line: None,
                            column: None,
                            json_path: vec![],
                            message: error.to_string(),
                        }]
                    })
            });
        let outcome = match decoded {
            Ok((configuration, request)) => {
                owned(sdk_host::run(configuration, request, token), |failure| {
                    RunOutcome::Failed {
                        failure,
                        report: None,
                    }
                })
                .await
            }
            Err(diagnostics) => RunOutcome::Invalid {
                diagnostics,
                report: None,
            },
        };
        Ok(encode(&outcome))
    })
}

/// Inspect selected evidence without rule evaluation. Resolves with `InspectionOutcome` JSON.
#[napi]
pub fn inspect<'env>(
    env: &'env Env,
    configuration_json: String,
    request_json: String,
    cancellation: &CallCancellation,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    env.spawn_future(async move {
        if token.is_cancelled() {
            return Ok(encode(&InspectionOutcome::Cancelled { report: None }));
        }
        let decoded =
            sdk_host::decode_configuration(&configuration_json).and_then(|configuration| {
                serde_json::from_str::<InspectRequest>(&request_json)
                    .map(|request| (configuration, request))
                    .map_err(|error| {
                        vec![sunstack::sdk_host::sdk_contracts::Diagnostic {
                            code: sunstack::sdk_host::sdk_contracts::DiagnosticCode::Syntax,
                            origin: None,
                            path: None,
                            line: None,
                            column: None,
                            json_path: vec![],
                            message: error.to_string(),
                        }]
                    })
            });
        let outcome = match decoded {
            Ok((configuration, request)) => {
                owned(
                    sdk_host::inspection_projection::inspect(configuration, request, token),
                    |failure| InspectionOutcome::Failed {
                        failure,
                        report: None,
                    },
                )
                .await
            }
            Err(diagnostics) => InspectionOutcome::Invalid { diagnostics },
        };
        Ok(encode(&outcome))
    })
}

/// Validate original saved text in one blocking unit, without engine or credential access.
#[napi]
pub fn deserialize(env: &Env, request_json: String) -> napi::Result<PromiseRaw<'_, String>> {
    env.spawn_future(async move {
        let outcome = match serde_json::from_str::<DeserializeSdkRequest>(&request_json) {
            Ok(request) => {
                match tokio::task::spawn_blocking(move || deserialize_document(&request.json)).await
                {
                    Ok(outcome) => outcome,
                    Err(_) => DeserializationOutcome::Failed {
                        failure: task_failure("document decoding stopped unexpectedly"),
                    },
                }
            }
            Err(error) => DeserializationOutcome::Invalid {
                reason: DocumentInvalidReason::InvalidDocument,
                diagnostics: vec![SdkDocumentDiagnostic {
                    json_path: vec!["json".into()],
                    message: error.to_string(),
                }],
            },
        };
        Ok(encode(&outcome))
    })
}
