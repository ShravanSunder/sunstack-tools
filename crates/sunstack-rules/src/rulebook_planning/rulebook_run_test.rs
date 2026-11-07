#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use crate::{FilterAnswer, FilterAnswerState, FilterAnswers, FilterIncomplete, OnlyWhenGate};
use sunstack_decisions::{ChoiceLabel, ModelName};

#[tokio::test(flavor = "current_thread")]
async fn filter_gate_prevents_real_first_walk_and_deduplicates_skips() {
    for marker in ["K001", "K002"] {
        let filter = crate::FilterName::new("purpose").unwrap();
        let a = ChoiceLabel::new("a").unwrap();
        let b = ChoiceLabel::new("b").unwrap();
        let mut rule = fixture::rule(marker);
        rule.gate = Some(OnlyWhenGate {
            filter: filter.clone(),
            answers: [a.clone()].into(),
        });
        let piece = fixture::piece();
        rule.kind = piece.kind.clone();
        let settings = RulebookRunSettings {
            backend: BackendSelection::JevOpenrouter {
                model: ModelName::new("jev-latest").unwrap(),
                endpoint: "http://127.0.0.1:1/v1/systemone".into(),
                context_ceiling_tokens: None,
            },
            attempts: AttemptSettings::default(),
            request_budget: RequestBudgetSettings {
                max_input_tokens: 100000,
                max_request_bytes: 100000,
                max_questions: 10,
                max_requests: 10,
                safety_margin_tokens: 0,
                accounting_policy: sunstack_decisions::AccountingPolicy::AllowEstimated,
            },
            request_max_chars: 100000,
            max_concurrent_requests: 1,
            max_model_requests: 10,
            max_locate_lines: 100,
        };
        let mut dispatcher = ModelRequestDispatch::new(settings, None, CancellationToken::new());
        let mut answers = FilterAnswers {
            prepared: vec![],
            pieces: Default::default(),
            not_sent: vec![],
            entries: vec![],
            requests: vec![],
            folders: BTreeMap::from([(
                (filter.clone(), piece.location.file.clone()),
                "folder".into(),
            )]),
        };
        for state in [
            FilterAnswerState::Judged {
                label: b.clone(),
                path: vec![],
                answers: BTreeMap::new(),
            },
            FilterAnswerState::Unsure {
                step: crate::StepName::new("kind").unwrap(),
                path: vec![],
                answers: BTreeMap::new(),
            },
            FilterAnswerState::Incomplete {
                reason: FilterIncomplete::ReadsAbsent,
                field: None,
                path: vec![],
                answers: BTreeMap::new(),
            },
        ] {
            answers.entries = vec![FilterAnswer {
                reads_sha256: None,
                filter: filter.clone(),
                folder: "folder".into(),
                input_cut: false,
                requested_model: ModelName::new("jev-latest").unwrap(),
                resolved_model: None,
                state,
            }];
            FIRST_WALKS.with(|walks| walks.set(0));
            let run = run_rulebook_with_dispatcher(
                &[rule.clone()],
                &[piece.clone(), piece.clone(), piece.clone()],
                &BTreeMap::new(),
                &mut dispatcher,
                &answers,
            )
            .await;
            assert_eq!(FIRST_WALKS.with(|walks| walks.get()), 0);
            if matches!(answers.entries[0].state, FilterAnswerState::Judged { .. }) {
                assert_eq!(run.results.len(), 3);
                assert!(run.skips.is_empty());
                for result in run.results {
                    assert!(matches!(
                        result.result,
                        RulebookResult::NotApplicable { .. }
                    ));
                    assert_eq!(result.result.basis(), Some(crate::Basis::Judged));
                }
            } else {
                assert!(run.results.is_empty());
                assert_eq!(run.skips.len(), 1);
            }
        }
        answers.entries[0].state = FilterAnswerState::Judged {
            label: a.clone(),
            path: vec![],
            answers: BTreeMap::new(),
        };
        FIRST_WALKS.with(|walks| walks.set(0));
        let run = run_rulebook_with_dispatcher(
            &[rule.clone()],
            std::slice::from_ref(&piece),
            &BTreeMap::new(),
            &mut dispatcher,
            &answers,
        )
        .await;
        assert_eq!(FIRST_WALKS.with(|walks| walks.get()), 1);
        assert_eq!(run.results.len(), 1);
        FIRST_WALKS.with(|walks| walks.set(0));
        let run = run_rulebook_with_dispatcher(
            &[rule],
            &[piece],
            &BTreeMap::new(),
            &mut dispatcher,
            &FilterAnswers::default(),
        )
        .await;
        assert_eq!(FIRST_WALKS.with(|walks| walks.get()), 0);
        assert!(run.results.is_empty());
        assert!(matches!(
            run.skips[0].why,
            crate::FilterSkipReason::NoFolder
        ));
    }
}
