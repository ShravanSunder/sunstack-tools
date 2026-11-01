#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::{
    Locator, RulebookRunSettings, piece_state, rulebook_planning::rulebook_test_support as fixture,
};
use std::sync::Arc;
use sunstack_decisions::*;
use sunstack_evidence::{FieldValue, project_model_view};

fn piece() -> EvidencePiece {
    let mut piece = fixture::piece();
    piece.text = (49..=60).map(|line| format!("line{line}\n")).collect();
    piece.location.span.byte_start = 100;
    piece.location.span.byte_end = 100 + piece.text.len();
    piece.location.span.start_line = 49;
    piece.location.span.end_line = 61;
    piece.location.span.end_column = 1;
    let mut span = piece.location.span.clone();
    span.byte_start += "line49\nline50\n".len();
    span.start_line = 51;
    piece.fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Text {
            value: piece.text["line49\nline50\n".len()..].into(),
            spans: vec![span],
        },
    );
    piece.model_view = project_model_view(&piece.fields, 10000);
    piece
}

#[test]
fn candidate_line_sets_intersect_piece_bytes_deduplicate_spans_and_omit_empty_fields() {
    let mut piece = piece();
    let field = FieldName::new("body").unwrap();
    assert_eq!(
        candidate_lines(&piece, &field),
        (51..=60)
            .map(|line| (line, format!("line{line}")))
            .collect()
    );
    let mut enclosing = piece.location.span.clone();
    enclosing.byte_start = 0;
    enclosing.byte_end += 100;
    enclosing.start_line = 1;
    enclosing.end_line = 80;
    piece.fields.insert(
        field.clone(),
        FieldValue::Text {
            value: "enclosing context".into(),
            spans: vec![enclosing.clone(), enclosing],
        },
    );
    assert_eq!(
        candidate_lines(&piece, &field),
        (49..=60)
            .map(|line| (line, format!("line{line}")))
            .collect()
    );
    piece.fields.insert(field.clone(), FieldValue::Absent);
    assert!(candidate_lines(&piece, &field).is_empty());
    let mut empty = piece.location.span.clone();
    empty.byte_end = empty.byte_start;
    piece.fields.insert(
        field.clone(),
        FieldValue::Text {
            value: "".into(),
            spans: vec![empty],
        },
    );
    assert!(candidate_lines(&piece, &field).is_empty());
}

#[tokio::test]
async fn locators_fit_only_after_every_authored_question_and_respect_each_request_budget() {
    for case in [
        "fits",
        "exact_lines",
        "one_line",
        "lines",
        "questions",
        "chars",
        "bytes",
        "absent",
    ] {
        let mut rule = fixture::rule("K001");
        let DecisionStep::Combine { routes, .. } = rule
            .decision
            .steps
            .get_mut(&crate::StepName::new("quality").unwrap())
            .unwrap()
        else {
            panic!()
        };
        let DecisionRoute::Finding(finding) = &mut routes.no else {
            panic!()
        };
        finding.at = Some(FieldName::new("body").unwrap());
        finding.locate = Some(Locator {
            name: QuestionName::new("focus_line").unwrap(),
            task: if ["chars", "bytes"].contains(&case) {
                "x".repeat(100001)
            } else {
                "Choose a line.".into()
            },
            min_probability: 0.7,
        });
        let mut piece = piece();
        if case == "absent" {
            piece
                .fields
                .insert(FieldName::new("body").unwrap(), FieldValue::Absent);
        }
        if case == "one_line" {
            let FieldValue::Text { spans, value } = piece
                .fields
                .get_mut(&FieldName::new("body").unwrap())
                .unwrap()
            else {
                panic!()
            };
            *value = "line51\n".into();
            spans[0].byte_end = spans[0].byte_start + "line51\n".len();
            spans[0].end_line = 52;
        }
        piece.model_view = project_model_view(&piece.fields, 10000);
        let settings = RulebookRunSettings {
            backend: BackendSelection::JevOpenrouter {
                endpoint: "http://127.0.0.1:1/v1/systemone".into(),
                model: ModelName::new("fixture").unwrap(),
                context_ceiling_tokens: None,
            },
            attempts: AttemptSettings::default(),
            request_budget: RequestBudgetSettings {
                max_input_tokens: 1000000,
                max_request_bytes: 100000,
                max_questions: if case == "questions" { 3 } else { 4 },
                max_requests: 10,
                safety_margin_tokens: 0,
                accounting_policy: AccountingPolicy::AllowEstimated,
            },
            request_max_chars: if case == "bytes" { 1000000 } else { 100000 },
            max_concurrent_requests: 1,
            max_model_requests: 10,
            max_locate_lines: if case == "lines" {
                9
            } else if case == "exact_lines" {
                10
            } else {
                80
            },
        };
        let backend = Arc::new(
            JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC_NO_DISPATCH").unwrap(),
        );
        let dispatcher =
            ModelRequestDispatch::new(settings, Some(backend), CancellationToken::new());
        let authored = rule.questions.clone();
        assert_eq!(authored.len(), 3);
        let mut request = StructuredJudgmentRequest {
            questions: authored.clone(),
            state: piece_state(&rule, &piece).unwrap(),
        };
        add_locators(&rule, &piece, &dispatcher, &mut request);
        let sent = ["fits", "exact_lines", "one_line"].contains(&case);
        assert_eq!(request.questions.len(), if sent { 4 } else { 3 }, "{case}");
        for (name, question) in &authored {
            assert_eq!(
                serde_json::to_value(request.questions.get(name)).unwrap(),
                serde_json::to_value(question).unwrap()
            );
        }
        if sent {
            assert!(
                sunstack_decisions::judgment_processing::judgment_validation::validate_questions(
                    &request.questions
                )
                .is_ok()
            );
            let JudgmentQuestion::Choice { criteria, .. } =
                &request.questions[&QuestionName::new("focus_line").unwrap()]
            else {
                panic!()
            };
            let expected = (51..=if case == "one_line" { 51 } else { 60 })
                .map(|line| format!("L{line}"))
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                criteria
                    .keys()
                    .map(|label| label.as_str().to_owned())
                    .collect::<std::collections::BTreeSet<_>>(),
                expected
            );
        }
    }
}
