#![allow(clippy::unwrap_used)]
use super::super::*;
use sunstack_decisions::ModelName;
use sunstack_rules::{
    FilterAnswer, FilterAnswerState, FilterIncomplete, FilterName, FilterSkip, FilterSkipReason,
};

#[test]
fn filter_status_preserves_cancelled_incomplete_findings_and_uncertainty_priority() {
    for (cancelled, incomplete, findings, expected) in [
        (false, false, false, LintStatus::Uncertain),
        (false, false, true, LintStatus::Findings),
        (false, true, true, LintStatus::Incomplete),
        (true, true, true, LintStatus::Cancelled),
    ] {
        let mut report = LintReport::empty(
            "config.json".into(),
            ScopeChoice::All,
            EffectiveLimits::default(),
        );
        report.counts.error = u32::from(findings);
        report.filter_answers.push(IdentifiedResult {
            id: 1,
            request_ids: vec![],
            value: FilterAnswer {
                reads_sha256: None,
                filter: FilterName::new("purpose").unwrap(),
                folder: "packages/one".into(),
                input_cut: false,
                requested_model: ModelName::new("jev-latest").unwrap(),
                resolved_model: None,
                state: FilterAnswerState::Unsure {
                    step: sunstack_rules::StepName::new("purpose").unwrap(),
                    path: vec![sunstack_rules::StepName::new("purpose").unwrap()],
                    answers: Default::default(),
                },
            },
        });
        if incomplete {
            report.filter_answers[0].value.state = FilterAnswerState::Incomplete {
                reason: FilterIncomplete::ReadsAbsent,
                field: None,
                path: vec![],
                answers: Default::default(),
            };
        }
        let run = RulebookRun {
            prepared: vec![],
            not_sent: vec![],
            results: vec![],
            requests: vec![],
            skips: vec![],
        };
        let result = project_outcome(run, &[], &[], report, None, cancelled);
        assert_eq!(result.status, expected);
    }
    let report = LintReport::empty(
        "config.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    let run = RulebookRun {
        prepared: vec![],
        not_sent: vec![],
        results: vec![],
        requests: vec![],
        skips: vec![FilterSkip {
            locations: vec![],
            rule: sunstack_rules::RuleId::new("check").unwrap(),
            file: "outside.md".into(),
            filter: FilterName::new("purpose").unwrap(),
            folder: None,
            why: FilterSkipReason::NoFolder,
            path: vec![],
            answers: Default::default(),
        }],
    };
    let report = project_outcome(run, &[], &[], report, None, false);
    assert_eq!(report.status, LintStatus::Incomplete);
    assert!(matches!(
        report.incomplete[0].reason,
        IncompleteReason::NoFilterFolder { .. }
    ));
}

#[tokio::test]
async fn runtime_filter_choice_boundary_preserves_typed_count_limit_through_report_projection() {
    use serde_json::json;
    use std::{collections::BTreeMap, sync::Arc};
    use sunstack_decisions::*;
    use sunstack_evidence::*;
    use sunstack_rules::test_support::controlled_http::ControlledServer;
    use sunstack_rules::*;
    for count in [255, 256] {
        let mut server = ControlledServer::start().await;
        let name = FilterName::new("purpose").unwrap();
        let criteria = (0..count)
            .map(|index| {
                (
                    ChoiceLabel::new(format!("option{index}")).unwrap(),
                    StructuredEntry::Null,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let plan = FilterPlan {
            name: name.clone(),
            per: FolderPattern::new("packages/{name}").unwrap(),
            reads: ReadsSpec {
                file: "about.md".into(),
                kind: EvidenceKindName::new("skill").unwrap(),
                fields: vec![FieldName::new("body").unwrap()],
            },
            answers: criteria.keys().cloned().collect(),
            questions: BTreeMap::from([(
                QuestionName::new("filter").unwrap(),
                JudgmentQuestion::Choice {
                    instructions: serde_json::from_value(json!({"check":"body"})).unwrap(),
                    criteria,
                },
            )]),
            reference: BTreeMap::new(),
            decision: DecisionGraph {
                start: StepName::new("purpose").unwrap(),
                steps: BTreeMap::from([(
                    StepName::new("purpose").unwrap(),
                    DecisionStep::AskChoice {
                        question: QuestionName::new("filter").unwrap(),
                        min_probability: 0.7,
                        routes: (0..count)
                            .map(|index| {
                                (
                                    ChoiceLabel::new(format!("option{index}")).unwrap(),
                                    DecisionRoute::Label(
                                        ChoiceLabel::new(format!("option{index}")).unwrap(),
                                    ),
                                )
                            })
                            .collect(),
                        uncertain: DecisionRoute::Uncertain,
                    },
                )]),
            },
        };
        let span = SourceSpan {
            source: SourceRef::Document {
                repository: RepositoryId::new("fixture").unwrap(),
                path: "about.md".into(),
                content_identity: "fixture".into(),
            },
            byte_start: 0,
            byte_end: 7,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 8,
        };
        let fields = BTreeMap::from([(
            FieldName::new("body").unwrap(),
            FieldValue::Text {
                value: "Purpose".into(),
                spans: vec![span.clone()],
            },
        )]);
        let piece = EvidencePiece {
            kind: EvidenceKindName::new("skill").unwrap(),
            location: PieceLocation {
                file: "about.md".into(),
                first_line: 1,
                last_line: 1,
                span,
            },
            text: "Purpose".into(),
            model_view: project_model_view(&fields, 1000),
            fields,
        };
        let settings = RulebookRunSettings {
            backend: BackendSelection::JevOpenrouter {
                endpoint: server.endpoint.clone(),
                model: ModelName::new("jev-latest").unwrap(),
                context_ceiling_tokens: None,
            },
            attempts: AttemptSettings {
                retry_count: 0,
                ..Default::default()
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
            max_concurrent_requests: 1,
            max_model_requests: 10,
            max_locate_lines: 80,
        };
        let backend =
            Arc::new(JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC").unwrap());
        let fixture_root = tempfile::tempdir().unwrap();
        crate::test_support::lint_fixture::write(fixture_root.path());
        let project = load_evidence_project(
            &fixture_root.path().join("evidence/sgconfig.yml"),
            &[],
            None,
            &Default::default(),
        )
        .unwrap();
        let filter_kinds = serde_json::from_value(
            json!({"skill":{"rule":"skill","fields":{"body":{"from":"remainder"}}}}),
        )
        .unwrap();
        let peer_piece = piece.clone();
        let peer_settings = settings.clone();
        let mut task = tokio::spawn(async move {
            let mut dispatcher =
                ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());
            ask_filters(
                &BTreeMap::from([(name.clone(), plan)]),
                &[FilterFolder {
                    filter: name,
                    folder: "packages/one".into(),
                    members: vec!["packages/one/member.md".into()],
                    reads: ReadsPiece::One(0),
                }],
                FilterEvidence {
                    pieces: &[piece],
                    kinds: &filter_kinds,
                    project: &project,
                    matches: &BTreeMap::new(),
                },
                1000,
                &mut dispatcher,
            )
            .await
        });
        let mut bodies = Vec::new();
        let answers = loop {
            tokio::select! {biased;result=&mut task=>break result.unwrap(),pending=server.receive()=>{
                let criteria=pending.body["questions"]["filter"]["criteria"].as_object().unwrap();
                let probabilities=criteria.keys().map(|label|(label.clone(),json!(if label=="option0" {1.0}else{0.0}))).collect::<serde_json::Map<_,_>>();
                bodies.push(pending.body.clone());pending.respond(200,json!({"model":"typesafe/controlled","answers":{"filter":{"type":"choice","choice":"option0","confidence":1.0,"probabilities":probabilities}},"usage":{"input_tokens":2,"output_tokens":1}}));
            }}
        };
        assert_eq!(server.finish().await, bodies.len());
        let mut report = LintReport::empty(
            "config.json".into(),
            ScopeChoice::All,
            EffectiveLimits::default(),
        );
        report.filter_answers = answers
            .entries
            .into_iter()
            .map(|value| IdentifiedResult {
                id: 1,
                request_ids: vec![],
                value,
            })
            .collect();
        let peer_step = StepName::new("peer_check").unwrap();
        let peer_rule = RulebookRule {
            reference: BTreeMap::new(),
            gate: None,
            name: RuleId::new("peer").unwrap(),
            sentence: RulebookSentence {
                marker: MarkerId::new("K001").unwrap(),
                rulebook: sunstack_evidence::DefinitionOrigin::file("rulebook.md"),
                line: 1,
                standard: "Check purpose".into(),
            },
            kind: peer_piece.kind.clone(),
            questions: BTreeMap::new(),
            decision: DecisionGraph {
                start: peer_step.clone(),
                steps: BTreeMap::from([(
                    peer_step,
                    DecisionStep::Mechanical {
                        tool: DecisionMechanicalTool::Contains {
                            field: FieldName::new("body").unwrap(),
                            text: "Purpose".into(),
                        },
                        yes: DecisionRoute::Pass,
                        no: DecisionRoute::NotApplicable,
                    },
                )]),
            },
            severity: Severity::Warn,
            related: vec![],
        };
        let peer_run = run_rulebook(
            std::slice::from_ref(&peer_rule),
            std::slice::from_ref(&peer_piece),
            &BTreeMap::new(),
            peer_settings,
            None,
            CancellationToken::new(),
        )
        .await;
        let report = project_outcome(peer_run, &[peer_rule], &[peer_piece], report, None, false);
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["results"][0]["outcome"], "pass");
        assert_eq!(value["results"][0]["basis"], "mechanical");
        assert_eq!(value["filter_answers"][0]["filter"], "purpose");
        assert_eq!(value["filter_answers"][0]["folder"], "packages/one");
        assert_eq!(value["filter_answers"][0]["requested_model"], "jev-latest");
        if count == 255 {
            assert_eq!(bodies.len(), 1);
            assert_eq!(
                bodies[0]["questions"]["filter"]["criteria"]
                    .as_object()
                    .unwrap()
                    .len(),
                255
            );
            assert_eq!(value["filter_answers"][0]["state"], "judged");
        } else {
            assert!(bodies.is_empty());
            assert_eq!(report.status, LintStatus::Incomplete);
            let expected = json!({"options":256,"limit":255});
            assert_eq!(
                value["filter_answers"][0]["reason"]["reason"]["choice_options_exceeded"],
                expected
            );
            assert_eq!(
                value["incomplete"][0]["reason"]["reason"]["reason"]["choice_options_exceeded"],
                expected
            );
            let human = render(&report, OutputFormat::Default);
            assert!(
                human.contains("choice_options_exceeded")
                    && human.contains("256")
                    && human.contains("255"),
                "{human}"
            );
            let schema = jsonschema::validator_for(report_schema().as_value()).unwrap();
            assert!(schema.is_valid(&value));
            for field in ["options", "limit"] {
                let mut bad = value.clone();
                bad["filter_answers"][0]["reason"]["reason"]["choice_options_exceeded"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
                assert!(!schema.is_valid(&bad));
            }
        }
    }
}
