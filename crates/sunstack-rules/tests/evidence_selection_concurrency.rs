#![allow(clippy::unwrap_used)]
#[path = "../src/rulebook_planning/rulebook_test_support.rs"]
mod rule_fixture;

use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use sunstack_decisions::*;
use sunstack_evidence::*;
use sunstack_rules::*;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestStage {
    Selection,
    Rule,
}

struct HeldReply {
    stage: RequestStage,
    release: oneshot::Sender<()>,
}

struct GatedBackend {
    inner: JevOpenRouterBackend,
    entered: mpsc::UnboundedSender<HeldReply>,
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl DecisionBackend for GatedBackend {
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
    fn dispatch<'request>(
        &'request self,
        request: &'request AdmittedBackendRequest,
        _max_response_bytes: usize,
    ) -> BackendAttemptFuture<'request> {
        Box::pin(async move {
            let body: Value = serde_json::from_slice(request.body()).unwrap();
            let stage = if body["state"].get("candidates").is_some() {
                RequestStage::Selection
            } else {
                RequestStage::Rule
            };
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            let (release, wait) = oneshot::channel();
            self.entered.send(HeldReply { stage, release }).unwrap();
            wait.await.unwrap();
            self.active.fetch_sub(1, Ordering::SeqCst);
            let answers = body["questions"]
                .as_object()
                .unwrap()
                .keys()
                .map(|name| (name.clone(), json!({"type":"noul","noul":0.93})))
                .collect::<serde_json::Map<_, _>>();
            Ok(BackendResponse {
                http_status: 200,
                body: serde_json::to_vec(&json!({
                    "model":"typesafe/controlled",
                    "answers":answers,
                    "usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}
                }))
                .unwrap(),
            })
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

fn selection_input() -> SelectionInput {
    let root = rule_fixture::fixture().join("selection");
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let mut kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> =
        serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    kinds.retain(|kind, _| kind.as_str() == "workflow_section");
    let selection = admit_selections(&kinds).unwrap().remove(0);
    let piece = rule_fixture::piece();
    let candidates = (0..2)
        .map(|position| {
            let mut location = piece.location.clone();
            location.span.byte_start += position;
            SelectionCandidate {
                id: CandidateId {
                    kind: selection.kind.clone(),
                    file: location.file.clone(),
                    span: location.span.clone(),
                },
                key: CandidateKey::at(position),
                location,
                first_line: "## 1. Check the boundary".into(),
                fields: piece.fields.clone(),
                selection_view: piece.model_view.clone(),
            }
        })
        .collect();
    SelectionInput {
        shared: vec![SharedView {
            file: piece.location.file.clone(),
            kind: selection.kind.clone(),
            fields: BTreeMap::new(),
            view: project_model_view(&BTreeMap::new(), 6000),
        }],
        selection,
        candidates,
        files: BTreeSet::from([piece.location.file]),
    }
}

async fn observe_stage(
    entered: &mut mpsc::UnboundedReceiver<HeldReply>,
    active: &AtomicUsize,
    peak: &AtomicUsize,
    stage: RequestStage,
    concurrency: usize,
) {
    for _ in 0..2 / concurrency {
        let mut held = Vec::new();
        for _ in 0..concurrency {
            let request = tokio::time::timeout(Duration::from_secs(1), entered.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(request.stage, stage);
            held.push(request);
        }
        // The clock is paused: this deadline advances only once runnable work
        // has quiesced. Every received request remains blocked on its release.
        // A bypass therefore enters the backend before this negative probe ends.
        let extra = tokio::time::timeout(Duration::from_millis(1), entered.recv()).await;
        assert!(
            extra.is_err(),
            "another request entered while replies were held"
        );
        assert_eq!(active.load(Ordering::SeqCst), concurrency);
        assert_eq!(peak.load(Ordering::SeqCst), concurrency);
        for request in held {
            request.release.send(()).unwrap();
        }
    }
}

// R43's configured limit applies to both request stages. Two is the separating
// control: serializing everything would fail it, bypassing the limit fails one.
#[tokio::test(start_paused = true)]
async fn held_selection_and_rule_replies_obey_the_same_concurrency_limit() {
    for concurrency in [1, 2] {
        let selection = selection_input();
        let backend_selection = BackendSelection::JevOpenrouter {
            endpoint: "http://127.0.0.1:1/unused-test-transport".into(),
            model: ModelName::new("jev-latest").unwrap(),
            context_ceiling_tokens: None,
        };
        let settings = RulebookRunSettings {
            backend: backend_selection.clone(),
            attempts: AttemptSettings {
                retry_count: 0,
                ..AttemptSettings::default()
            },
            request_budget: RequestBudgetSettings {
                max_input_tokens: 100000,
                max_request_bytes: 100000,
                max_questions: 1,
                max_requests: 100,
                safety_margin_tokens: 0,
                accounting_policy: AccountingPolicy::AllowEstimated,
            },
            request_max_chars: 20000,
            max_concurrent_requests: concurrency,
            max_model_requests: 4,
            max_locate_lines: 80,
        };
        let (send, mut entered) = mpsc::unbounded_channel();
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let backend = Arc::new(GatedBackend {
            inner: JevOpenRouterBackend::new(backend_selection, "SYNTHETIC_TEST_KEY").unwrap(),
            entered: send,
            active: active.clone(),
            peak: peak.clone(),
        });
        let mut dispatcher =
            ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());
        let mut rule = rule_fixture::rule("K003");
        rule.kind = EvidenceKindName::new("step").unwrap();
        let mut piece = rule_fixture::piece();
        piece.fields.insert(
            FieldName::new("description").unwrap(),
            FieldValue::Text {
                value: "Use when testing".into(),
                spans: vec![],
            },
        );
        piece.model_view = project_model_view(&piece.fields, 6000);
        let operation = tokio::spawn(async move {
            let selected = select_candidates(&selection, &mut dispatcher).await;
            let rules = run_rulebook_with_dispatcher(
                &[rule],
                &[piece.clone(), piece],
                &BTreeMap::new(),
                &mut dispatcher,
            )
            .await;
            (selected, rules, dispatcher.records)
        });
        observe_stage(
            &mut entered,
            &active,
            &peak,
            RequestStage::Selection,
            concurrency,
        )
        .await;
        observe_stage(
            &mut entered,
            &active,
            &peak,
            RequestStage::Rule,
            concurrency,
        )
        .await;
        let (selected, rules, records) = tokio::time::timeout(Duration::from_secs(1), operation)
            .await
            .unwrap()
            .unwrap();
        assert!(
            selected
                .values()
                .all(|outcome| matches!(outcome, SelectionOutcome::Selected { .. }))
        );
        assert_eq!(selected.len(), 2);
        assert_eq!(rules.results.len(), 2);
        assert!(
            rules
                .results
                .iter()
                .all(|result| matches!(result.result, RulebookResult::Pass { .. }))
        );
        assert_eq!(records.len(), 4);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(peak.load(Ordering::SeqCst), concurrency);
    }
}
