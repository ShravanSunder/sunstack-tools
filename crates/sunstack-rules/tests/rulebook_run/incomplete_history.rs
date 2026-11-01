use super::*;

#[tokio::test]
async fn request_refusal_keeps_the_mechanical_prefix_and_reached_question_without_answers() {
    let mut subject = rule();
    let prefix = StepName::new("local_guard").unwrap();
    subject.decision.steps.insert(
        prefix.clone(),
        DecisionStep::Mechanical {
            tool: DecisionMechanicalTool::Contains {
                field: FieldName::new("body").unwrap(),
                text: "Choose".into(),
            },
            yes: DecisionRoute::Step(StepName::new("q").unwrap()),
            no: DecisionRoute::Pass,
        },
    );
    subject.decision.start = prefix;
    let mut bounded = settings("not-dispatched");
    bounded.request_max_chars = 1;
    let run = run_rulebook(
        &[subject],
        &pieces(),
        &BTreeMap::new(),
        bounded,
        None,
        CancellationToken::new(),
    )
    .await;
    assert!(run.requests.is_empty());
    assert_eq!(run.results.len(), 2);
    for pair in run.results {
        assert_eq!(pair.result.basis(), None);
        let result = serde_json::to_value(pair.result).unwrap();
        assert_eq!(result["reason"], "request_too_large");
        assert_eq!(result["step"], "q");
        assert_eq!(result["path_taken"], json!(["local_guard", "q"]));
        assert_eq!(result["answers_read"], json!({}));
    }
}
