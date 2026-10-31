#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/controlled_http.rs"]
mod controlled;
#[path = "../src/rulebook_planning/rulebook_test_support.rs"]
mod rule_fixture;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use sunstack_decisions::*;
use sunstack_evidence::*;
use sunstack_rules::*;
fn root() -> PathBuf {
    rule_fixture::fixture().join("selection")
}
fn input(file: &str) -> SelectionInput {
    let root = root();
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let mut kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition> =
        serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    kinds.retain(|kind, _| kind.as_str() == "workflow_section");
    let selection = admit_selections(&kinds).unwrap().remove(0);
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let file = PathBuf::from(format!("skills/{file}/SKILL.md"));
    let mut source = FilesystemCheckedSource::new(&root).unwrap();
    let mut build = EvidenceBuild::start(
        &project,
        &kinds,
        std::slice::from_ref(&file),
        &mut source,
        PieceBuildLimits {
            max_files: 100,
            max_file_bytes: 100000,
            max_total_bytes: 1000000,
            max_matches: 10000,
            evidence_max_chars: 6000,
        },
    )
    .unwrap();
    let BuildStep::NeedsSelection {
        candidates, shared, ..
    } = build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!()
    };
    SelectionInput {
        selection,
        candidates,
        shared,
        files: BTreeSet::from([file]),
    }
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
            max_questions: 32,
            max_requests: 100,
            safety_margin_tokens: 0,
            accounting_policy: AccountingPolicy::AllowEstimated,
        },
        request_max_chars: 20000,
        max_concurrent_requests: 1,
        max_model_requests: 100,
    }
}
fn dispatcher(settings: RulebookRunSettings) -> ModelRequestDispatch {
    let backend = Arc::new(
        JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_SELECTION_KEY").unwrap(),
    );
    ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new())
}
#[test]
fn selection_admission_refuses_each_crafted_cause_and_accepts_the_canonical_config() {
    let root = root();
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    assert!(admit_evidence_kinds(&kinds, &project, 6000).is_empty());
    for (name, expected) in [
        ("choice-question", "noul"),
        ("invalid-question", "schema"),
        ("high-threshold", "threshold"),
        ("equal-threshold", "threshold"),
        ("shared-undefined", "same_file"),
        ("shared-first-line", "same_file"),
        ("question-undefined", "defined fields"),
        ("question-target", "target field"),
        ("within-undefined", "undefined kind"),
        ("within-cycle", "cycle"),
        ("kind-cycle", "cycle"),
        ("kind-undefined", "undefined"),
        ("exclude-undefined", "undefined"),
        ("as-kind-selected", "selected"),
        ("as-kind-dependent", "selection-dependent"),
    ] {
        let config: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!("refusals/{name}.json"))).unwrap(),
        )
        .unwrap();
        let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
        let errors = admit_evidence_kinds(&kinds, &project, 6000);
        let public_paths = errors
            .iter()
            .map(|error| error.field_path.to_string())
            .collect::<BTreeSet<_>>();
        let expected_paths: &[&str] = match name {
            "choice-question" | "invalid-question" => &[
                "evidence.kinds.selected_targets.select.question",
                "evidence.kinds.workflow_section.select.question",
            ],
            "high-threshold" | "equal-threshold" => &[
                "evidence.kinds.selected_targets.select",
                "evidence.kinds.workflow_section.select",
            ],
            "shared-undefined" | "shared-first-line" => &[
                "evidence.kinds.selected_targets.select.shared",
                "evidence.kinds.workflow_section.select.shared",
            ],
            "question-undefined" => &[
                "evidence.kinds.selected_targets.select.question.instructions.check",
                "evidence.kinds.workflow_section.select.question.instructions.check",
            ],
            "question-target" => &["evidence.kinds.selected_targets.select.question"],
            "within-undefined" | "within-cycle" | "kind-cycle" => &["evidence.kinds.step"],
            "kind-undefined" => &["evidence.kinds.load", "evidence.kinds.load.fields.step"],
            "exclude-undefined" => &[
                "evidence.kinds.workflow_section",
                "evidence.kinds.workflow_section.fields.body",
            ],
            "as-kind-selected" => &["evidence.kinds.a_load"],
            "as-kind-dependent" => &["evidence.kinds.a_load", "evidence.kinds.selected_targets"],
            _ => panic!(),
        };
        assert_eq!(
            public_paths,
            expected_paths
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
            "public field spelling changed for {name}"
        );
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "{name}: {errors:?}"
        );
    }
}
#[tokio::test]
async fn selection_thresholds_are_per_candidate_with_qualified_questions_and_no_raw_or_target_state()
 {
    let mut input = input("thirty-three");
    input.candidates.truncate(6);
    let expected_question = input.selection.question.clone();
    let mut server = controlled::ControlledServer::start().await;
    let dispatch = dispatcher(settings(&server.endpoint));
    let operation = tokio::spawn(async move {
        let mut dispatch = dispatch;
        let outcomes = select_candidates(&input, &mut dispatch).await;
        (outcomes, dispatch.records)
    });
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 6);
    assert!(request.body["state"].get("text").is_none());
    assert!(request.body["state"].get("standard").is_none());
    assert_eq!(
        request.body["questions"]["c1"]["instructions"]["check"],
        "candidates.c1.body"
    );
    assert_eq!(
        request.body["questions"]["c1"]["instructions"]["context"],
        json!(["shared.outline"])
    );
    assert_eq!(
        request.body["questions"]["c1"]["instructions"]["task"],
        serde_json::to_value(expected_question).unwrap()["instructions"]["task"]
    );
    let probabilities = [0.69, 0.70, 0.71, 0.29, 0.30, 0.31];
    let answers = probabilities
        .iter()
        .enumerate()
        .map(|(index, probability)| {
            (
                format!("c{}", index + 1),
                json!({"type":"noul","noul":probability}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    request.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":10,"output_tokens":2}}));
    let (outcomes, records) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    let outcomes = outcomes
        .values()
        .map(|outcome| {
            serde_json::to_value(outcome).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            "uncertain",
            "selected",
            "selected",
            "not_selected",
            "not_selected",
            "uncertain"
        ]
    );
    assert_eq!(records.len(), 1);
    assert_eq!(server.finish().await, 1);
}
#[tokio::test]
async fn thirty_three_short_candidates_pack_at_the_real_question_limit_as_thirty_two_and_one() {
    let input = input("thirty-three");
    let mut server = controlled::ControlledServer::start().await;
    let mut dispatch = dispatcher(settings(&server.endpoint));
    let operation = tokio::spawn(async move {
        let outcomes = select_candidates(&input, &mut dispatch).await;
        (outcomes, dispatch.records)
    });
    let first = server.next().await;
    assert_eq!(first.body["questions"].as_object().unwrap().len(), 32);
    let shared = first.body["state"]["shared"].clone();
    first.noul(0.93);
    let second = server.next().await;
    assert_eq!(second.body["questions"].as_object().unwrap().len(), 1);
    assert!(second.body["questions"].get("c33").is_some());
    assert_eq!(second.body["state"]["shared"], shared);
    second.noul(0.93);
    let (outcomes, records) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcomes.len(), 33);
    assert!(
        outcomes
            .values()
            .all(|outcome| matches!(outcome, SelectionOutcome::Selected { .. }))
    );
    assert_eq!(records.len(), 2);
    assert_eq!(server.finish().await, 2);
}
#[tokio::test]
async fn unavailable_referenced_fields_block_only_their_candidate_and_input_token_limit_is_named() {
    let mut input = input("thirty-three");
    input.candidates.truncate(3);
    input.candidates[0].fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::FrontmatterMalformed,
        },
    );
    input.candidates[1].fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::NotAString,
        },
    );
    input.candidates[2].fields.insert(
        FieldName::new("unused").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::NotAString,
        },
    );
    input.candidates[2].selection_view = selection_model_view(
        &input.candidates[2]
            .fields
            .iter()
            .filter(|(name, _)| name.as_str() != "outline")
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        6000,
    );
    let mut server = controlled::ControlledServer::start().await;
    let mut dispatch = dispatcher(settings(&server.endpoint));
    let operation = tokio::spawn(async move {
        let outcomes = select_candidates(&input, &mut dispatch).await;
        (outcomes, dispatch)
    });
    let request = server.next().await;
    assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
    assert!(
        request.body["state"]["candidates"]["c3"]
            .get("unused")
            .is_none()
    );
    request.noul(0.9);
    let (outcomes, _) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        outcomes
            .values()
            .filter(|outcome| matches!(
                outcome,
                SelectionOutcome::Incomplete {
                    reason: SelectionIncompleteReason::EvidenceUnavailable { .. }
                }
            ))
            .count(),
        2
    );
    assert_eq!(server.finish().await, 1);
    let input = input_for_token_limit();
    let mut limited = settings("http://127.0.0.1:1/v1/systemone");
    limited.request_budget.max_input_tokens = 1;
    let mut dispatch = dispatcher(limited);
    let outcomes = select_candidates(&input, &mut dispatch).await;
    assert!(dispatch.records.is_empty());
    assert!(outcomes.values().all(|outcome| matches!(
        outcome,
        SelectionOutcome::Incomplete {
            reason: SelectionIncompleteReason::RequestTooLarge {
                limit: RequestLimit::InputTokens
            }
        }
    )));
}
fn input_for_token_limit() -> SelectionInput {
    input("practices-research")
}
#[tokio::test]
async fn selection_and_rule_work_share_one_request_allowance_and_rule_results_still_settle() {
    let mut selection = input("thirty-three");
    selection.candidates.truncate(2);
    // One-question groups make the selection consume two of the run's three requests.
    let mut server = controlled::ControlledServer::start().await;
    let mut limits = settings(&server.endpoint);
    limits.request_budget.max_questions = 1;
    limits.max_model_requests = 3;
    let mut dispatch = dispatcher(limits);
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
        let selected = select_candidates(&selection, &mut dispatch).await;
        let rules = run_rulebook_with_dispatcher(
            &[rule],
            &[piece.clone(), piece],
            &BTreeMap::new(),
            &mut dispatch,
        )
        .await;
        (selected, rules, dispatch.records)
    });
    for _ in 0..3 {
        let request = server.next().await;
        request.noul(0.93);
    }
    let (selected, rules, records) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(selected.len(), 2);
    assert_eq!(records.len(), 3);
    assert!(matches!(
        rules.results[0].result,
        RulebookResult::Pass { .. }
    ));
    assert!(matches!(
        rules.results[1].result,
        RulebookResult::Incomplete {
            reason: RulebookIncompleteReason::RequestLimitReached,
            ..
        }
    ));
    assert_eq!(server.finish().await, 3);
}
#[tokio::test]
async fn encoded_character_packing_keeps_consecutive_groups_and_names_singleton_overflow() {
    let mut input = input("thirty-three");
    input.candidates.truncate(5);
    for candidate in &mut input.candidates {
        candidate.fields.insert(
            FieldName::new("body").unwrap(),
            FieldValue::Text {
                value: "é".repeat(2000),
                spans: vec![],
            },
        );
        candidate.selection_view = selection_model_view(
            &candidate
                .fields
                .iter()
                .filter(|(name, _)| name.as_str() != "outline")
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            6000,
        );
    }
    let mut server = controlled::ControlledServer::start().await;
    let mut limits = settings(&server.endpoint);
    limits.request_max_chars = 8000;
    let mut dispatch = dispatcher(limits);
    let operation = tokio::spawn(async move {
        let outcomes = select_candidates(&input, &mut dispatch).await;
        (outcomes, dispatch.records)
    });
    let first = server.next().await;
    assert_eq!(
        first.body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        ["c1", "c2", "c3"]
    );
    assert!(
        std::str::from_utf8(&first.raw_body)
            .unwrap()
            .chars()
            .count()
            <= 8000
    );
    first.noul(0.9);
    let second = server.next().await;
    assert_eq!(
        second.body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        ["c4", "c5"]
    );
    second.noul(0.9);
    let (outcomes, records) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcomes.len(), 5);
    assert_eq!(records.len(), 2);
    assert_eq!(server.finish().await, 2);
    let mut input = input_for_token_limit();
    input.candidates[0].selection_view = ModelView::Bounded {
        fields: BTreeMap::from([(
            FieldName::new("body").unwrap(),
            ModelFieldValue::Text("x".repeat(21000)),
        )]),
        cut: vec![],
    };
    let mut dispatch = dispatcher(settings("http://127.0.0.1:1/v1/systemone"));
    let outcomes = select_candidates(&input, &mut dispatch).await;
    assert!(matches!(
        outcomes.values().next().unwrap(),
        SelectionOutcome::Incomplete {
            reason: SelectionIncompleteReason::RequestTooLarge {
                limit: RequestLimit::RequestMaxChars
            }
        }
    ));
    assert!(dispatch.records.is_empty());
}
#[tokio::test]
async fn provider_failure_is_contained_to_its_file_while_the_other_file_selects() {
    let mut first = input("practices-research");
    let second = input("presentation-tui");
    first.files.extend(second.files);
    first.candidates.extend(second.candidates);
    first.shared.extend(second.shared);
    let mut server = controlled::ControlledServer::start().await;
    let mut dispatch = dispatcher(settings(&server.endpoint));
    let operation = tokio::spawn(async move {
        let outcomes = select_candidates(&first, &mut dispatch).await;
        (outcomes, dispatch.records)
    });
    let request = server.next().await;
    assert!(
        request.body["state"]["where"]["file"]
            .as_str()
            .unwrap()
            .contains("practices-research")
    );
    request.respond(500, json!({"error":"controlled failure"}));
    server.next().await.noul(0.93);
    let (outcomes, records) = tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().any(|(id, outcome)| {
        id.file.to_string_lossy().contains("practices-research")
            && matches!(
                outcome,
                SelectionOutcome::Incomplete {
                    reason: SelectionIncompleteReason::ProviderFailure {
                        failure_class: ProviderFailureClass::ProviderUnavailable,
                        http_status: Some(500)
                    }
                }
            )
    }));
    assert!(outcomes.iter().any(|(id, outcome)| {
        id.file.to_string_lossy().contains("presentation-tui")
            && matches!(outcome, SelectionOutcome::Selected { .. })
    }));
    assert_eq!(records.len(), 2);
    assert_eq!(server.finish().await, 2);
}
#[tokio::test]
async fn unequal_candidates_share_one_independently_bounded_shared_view_in_each_group() {
    let mut input = input("thirty-three");
    input.candidates.truncate(2);
    let outline = FieldValue::Texts {
        values: vec!["outline".repeat(120); 12],
        spans: vec![],
    };
    input.shared[0].fields = BTreeMap::from([(FieldName::new("outline").unwrap(), outline)]);
    input.shared[0].view = selection_model_view(&input.shared[0].fields, 1000);
    let expected = match &input.shared[0].view {
        ModelView::Bounded { fields, .. } => serde_json::to_value(fields).unwrap(),
        _ => panic!(),
    };
    for (candidate, length) in input.candidates.iter_mut().zip([2000, 700]) {
        candidate.fields.insert(
            FieldName::new("body").unwrap(),
            FieldValue::Text {
                value: "x".repeat(length),
                spans: vec![],
            },
        );
        let fields = candidate
            .fields
            .iter()
            .filter(|(name, _)| name.as_str() != "outline")
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        candidate.selection_view = selection_model_view(&fields, 3000);
    }
    let mut server = controlled::ControlledServer::start().await;
    let mut limits = settings(&server.endpoint);
    limits.request_max_chars = 3800;
    let mut dispatch = dispatcher(limits);
    let operation = tokio::spawn(async move { select_candidates(&input, &mut dispatch).await });
    for _ in 0..2 {
        let request = server.next().await;
        assert_eq!(request.body["questions"].as_object().unwrap().len(), 1);
        assert_eq!(request.body["state"]["shared"], expected);
        request.noul(0.93);
    }
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), operation)
            .await
            .unwrap()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(server.finish().await, 2);
}

#[path = "evidence_selection/incomplete_reasons.rs"]
mod incomplete_reasons;
