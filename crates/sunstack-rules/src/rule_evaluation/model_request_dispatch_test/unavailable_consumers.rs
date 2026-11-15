#![allow(clippy::panic)]
use super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use crate::{
    AdmittedSelection, RulebookResult, RulebookRule, RulebookRun, SelectionInput, SelectionOutcome,
    WorkUnavailableReason, run_rulebook_with_dispatcher, select_candidates,
};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{
    CandidateId, CandidateKey, EvidencePiece, FieldName, FieldUnavailable, FieldValue, ModelView,
    SelectionCandidate, SharedView, project_kind_model_view,
};

fn rule_referencing(references: serde_json::Value) -> RulebookRule {
    let mut wire = fixture::entry_json("K003");
    wire["evidence"] = json!("step");
    wire["questions"]["names_trigger"]["instructions"]["check"] = references;
    wire["decision"]["steps"]["names_trigger"]["no"] = json!("pass");
    let entry: crate::RuleEntryWire = serde_json::from_value(wire).unwrap();
    let decision = crate::admit_rule_entry(
        &entry,
        &fixture::kinds(),
        &fixture::project(),
        &Default::default(),
    )
    .unwrap();
    let mut rule = fixture::rule("K003");
    rule.kind = entry.evidence;
    rule.questions = entry.questions;
    rule.decision = decision;
    rule
}

fn projected_piece(fields: BTreeMap<FieldName, FieldValue>) -> EvidencePiece {
    let mut piece = fixture::piece();
    let kinds = fixture::kinds();
    piece.model_view = project_kind_model_view(
        &fixture::project(),
        &piece.kind,
        &kinds[&piece.kind],
        &fields,
        6000,
    );
    piece.fields = fields;
    piece
}

fn blocked_field(reason: FieldUnavailable) -> FieldValue {
    FieldValue::Unavailable { reason }
}

async fn run_rule(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    dispatcher: &mut ModelRequestDispatch,
) -> RulebookRun {
    run_rulebook_with_dispatcher(
        std::slice::from_ref(rule),
        std::slice::from_ref(piece),
        &BTreeMap::new(),
        dispatcher,
        &crate::FilterAnswers::default(),
        None,
    )
    .await
}

fn assert_unavailable_rule(
    run: &RulebookRun,
    expected_reason: WorkUnavailableReason,
    expected_field: Option<FieldName>,
) {
    assert!(run.prepared.is_empty());
    assert!(run.requests.is_empty());
    assert!(run.not_sent.is_empty());
    assert!(run.skips.is_empty());
    assert_eq!(run.results.len(), 1);
    let RulebookResult::Unavailable {
        reason,
        field,
        step,
        path_taken,
        answers_read,
    } = &run.results[0].result
    else {
        panic!("blocked consumer returned {:?}", run.results[0].result);
    };
    assert_eq!(*reason, expected_reason);
    assert_eq!(*field, expected_field);
    assert_eq!(step.as_ref().unwrap().as_str(), "names_trigger");
    assert_eq!(path_taken.len(), 1);
    assert_eq!(path_taken[0].as_str(), "names_trigger");
    assert!(answers_read.is_empty());
}

async fn wholly_unavailable_asking(use_credential_unavailable: bool) {
    // Arrange a live asking neighbor with independent bounded source content.
    let mut server = ControlledServer::start().await;
    let settings = settings(&server.endpoint);
    let backend = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "synthetic-loopback-only").unwrap(),
    );
    let original = fixture::piece();
    let body = FieldName::new("body").unwrap();
    let independent = projected_piece(BTreeMap::from([
        (body.clone(), original.fields[&body].clone()),
        (
            FieldName::new("completion").unwrap(),
            blocked_field(FieldUnavailable::SelectionUnavailable),
        ),
    ]));
    assert!(matches!(independent.model_view, ModelView::Bounded { .. }));
    let independent_rule = rule_referencing(json!("body"));
    let mut keyed = ModelRequestDispatch::new(
        settings.clone(),
        ModelDispatchPolicy::Execution {
            backend: backend.clone(),
        },
        CancellationToken::new(),
    );
    let (neighbor, ()) = tokio::join!(
        run_rule(&independent_rule, &independent, &mut keyed),
        async {
            let request = server.next().await;
            assert_eq!(
                request.body["state"]["body"],
                "Choose an observation for every obligation."
            );
            assert!(request.body["state"].get("completion").is_none());
            request.noul(0.9);
        }
    );
    assert!(matches!(
        neighbor.results[0].result,
        RulebookResult::Pass { .. }
    ));
    assert_eq!(neighbor.requests.len(), 1);
    assert_eq!(reservations(&keyed), [1, 1, 1, 0, 1, 0, 0, 0]);

    // Act through the real projection and supported asking API, with valid non-field references.
    let blocked = projected_piece(
        original
            .fields
            .into_keys()
            .map(|name| (name, blocked_field(FieldUnavailable::SelectionUnavailable)))
            .collect(),
    );
    assert!(matches!(
        blocked.model_view,
        ModelView::SelectionUnavailable
    ));
    let rule = rule_referencing(json!(["standard", "where"]));
    let cancellation = CancellationToken::new();
    let policy = if use_credential_unavailable {
        ModelDispatchPolicy::CredentialUnavailable
    } else {
        ModelDispatchPolicy::Execution { backend }
    };
    let mut dispatcher = ModelRequestDispatch::new(settings, policy, cancellation.clone());
    assert_eq!(reservations(&dispatcher), [0, 0, 1, 0, 0, 0, 0, 0]);
    let run = {
        let running = run_rule(&rule, &blocked, &mut dispatcher);
        tokio::pin!(running);
        tokio::select! {
            result = &mut running => result,
            request = server.receive() => {
                // A wrong build must finish with a real wrong outcome, rather than hang on HTTP.
                request.noul(0.9);
                running.await
            }
        }
    };

    // Assert upstream identity, reached path and every private reservation remain truthful.
    assert_eq!(
        reservations(&dispatcher),
        [0, 0, 1, 0, 0, 0, 0, 0],
        "{run:?}"
    );
    assert_unavailable_rule(
        &run,
        WorkUnavailableReason::SelectionUnavailable {
            evidence_kind: blocked.kind.clone(),
            file: blocked.location.file.clone(),
        },
        None,
    );
    cancellation.cancel();
    let cancelled = run_rule(&rule, &blocked, &mut dispatcher).await;
    assert!(
        matches!(cancelled.results[0].result, RulebookResult::Incomplete {
        reason: crate::RulebookIncompleteReason::Cancelled, ref path_taken, ..
    } if path_taken.is_empty())
    );
    assert_eq!(reservations(&dispatcher), [0, 0, 1, 0, 0, 0, 0, 0]);
    assert_eq!(
        server.finish().await,
        1,
        "only the eligible neighbor reaches HTTP"
    );
}

#[tokio::test]
async fn whole_unavailable_view_never_dispatches_under_execution() {
    wholly_unavailable_asking(false).await;
}

#[tokio::test]
async fn whole_unavailable_view_keeps_its_cause_without_credentials() {
    wholly_unavailable_asking(true).await;
}

fn mixed_piece(unavailable_field: &str) -> EvidencePiece {
    projected_piece(
        ["body", "completion"]
            .into_iter()
            .map(|name| {
                (
                    FieldName::new(name).unwrap(),
                    blocked_field(if name == unavailable_field {
                        FieldUnavailable::SelectionUnavailable
                    } else {
                        FieldUnavailable::SelectionPending
                    }),
                )
            })
            .collect(),
    )
}

fn selection_input(piece: &EvidencePiece, rule: &RulebookRule) -> SelectionInput {
    let empty = projected_piece(BTreeMap::new());
    SelectionInput {
        selection: AdmittedSelection {
            kind: piece.kind.clone(),
            question: rule.questions.values().next().unwrap().clone(),
            yes_at_or_above: 0.7,
            no_at_or_below: 0.3,
            shared: vec![],
        },
        candidates: vec![SelectionCandidate {
            id: CandidateId {
                kind: piece.kind.clone(),
                file: piece.location.file.clone(),
                span: piece.location.span.clone(),
            },
            key: CandidateKey::at(0),
            location: piece.location.clone(),
            first_line: piece.text.lines().next().unwrap().into(),
            selection_view: piece.model_view.clone(),
            fields: piece.fields.clone(),
        }],
        shared: vec![SharedView {
            kind: piece.kind.clone(),
            file: piece.location.file.clone(),
            view: empty.model_view,
            fields: BTreeMap::new(),
        }],
        files: BTreeSet::from([piece.location.file.clone()]),
    }
}

fn preparing_dispatcher() -> ModelRequestDispatch {
    let settings = settings("http://127.0.0.1:1/unused");
    let encoder = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "synthetic-preparation-only").unwrap(),
    );
    ModelRequestDispatch::new(
        settings,
        ModelDispatchPolicy::Preparation { encoder },
        CancellationToken::new(),
    )
}

async fn mixed_rule_preparation(unavailable_field: &str) {
    // Arrange both referenced states; swap their lexical names in the two callers.
    let piece = mixed_piece(unavailable_field);
    let rule = rule_referencing(json!(["body", "completion"]));
    let mut dispatcher = preparing_dispatcher();

    // Act through first walking and the ordinary preparation wrapper.
    let run = run_rule(&rule, &piece, &mut dispatcher).await;

    // Assert a pending neighbor cannot erase the needed terminal field or its path.
    assert_unavailable_rule(
        &run,
        WorkUnavailableReason::SelectedFieldUnavailable {
            field: FieldName::new(unavailable_field).unwrap(),
        },
        Some(FieldName::new(unavailable_field).unwrap()),
    );
    assert_eq!(reservations(&dispatcher), [0, 0, 1, 0, 0, 0, 0, 0]);
}

async fn mixed_selection_preparation(unavailable_field: &str) {
    // Arrange the same producer-made mixed state at the supported selector API.
    let piece = mixed_piece(unavailable_field);
    let rule = rule_referencing(json!(["body", "completion"]));
    let input = selection_input(&piece, &rule);
    let candidate_id = input.candidates[0].id.clone();
    let mut dispatcher = preparing_dispatcher();

    // Act and assert terminal availability wins before pending-only deferral.
    let outcomes = select_candidates(&input, &mut dispatcher).await;
    assert_eq!(outcomes.len(), 1);
    assert!(
        matches!(&outcomes[&candidate_id], SelectionOutcome::Unavailable {
        reason: WorkUnavailableReason::SelectedFieldUnavailable { field }
    } if field.as_str() == unavailable_field),
        "{outcomes:?}"
    );
    assert_eq!(reservations(&dispatcher), [0, 0, 1, 0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn preparation_rule_keeps_unavailable_before_pending() {
    mixed_rule_preparation("body").await;
}

#[tokio::test]
async fn preparation_rule_keeps_unavailable_after_pending() {
    mixed_rule_preparation("completion").await;
}

#[tokio::test]
async fn preparation_selection_keeps_unavailable_before_pending() {
    mixed_selection_preparation("body").await;
}

#[tokio::test]
async fn preparation_selection_keeps_unavailable_after_pending() {
    mixed_selection_preparation("completion").await;
}

#[tokio::test]
async fn pending_only_preparation_retains_its_distinct_deferral() {
    // Arrange pending-only fields through the same projection and real consumers.
    let piece = projected_piece(
        ["body", "completion"]
            .into_iter()
            .map(|name| {
                (
                    FieldName::new(name).unwrap(),
                    blocked_field(FieldUnavailable::SelectionPending),
                )
            })
            .collect(),
    );
    assert!(matches!(piece.model_view, ModelView::SelectionPending));
    let rule = rule_referencing(json!(["body", "completion"]));
    let mut dispatcher = preparing_dispatcher();

    // Act and assert unresolved preparation is still deferred, never terminal unavailable.
    let run = run_rule(&rule, &piece, &mut dispatcher).await;
    assert!(run.results.is_empty());
    assert!(run.prepared.is_empty());
    let input = selection_input(&piece, &rule);
    let outcomes = select_candidates(&input, &mut dispatcher).await;
    assert!(matches!(
        outcomes[&input.candidates[0].id],
        SelectionOutcome::Deferred
    ));
    assert_eq!(reservations(&dispatcher), [0, 0, 1, 0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn preparation_keeps_independent_bounded_fields_eligible() {
    // Arrange an unreferenced terminal field beside the real body this path needs.
    let original = fixture::piece();
    let body = FieldName::new("body").unwrap();
    let piece = projected_piece(BTreeMap::from([
        (body.clone(), original.fields[&body].clone()),
        (
            FieldName::new("completion").unwrap(),
            blocked_field(FieldUnavailable::SelectionUnavailable),
        ),
    ]));
    assert!(matches!(piece.model_view, ModelView::Bounded { .. }));
    let rule = rule_referencing(json!("body"));
    let mut rule_dispatcher = preparing_dispatcher();

    // Act through both real preparation consumers, without sending either request.
    let run = run_rule(&rule, &piece, &mut rule_dispatcher).await;
    let input = selection_input(&piece, &rule);
    let mut selection_dispatcher = preparing_dispatcher();
    let outcomes = select_candidates(&input, &mut selection_dispatcher).await;

    // Assert independent work is genuinely prepared, rather than globally blocked.
    assert!(run.results.is_empty());
    assert_eq!(run.prepared.len(), 1);
    assert_eq!(reservations(&rule_dispatcher), [1, 1, 1, 0, 0, 1, 0, 0]);
    let rule_request = serde_json::to_value(&rule_dispatcher.prepared[0].request).unwrap();
    assert_eq!(
        rule_request["state"]["body"],
        "Choose an observation for every obligation."
    );
    assert!(rule_request["state"].get("completion").is_none());
    assert!(matches!(
        outcomes[&input.candidates[0].id],
        SelectionOutcome::Prepared { request_id: 0 }
    ));
    assert_eq!(
        reservations(&selection_dispatcher),
        [1, 1, 1, 0, 0, 1, 1, 0]
    );
    let selection_request =
        serde_json::to_value(&selection_dispatcher.prepared[0].request).unwrap();
    assert_eq!(
        selection_request["state"]["candidates"]["c1"]["body"],
        "Choose an observation for every obligation."
    );
}
