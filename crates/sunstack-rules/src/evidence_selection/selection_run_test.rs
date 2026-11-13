#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::{Value, json};
use std::{collections::BTreeSet, sync::Arc};
use sunstack_decisions::{
    AccountingPolicy, AttemptSettings, BackendSelection, CancellationToken, DecisionBackend,
    DecisionRunOutcome, JevOpenRouterBackend, ModelName, RequestBudgetSettings,
    encode_judgment_body,
};
use sunstack_evidence::{FieldValue, project_model_view};

fn text_fields(name: &str, value: String) -> BTreeMap<FieldName, FieldValue> {
    BTreeMap::from([(
        FieldName::new(name).unwrap(),
        FieldValue::Text {
            value,
            spans: vec![],
        },
    )])
}

fn candidate(
    fields: BTreeMap<FieldName, FieldValue>,
    limit: usize,
    position: usize,
) -> SelectionCandidate {
    let piece = crate::rulebook_planning::rulebook_test_support::piece();
    SelectionCandidate {
        id: CandidateId {
            kind: sunstack_evidence::EvidenceKindName::new("section").unwrap(),
            file: piece.location.file.clone(),
            span: piece.location.span.clone(),
        },
        key: sunstack_evidence::CandidateKey::at(position),
        location: piece.location,
        first_line: "source heading".into(),
        selection_view: project_model_view(&fields, limit),
        fields,
    }
}

fn shared(fields: BTreeMap<FieldName, FieldValue>, limit: usize) -> SharedView {
    let piece = crate::rulebook_planning::rulebook_test_support::piece();
    SharedView {
        kind: sunstack_evidence::EvidenceKindName::new("section").unwrap(),
        file: piece.location.file,
        view: project_model_view(&fields, limit),
        fields,
    }
}

fn selection() -> AdmittedSelection {
    AdmittedSelection {
        kind: sunstack_evidence::EvidenceKindName::new("section").unwrap(),
        question: serde_json::from_value(json!({"type":"noul","instructions":{
            "check":"body","context":["outline"],"task":"Keep this section?"
        }}))
        .unwrap(),
        yes_at_or_above: 0.7,
        no_at_or_below: 0.3,
        shared: vec![FieldName::new("outline").unwrap()],
    }
}

fn dispatch_settings() -> crate::RulebookRunSettings {
    crate::RulebookRunSettings {
        backend: BackendSelection::JevOpenrouter {
            endpoint: "http://127.0.0.1:1/unused".into(),
            model: ModelName::new("typesafe/selection-control").unwrap(),
            context_ceiling_tokens: None,
        },
        attempts: AttemptSettings::default(),
        request_budget: RequestBudgetSettings {
            max_input_tokens: 100000,
            max_request_bytes: 100000,
            max_questions: 32,
            max_requests: 100,
            safety_margin_tokens: 0,
            accounting_policy: AccountingPolicy::AllowEstimated,
        },
        request_max_chars: 20000,
        max_concurrent_requests: 1,
        max_model_requests: 100,
        max_locate_lines: 80,
    }
}

#[test]
fn selection_cut_fields_name_exact_candidate_and_shared_cuts_without_rewriting_text() {
    let mut raw = text_fields("body", "body ".repeat(500));
    raw.extend(text_fields("summary", "summary ".repeat(400)));
    let cut = candidate(raw, 600, 0);
    let literal = candidate(
        text_fields(
            "body",
            "Authored [cut] and [cut: 2 more] stay literal.".into(),
        ),
        1000,
        1,
    );
    let context = shared(text_fields("outline", "context ".repeat(500)), 400);
    let expected_cut = serde_json::to_value(&cut.selection_view).unwrap();
    let expected_context = serde_json::to_value(&context.view).unwrap();
    assert_eq!(expected_cut["cut"], json!(["body", "summary"]));
    assert_eq!(expected_context["cut"], json!(["outline"]));

    let request = request_for(&selection(), &[&cut, &literal], &context).unwrap();

    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(
        wire["state"]["candidates"]["c1"]["cut-fields"],
        json!(["body", "summary"])
    );
    assert_eq!(wire["state"]["shared"]["cut-fields"], json!(["outline"]));
    for name in ["body", "summary"] {
        assert_eq!(
            wire["state"]["candidates"]["c1"][name],
            expected_cut["fields"][name]
        );
    }
    assert_eq!(
        wire["state"]["shared"]["outline"],
        expected_context["fields"]["outline"]
    );
    assert!(
        wire["state"]["candidates"]["c2"]
            .get("cut-fields")
            .is_none()
    );
    assert_eq!(
        wire["state"]["candidates"]["c2"]["body"],
        "Authored [cut] and [cut: 2 more] stay literal."
    );
    assert_eq!(
        wire["questions"]["c1"]["instructions"]["check"],
        "candidates.c1.body"
    );
    assert_eq!(
        wire["questions"]["c1"]["instructions"]["context"],
        json!(["shared.outline"])
    );
}

#[test]
fn selection_cut_fields_include_list_reductions_and_change_only_cut_bearing_request_hashes() {
    let list = FieldValue::Texts {
        values: vec!["short item ".repeat(5); 40],
        spans: vec![],
    };
    let item = candidate(
        BTreeMap::from([(FieldName::new("body").unwrap(), list)]),
        500,
        0,
    );
    let context = shared(text_fields("outline", "shared context".into()), 500);
    let view = serde_json::to_value(&item.selection_view).unwrap();
    assert_eq!(view["cut"], json!(["body"]));
    assert!(
        view["fields"]["body"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("[cut:")
    );

    let request = request_for(&selection(), &[&item], &context).unwrap();

    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(
        wire["state"]["candidates"]["c1"]["cut-fields"],
        json!(["body"])
    );
    assert_eq!(
        wire["state"]["candidates"]["c1"]["body"],
        view["fields"]["body"]
    );
    assert_eq!(wire["state"]["shared"], json!({"outline":"shared context"}));
    let mut legacy = wire.clone();
    legacy["state"]["candidates"]["c1"]
        .as_object_mut()
        .unwrap()
        .remove("cut-fields");
    let digest =
        |value: &Value| sunstack_evidence::content_digest(&serde_json::to_vec(value).unwrap());
    assert_ne!(digest(&wire), digest(&legacy));

    let uncut = candidate(text_fields("body", "Short source".into()), 500, 0);
    let current =
        serde_json::to_value(request_for(&selection(), &[&uncut], &context).unwrap()).unwrap();
    let expected = json!({"questions":current["questions"],"state":{
        "where":{"file":context.file},"shared":{"outline":"shared context"},"candidates":{"c1":{"body":"Short source"}}
    }});
    assert_eq!(current, expected);
    assert_eq!(digest(&current), digest(&expected));
}

#[test]
fn selection_cut_fields_count_toward_the_existing_exact_request_size_limit() {
    let item = candidate(text_fields("body", "source ".repeat(400)), 500, 0);
    let context = shared(text_fields("outline", "shared context".into()), 500);
    let current = request_for(&selection(), &[&item], &context).unwrap();
    let mut old_wire = serde_json::to_value(&current).unwrap();
    old_wire["state"]["candidates"]["c1"]
        .as_object_mut()
        .unwrap()
        .remove("cut-fields");
    let legacy = serde_json::from_value(old_wire).unwrap();
    let mut settings = dispatch_settings();
    let BackendSelection::JevOpenrouter { model, .. } = &settings.backend;
    let old_size = std::str::from_utf8(&encode_judgment_body(&legacy, model).unwrap())
        .unwrap()
        .chars()
        .count();
    settings.request_max_chars = old_size;
    let backend = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_SIZE_CONTROL").unwrap(),
    );
    let dispatcher = ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());

    assert!(dispatcher.assess_selection(&legacy).is_ok());
    assert!(matches!(
        dispatcher.assess_selection(&current),
        Err(SelectionIncompleteReason::RequestTooLarge {
            limit: crate::RequestLimit::RequestMaxChars
        })
    ));
}

#[tokio::test]
async fn selection_cut_fields_replay_only_matching_request_hashes_and_keep_uncut_answers() {
    for cut in [true, false] {
        let item = candidate(
            text_fields(
                "body",
                if cut {
                    "source ".repeat(400)
                } else {
                    "short source".into()
                },
            ),
            500,
            0,
        );
        let context = shared(text_fields("outline", "shared context".into()), 500);
        let selection = selection();
        let current = request_for(&selection, &[&item], &context).unwrap();
        let mut old_wire = serde_json::to_value(&current).unwrap();
        old_wire["state"]["candidates"]["c1"]
            .as_object_mut()
            .unwrap()
            .remove("cut-fields");
        let legacy: StructuredJudgmentRequest = serde_json::from_value(old_wire).unwrap();
        let settings = dispatch_settings();
        let backend = Arc::new(
            JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_REPLAY_CONTROL")
                .unwrap(),
        );
        let response = serde_json::to_vec(&json!({"model":"typesafe/selection-control",
            "answers":{"c1":{"type":"noul","noul":0.93}},
            "usage":{"input_tokens":10,"output_tokens":1}
        }))
        .unwrap();
        let (answers, receipt) = backend.admit_response(&legacy, &response).unwrap();
        let record = crate::RecordedSelection {
            source_request_id: 17,
            source_revision: Some("old-selection-shape".into()),
            selection_kind: selection.kind.clone(),
            input_sha256: crate::selection_input_hash(&legacy).unwrap(),
            request: legacy,
            outcome: DecisionRunOutcome::Completed {
                answers,
                receipt,
                attempts: vec![],
            },
        };
        let file = item.location.file.clone();
        let input = SelectionInput {
            selection,
            candidates: vec![item],
            shared: vec![context],
            files: BTreeSet::from([file]),
        };
        let mut dispatch = ModelRequestDispatch::new(
            settings.clone(),
            Some(backend.clone()),
            CancellationToken::new(),
        )
        .prepare_only()
        .with_recorded_selections(vec![record.clone()]);

        let outcomes = select_candidates(&input, &mut dispatch).await;

        assert_eq!(outcomes.len(), 1);
        assert!(dispatch.records.is_empty(), "preparation must not dispatch");
        assert_eq!(dispatch.prepared.len(), 1);
        if cut {
            assert!(dispatch.reused_selections.is_empty());
            assert!(matches!(
                outcomes.values().next().unwrap(),
                SelectionOutcome::Prepared { .. }
            ));
            let matching = crate::RecordedSelection {
                source_request_id: 18,
                source_revision: Some("matching-cut-shape".into()),
                request: current.clone(),
                input_sha256: crate::selection_input_hash(&current).unwrap(),
                ..record
            };
            let mut matching_dispatch =
                ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new())
                    .prepare_only()
                    .with_recorded_selections(vec![matching]);
            let matching_outcomes = select_candidates(&input, &mut matching_dispatch).await;
            assert!(matches!(matching_outcomes.values().next().unwrap(),
                SelectionOutcome::Selected { yes } if *yes == 0.93));
            assert_eq!(matching_dispatch.reused_selections[0].source_request_id, 18);
            let prepared = serde_json::to_value(&matching_dispatch.prepared[0].request).unwrap();
            assert_eq!(
                prepared["state"]["candidates"]["c1"]["cut-fields"],
                json!(["body"])
            );
        } else {
            assert_eq!(dispatch.reused_selections.len(), 1);
            assert_eq!(dispatch.reused_selections[0].source_request_id, 17);
            assert!(
                matches!(outcomes.values().next().unwrap(), SelectionOutcome::Selected { yes } if *yes==0.93)
            );
        }
    }
}
