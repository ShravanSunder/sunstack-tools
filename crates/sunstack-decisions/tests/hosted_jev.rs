#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::json;
use sunstack_decisions::*;

/// Credential-gated integration, deliberately outside the default local gate.
/// The runner supplies OPENROUTER_API_KEY in memory; this test never reads files.
#[tokio::test]
#[ignore = "hosted Jev call requires OPENROUTER_API_KEY"]
async fn hosted_mixed_native_primitives_return_validated_model_and_usage() {
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("hosted credential not supplied");
    let selection = BackendSelection::JevOpenrouter {
        endpoint: "https://openrouter.ai/api/v1/systemone".into(),
        model: ModelName::new("jev-latest").unwrap(),
        context_ceiling_tokens: None,
    };
    let backend = JevOpenRouterBackend::new(selection.clone(), &api_key).unwrap();
    let input:RunJudgmentsInput=serde_json::from_value(json!({
        "request":{"state":{"message":"The sky is blue and the temperature is warm."},"questions":{
            "is_blue":{"type":"noul","instructions":{"question":"Does the message say the sky is blue?","field":"message"},"criteria":{"true":"It says blue.","false":"It does not say blue."}},
            "sky_color":{"type":"choice","instructions":"Which sky color does the message name?","criteria":{"blue":{"description":"Blue sky"},"red":null}},
            "warmth":{"type":"score","instructions":"How warm is the temperature in the message?","criteria":[{"description":"Cold"},{"description":"Warm"},{"description":"Hot"}]}
        }},"backend":selection,
        "attempts":{"attempt_timeout_ms":60000,"retry_count":0,"initial_backoff_ms":100,"max_backoff_ms":1000,"max_response_bytes":1048576},
        "request_budget":{"max_input_tokens":10000,"max_request_bytes":50000,"max_questions":10,"max_requests":1,"safety_margin_tokens":256,"accounting_policy":"allow_estimated"}
    })).unwrap();
    let outcome = run_judgments(&input, &backend, &CancellationToken::new()).await;
    match outcome {
        DecisionRunOutcome::Completed {
            answers,
            receipt,
            attempts,
        } => {
            assert_eq!(answers.len(), 3);
            assert!(receipt.usage.input_tokens > 0);
            assert_eq!(attempts.len(), 1);
            assert!(matches!(
                answers.get(&QuestionName::new("is_blue").unwrap()),
                Some(JudgmentAnswer::Noul { .. })
            ));
            assert!(matches!(
                answers.get(&QuestionName::new("sky_color").unwrap()),
                Some(JudgmentAnswer::Choice { .. })
            ));
            assert!(matches!(
                answers.get(&QuestionName::new("warmth").unwrap()),
                Some(JudgmentAnswer::Score { .. })
            ));
            println!(
                "hosted receipt: {}",
                serde_json::to_string(&receipt).unwrap()
            );
        }
        other => panic!(
            "hosted call did not complete: {}",
            serde_json::to_string(&other).unwrap()
        ),
    }
}
