#![allow(clippy::unwrap_used, clippy::panic)]
use crate::test_support::*;
use crate::*;
use serde::Serialize;

async fn check_public_route(admitted_route: bool) {
    for count in [255, 256] {
        // Arrange a valid response even for the forbidden request, so HTTP cannot hide refusal.
        let mut input = mixed_input("unused");
        let question = input
            .request
            .questions
            .get_mut(&QuestionName::new("native_choice").unwrap())
            .unwrap();
        let JudgmentQuestion::Choice { criteria, .. } = question else {
            panic!()
        };
        for index in 2..count {
            criteria.insert(
                ChoiceLabel::new(format!("option{index}")).unwrap(),
                StructuredEntry::Null,
            );
        }
        let probabilities = criteria
            .keys()
            .map(|label| {
                (
                    label.as_str().to_owned(),
                    if label.as_str() == "alpha" { 1.0 } else { 0.0 },
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut reply = response();
        reply["answers"]["native_choice"]["probabilities"] =
            serde_json::to_value(probabilities).unwrap();
        let mut server = TestServer::start(vec![HttpReply::json(reply)]).await;
        input.backend = selection(&server.endpoint);
        let backend = backend(&server.endpoint);
        // Public admitted callers can supply previously budget-admitted bytes. Recreate the
        // pre-cap body independently, preserving the backend identity and matching encoding.
        #[derive(Serialize)]
        struct Body<'a> {
            model: &'a ModelName,
            state: &'a SharedEvidenceState,
            questions: &'a NamedQuestionMap,
        }
        let BackendSelection::JevOpenrouter { model, .. } = &input.backend;
        let encoded = EncodedBackendRequest::new(
            backend.identity().clone(),
            serde_json::to_vec(&Body {
                model,
                state: &input.request.state,
                questions: &input.request.questions,
            })
            .unwrap(),
            input.request.questions.len(),
        );
        let accounting = backend.account_request(&encoded);
        let RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } =
            assess_request_budget(encoded, &input.request_budget, accounting)
        else {
            panic!("fixture budget")
        };
        // Act through each distinct public entry.
        let outcome = if admitted_route {
            run_admitted_judgments(
                &input.request,
                &backend,
                &admitted_request,
                &input.attempts,
                &CancellationToken::new(),
            )
            .await
        } else {
            run_judgments(&input, &backend, &CancellationToken::new()).await
        };
        let capture = server.captures.try_recv().ok();
        let calls = server.finish().await;
        // Assert exact wire cardinality for the allowed case, local actionable refusal otherwise.
        if count == 255 {
            assert!(
                matches!(outcome, DecisionRunOutcome::Completed { .. }),
                "{outcome:?}"
            );
            assert_eq!(calls, 1);
            let body: serde_json::Value = serde_json::from_slice(&capture.unwrap().body).unwrap();
            assert_eq!(
                body["questions"]["native_choice"]["criteria"]
                    .as_object()
                    .unwrap()
                    .len(),
                255
            );
        } else {
            assert_eq!(calls, 0, "oversized public request reached HTTP");
            let DecisionRunOutcome::InvalidRequest { diagnostics } = outcome else {
                panic!("{outcome:?}")
            };
            assert_eq!(
                diagnostics.items[0].code,
                DecisionDiagnosticCode::InvalidChoice
            );
            assert_eq!(
                diagnostics.items[0].origin,
                DecisionDiagnosticOrigin::Request
            );
            assert_eq!(
                diagnostics.items[0].retry_advice,
                RetryAdvice::RepairRequest
            );
            assert!(diagnostics.items[0].guidance.contains("256"));
            assert!(diagnostics.items[0].guidance.contains("255"));
        }
    }
}
#[tokio::test]
async fn direct_public_choice_255_completes_and_256_refuses_before_http() {
    check_public_route(false).await;
}
#[tokio::test]
async fn admitted_public_choice_255_completes_and_256_refuses_before_http() {
    check_public_route(true).await;
}
