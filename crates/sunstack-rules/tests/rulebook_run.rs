#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "../src/rulebook_planning/rulebook_test_support.rs"]
mod fixture;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use sunstack_decisions::*;
use sunstack_evidence::*;
use sunstack_rules::*;
use tokio::sync::Notify;

fn rule() -> RulebookRule {
    let mut entry = fixture::entry_json("K001");
    entry["questions"] =
        json!({"q":{"type":"noul","instructions":{"check":"body","task":"Does it comply?"}}});
    entry["decision"] = json!({"start":"q","steps":{"q":{"ask":"q","yes_at_or_above":0.8,"no_at_or_below":0.6,"yes":"pass","no":{"finding":"Not compliant.","at":"body"}}}});
    let entry: RuleEntryWire = serde_json::from_value(entry).unwrap();
    let mut rule = fixture::rule("K001");
    rule.questions = entry.questions.clone();
    rule.decision = admit_rule_entry(&entry, &fixture::kinds(), &fixture::project()).unwrap();
    rule
}
fn pieces() -> Vec<EvidencePiece> {
    ["first.md", "second.md"]
        .into_iter()
        .map(|name| {
            let mut piece = fixture::piece();
            piece.location.file = name.into();
            if let SourceRef::Document { path, .. } = &mut piece.location.span.source {
                *path = name.into();
            }
            piece
        })
        .collect()
}
fn settings(endpoint: &str) -> RulebookRunSettings {
    RulebookRunSettings {
        backend: BackendSelection::JevOpenrouter {
            endpoint: endpoint.into(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        },
        attempts: AttemptSettings {
            retry_count: 0,
            ..AttemptSettings::default()
        },
        request_budget: RequestBudgetSettings {
            max_input_tokens: 100000,
            max_request_bytes: 100000,
            max_questions: 10,
            max_requests: 10,
            safety_margin_tokens: 0,
            accounting_policy: AccountingPolicy::AllowEstimated,
        },
        request_max_chars: 100000,
        max_concurrent_requests: 2,
        max_model_requests: 10,
    }
}

#[tokio::test]
async fn incomplete_steps_distinguish_pre_walk_cancellation_from_a_request_guard() {
    let rules = [rule()];
    let pieces = pieces();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = run_rulebook(
        &rules,
        &pieces,
        &BTreeMap::new(),
        settings("not-dispatched"),
        None,
        cancellation,
    )
    .await;
    assert_eq!(cancelled.results.len(), 2);
    assert!(cancelled.requests.is_empty());
    for pair in cancelled.results {
        assert!(matches!(
            pair.result,
            RulebookResult::Incomplete {
                reason: RulebookIncompleteReason::Cancelled,
                step: None,
                ..
            }
        ));
    }
    let mut bounded = settings("not-dispatched");
    bounded.request_max_chars = 1;
    let guarded = run_rulebook(
        &rules,
        &pieces,
        &BTreeMap::new(),
        bounded,
        None,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(guarded.results.len(), 2);
    assert!(guarded.requests.is_empty());
    for pair in guarded.results {
        assert!(
            matches!(pair.result,RulebookResult::Incomplete {reason:RulebookIncompleteReason::RequestTooLarge,step:Some(step),..} if step.as_str() == "q")
        );
    }
}
struct OrderedBackend {
    inner: JevOpenRouterBackend,
    release_first: Arc<Notify>,
    second_completed: Arc<Notify>,
}
impl DecisionBackend for OrderedBackend {
    fn identity(&self) -> &BackendIdentity {
        self.inner.identity()
    }
    fn selection(&self) -> &BackendSelection {
        DecisionBackend::selection(&self.inner)
    }
    fn encode_request(
        &self,
        request: &StructuredJudgmentRequest,
    ) -> Result<EncodedBackendRequest, DecisionDiagnostic> {
        self.inner.encode_request(request)
    }
    fn account_request(&self, request: &EncodedBackendRequest) -> TokenAccounting {
        self.inner.account_request(request)
    }
    fn dispatch<'a>(
        &'a self,
        request: &'a AdmittedBackendRequest,
        max: usize,
    ) -> BackendAttemptFuture<'a> {
        Box::pin(async move {
            let value: serde_json::Value = serde_json::from_slice(request.body()).unwrap();
            let first = value["state"]["where"]["file"] == "first.md";
            if first {
                self.release_first.notified().await;
            }
            let response = self.inner.dispatch(request, max).await;
            if !first {
                self.second_completed.notify_one();
            }
            response
        })
    }
    fn admit_response(
        &self,
        request: &StructuredJudgmentRequest,
        body: &[u8],
    ) -> Result<(NamedAnswerMap, ExecutionReceipt), DecisionDiagnostic> {
        self.inner.admit_response(request, body)
    }
}
#[tokio::test]
async fn real_http_completion_in_reverse_order_keeps_pair_and_request_order() {
    let mut server = controlled::ControlledServer::start().await;
    let settings = settings(&server.endpoint);
    let release = Arc::new(Notify::new());
    let completed = Arc::new(Notify::new());
    let backend: Arc<dyn DecisionBackend> = Arc::new(OrderedBackend {
        inner: JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_KEY").unwrap(),
        release_first: release.clone(),
        second_completed: completed.clone(),
    });
    let operation = tokio::spawn(async move {
        run_rulebook(
            &[rule()],
            &pieces(),
            &BTreeMap::new(),
            settings,
            Some(backend),
            CancellationToken::new(),
        )
        .await
    });
    let second = server.next().await;
    assert_eq!(second.body["state"]["where"]["file"], "second.md");
    second.noul(0.9);
    tokio::time::timeout(Duration::from_secs(3), completed.notified())
        .await
        .unwrap();
    release.notify_one();
    let first = server.next().await;
    assert_eq!(first.body["state"]["where"]["file"], "first.md");
    first.noul(0.9);
    let run = tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 2);
    assert_eq!(
        run.results
            .iter()
            .map(|pair| pair.piece_index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(
        run.results
            .iter()
            .all(|pair| matches!(pair.result, RulebookResult::Pass { .. }))
    );
    assert_eq!(
        run.requests
            .iter()
            .map(|request| request.pair_index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}
#[tokio::test]
async fn request_limit_and_provider_failure_are_contained_to_their_pair() {
    for limited in [true, false] {
        let mut server = controlled::ControlledServer::start().await;
        let mut settings = settings(&server.endpoint);
        settings.max_concurrent_requests = 1;
        if limited {
            settings.max_model_requests = 1;
        }
        let backend: Arc<dyn DecisionBackend> =
            Arc::new(JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_KEY").unwrap());
        let operation = tokio::spawn(async move {
            run_rulebook(
                &[rule()],
                &pieces(),
                &BTreeMap::new(),
                settings,
                Some(backend),
                CancellationToken::new(),
            )
            .await
        });
        let first = server.next().await;
        if limited {
            first.noul(0.9);
        } else {
            first.respond(400, json!({"error":"controlled refusal"}));
            server.next().await.noul(0.9);
        }
        let run = tokio::time::timeout(Duration::from_secs(3), operation)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.finish().await, if limited { 1 } else { 2 });
        if limited {
            assert!(matches!(run.results[0].result, RulebookResult::Pass { .. }));
            assert!(matches!(
                run.results[1].result,
                RulebookResult::Incomplete {
                    reason: RulebookIncompleteReason::RequestLimitReached,
                    ..
                }
            ));
        } else {
            assert!(matches!(
                run.results[0].result,
                RulebookResult::Incomplete {
                    reason: RulebookIncompleteReason::ProviderFailure,
                    ..
                }
            ));
            assert!(matches!(run.results[1].result, RulebookResult::Pass { .. }));
        }
    }
}

#[tokio::test]
async fn ask_naming_an_unavailable_field_never_dispatches() {
    let mut server = controlled::ControlledServer::start().await;
    let settings = settings(&server.endpoint);
    let backend: Arc<dyn DecisionBackend> =
        Arc::new(JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_KEY").unwrap());
    let mut piece = fixture::piece();
    piece.fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable,
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    let mut operation = tokio::spawn(async move {
        run_rulebook(
            &[rule()],
            &[piece],
            &BTreeMap::new(),
            settings,
            Some(backend),
            CancellationToken::new(),
        )
        .await
    });
    let run=tokio::time::timeout(Duration::from_secs(3),async {
        tokio::select! {result=&mut operation=>result.unwrap(),request=server.receive()=>{request.noul(0.9);operation.await.unwrap()}}
    }).await.unwrap();
    assert_eq!(
        server.finish().await,
        0,
        "an unavailable referenced field reached HTTP"
    );
    assert!(
        matches!(&run.results[0].result,RulebookResult::Incomplete {reason:RulebookIncompleteReason::EvidenceUnavailable,field:Some(field), ..} if field.as_str()=="body")
    );
}

#[tokio::test]
async fn an_unreferenced_unavailable_field_is_omitted_while_the_named_available_field_dispatches() {
    let mut server = controlled::ControlledServer::start().await;
    let settings = settings(&server.endpoint);
    let backend: Arc<dyn DecisionBackend> =
        Arc::new(JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_KEY").unwrap());
    let mut piece = fixture::piece();
    piece.fields.insert(
        FieldName::new("heading").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable,
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    let operation = tokio::spawn(async move {
        run_rulebook(
            &[rule()],
            &[piece],
            &BTreeMap::new(),
            settings,
            Some(backend),
            CancellationToken::new(),
        )
        .await
    });
    let request = server.next().await;
    let state = request.body["state"].clone();
    request.noul(0.9);
    let run = tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 1);
    assert!(matches!(run.results[0].result, RulebookResult::Pass { .. }));
    assert!(state.get("heading").is_none());
    assert!(state.get("body").is_some());
}
