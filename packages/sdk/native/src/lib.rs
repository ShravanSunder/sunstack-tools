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
    general_rules::{
        AdmittedRuleSet, BoundRuleRun, GeneralRuleCall, RuleSetLoadRequest, RuleSetLoadStatus,
        admit_transcript_registry, invalid_sdk_request,
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

/// Capability marker for SDKs that supply transcript mapping arguments.
#[napi]
pub fn supports_transcript_adapter_mappings() -> bool {
    true
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

/// An admitted definition and profile belonging to this addon instance.
#[napi]
pub struct NativeRuleSet {
    admitted: AdmittedRuleSet,
}

#[napi(object, object_from_js = false)]
pub struct NativeRuleSetLoadReply {
    pub json: String,
    pub rule_set: Option<NativeRuleSet>,
}

/// Capture definitions locally, retaining the approved profile with them.
#[napi]
pub fn load_rule_set<'env>(
    env: &'env Env,
    input_json: String,
    cancellation: &CallCancellation,
) -> napi::Result<PromiseRaw<'env, NativeRuleSetLoadReply>> {
    let token = cancellation.token.clone();
    env.spawn_future(async move {
        let (status, rule_set) = match serde_json::from_str::<RuleSetLoadRequest>(&input_json) {
            Ok(input) => {
                let observation = token.clone();
                owned(
                    async move {
                        let captured = AdmittedRuleSet::load(input, token).await;
                        if observation.is_cancelled() {
                            return (RuleSetLoadStatus::Cancelled, None);
                        }
                        match captured {
                            Ok(admitted) => {
                                (RuleSetLoadStatus::Ready, Some(NativeRuleSet { admitted }))
                            }
                            Err(diagnostics) => (RuleSetLoadStatus::Invalid { diagnostics }, None),
                        }
                    },
                    |failure| (RuleSetLoadStatus::Failed { failure }, None),
                )
                .await
            }
            Err(_) => (
                RuleSetLoadStatus::Invalid {
                    diagnostics: invalid_sdk_request(),
                },
                None,
            ),
        };
        Ok(NativeRuleSetLoadReply {
            json: encode(&status),
            rule_set,
        })
    })
}

/// Execute a general native invocation over explicitly bound sources.
#[napi]
pub fn evaluate_rules<'env>(
    env: &'env Env,
    input_json: String,
    cancellation: &CallCancellation,
    rule_set: Option<&NativeRuleSet>,
    adapter_mappings_json: Option<String>,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    let admitted = rule_set.map(|set| set.admitted.clone());
    env.spawn_future(async move {
        use sunstack_rules::RuleInvocationOutcome;
        let result = owned(
            async move {
                let outcome = if let Some(set) = admitted {
                    match serde_json::from_str::<BoundRuleRun>(&input_json) {
                        Ok(binding) => {
                            set.evaluate(
                                binding,
                                admit_transcript_registry(adapter_mappings_json, &token).await,
                                token,
                            )
                            .await
                        }
                        Err(_) => RuleInvocationOutcome::Invalid {
                            diagnostics: invalid_sdk_request(),
                        },
                    }
                } else {
                    match sunstack_rules::decode_rule_invocation(input_json.as_bytes()) {
                        Ok(input) => {
                            sunstack::host_services::general_rules::evaluate_with_registry(
                                input,
                                admit_transcript_registry(adapter_mappings_json, &token).await,
                                token,
                            )
                            .await
                        }
                        Err(diagnostics) => RuleInvocationOutcome::Invalid { diagnostics },
                    }
                };
                GeneralRuleCall::Received { outcome }
            },
            |failure| GeneralRuleCall::Failed { failure },
        )
        .await;
        Ok(encode(&result))
    })
}

/// Read general evidence without provider access.
#[napi]
pub fn inspect_evidence<'env>(
    env: &'env Env,
    input_json: String,
    cancellation: &CallCancellation,
    adapter_mappings_json: Option<String>,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    env.spawn_future(async move {
        use sunstack_evidence::EvidencePlanOutcome;
        let result = match sunstack_evidence::decode_evidence_plan_input(input_json.as_bytes()) {
            Ok(input) => {
                owned(
                    async move {
                        GeneralRuleCall::Received {
                        outcome:
                            sunstack::host_services::general_rules::inspect_evidence_with_registry(
                                input,
                                admit_transcript_registry(adapter_mappings_json, &token).await,
                                token,
                            )
                            .await,
                    }
                    },
                    |failure| GeneralRuleCall::Failed { failure },
                )
                .await
            }
            Err(diagnostics) => GeneralRuleCall::Received {
                outcome: EvidencePlanOutcome::Refused {
                    diagnostics: sunstack_evidence::DiagnosticList {
                        items: vec![*diagnostics],
                        omitted_count: 0,
                    },
                },
            },
        };
        Ok(encode(&result))
    })
}

/// Preview general rule input without constructing an authenticated backend.
#[napi]
pub fn prepare_rules<'env>(
    env: &'env Env,
    input_json: String,
    cancellation: &CallCancellation,
    rule_set: Option<&NativeRuleSet>,
    adapter_mappings_json: Option<String>,
) -> napi::Result<PromiseRaw<'env, String>> {
    let token = cancellation.token.clone();
    let admitted = rule_set.map(|set| set.admitted.clone());
    env.spawn_future(async move {
        use sunstack_rules::{RulePreparationOutcome, RulePreviewOutcome};
        let invalid = |diagnostics| RulePreviewOutcome::Unavailable {
            outcome: Box::new(RulePreparationOutcome::Invalid { diagnostics }),
        };
        let result = owned(
            async move {
                let outcome = if let Some(set) = admitted {
                    match serde_json::from_str::<BoundRuleRun>(&input_json) {
                        Ok(binding) => {
                            set.preview(
                                binding,
                                admit_transcript_registry(adapter_mappings_json, &token).await,
                                token,
                            )
                            .await
                        }
                        Err(_) => invalid(invalid_sdk_request()),
                    }
                } else {
                    match sunstack_rules::decode_rule_invocation(input_json.as_bytes()) {
                        Ok(input) => {
                            sunstack::host_services::general_rules::preview_with_registry(
                                input,
                                admit_transcript_registry(adapter_mappings_json, &token).await,
                                token,
                            )
                            .await
                        }
                        Err(diagnostics) => invalid(diagnostics),
                    }
                };
                GeneralRuleCall::Received { outcome }
            },
            |failure| GeneralRuleCall::Failed { failure },
        )
        .await;
        Ok(encode(&result))
    })
}
