use super::*;

fn located_rule() -> RulebookRule {
    let mut entry = fixture::entry_json("K001");
    entry["questions"] =
        json!({"q":{"type":"noul","instructions":{"task":"Check body","check":"body"}}});
    entry["decision"] = json!({"start":"q","steps":{"q":{"ask":"q","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":{"finding":"Read this line.","at":"body","locate":{"name":"focus","task":"Choose line","min_probability":0.7}},"no":"pass"}}});
    let wire: RuleEntryWire = serde_json::from_value(entry).unwrap();
    let mut rule = fixture::rule("K001");
    rule.questions = wire.questions.clone();
    rule.decision = admit_rule_entry(
        &wire,
        &fixture::kinds(),
        &fixture::project(),
        &Default::default(),
    )
    .unwrap();
    rule
}
fn line_piece(count: usize) -> EvidencePiece {
    let mut piece = fixture::piece();
    piece.text = (1..=count).map(|line| format!("line {line}\n")).collect();
    piece.location.first_line = 10;
    piece.location.last_line = 9 + count;
    piece.location.span.byte_start = 100;
    piece.location.span.byte_end = 100 + piece.text.len();
    piece.location.span.start_line = 10;
    piece.location.span.end_line = 10 + count;
    piece.location.span.end_column = 1;
    let mut enclosing = piece.location.span.clone();
    enclosing.byte_start = 0;
    enclosing.byte_end += 100;
    enclosing.start_line = 1;
    enclosing.end_line += 100;
    piece.fields = BTreeMap::from([(
        FieldName::new("body").unwrap(),
        FieldValue::Text {
            value: piece.text.clone(),
            spans: vec![piece.location.span.clone(), enclosing],
        },
    )]);
    piece.model_view = project_model_view(&piece.fields, 100000);
    piece
}
async fn observed(
    rule: RulebookRule,
    pieces: Vec<EvidencePiece>,
    limit: usize,
    probability: f64,
) -> (RulebookRun, Vec<serde_json::Value>) {
    let mut server = controlled::ControlledServer::start().await;
    let mut settings = settings(&server.endpoint);
    settings.max_locate_lines = limit;
    let backend =
        Arc::new(JevOpenRouterBackend::new(settings.backend.clone(), "SYNTHETIC").unwrap());
    let mut task = tokio::spawn(async move {
        run_rulebook(
            &[rule],
            &pieces,
            &BTreeMap::new(),
            settings,
            Some(backend),
            CancellationToken::new(),
        )
        .await
    });
    let mut bodies = Vec::new();
    let run = loop {
        tokio::select! {biased;result=&mut task=>break result.unwrap(),pending=server.receive()=>{
            let answers=pending.body["questions"].as_object().unwrap().iter().map(|(name,question)|{
                let answer=if question["type"]=="choice" {let criteria=question["criteria"].as_object().unwrap();let selected=criteria.keys().next_back().unwrap();let probabilities=criteria.keys().map(|key|(key.clone(),json!(if key==selected {0.9}else {0.1/(criteria.len()-1).max(1) as f64}))).collect::<serde_json::Map<_,_>>();json!({"type":"choice","choice":selected,"confidence":0.9,"probabilities":probabilities})}else{json!({"type":"noul","noul":probability})};(name.clone(),answer)
            }).collect::<serde_json::Map<_,_>>();bodies.push(pending.body.clone());pending.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":2,"output_tokens":1}}));
        }}
    };
    assert_eq!(server.finish().await, bodies.len());
    (run, bodies)
}

#[tokio::test]
async fn locator_choice_255_reaches_http_but_256_is_a_local_refusal() {
    let (valid, bodies) = observed(located_rule(), vec![line_piece(255)], 1000, 0.9).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        bodies[0]["questions"]["focus"]["criteria"]
            .as_object()
            .unwrap()
            .len(),
        255
    );
    assert!(matches!(
        valid.results[0].result,
        RulebookResult::Finding {
            anchor: Anchor::Resolved,
            ..
        }
    ));
    let (invalid, bodies) = observed(located_rule(), vec![line_piece(256)], 1000, 0.9).await;
    assert!(bodies.is_empty(), "oversized locator reached HTTP");
    let value = serde_json::to_value(invalid).unwrap();
    assert_eq!(value["results"][0]["result"], "incomplete");
    assert_eq!(
        value["results"][0]["reason"]["choice_options_exceeded"]["options"],
        256
    );
}

#[tokio::test]
async fn omitted_locator_is_attached_only_to_the_reached_oversized_finding() {
    let mut branched = located_rule();
    let DecisionStep::AskNoul { routes, .. } = branched.decision.steps.values_mut().next().unwrap()
    else {
        panic!()
    };
    let DecisionRoute::Finding(finding) = &routes.yes else {
        panic!()
    };
    let mut unused = finding.clone();
    unused.locate.as_mut().unwrap().name = QuestionName::new("unreached_focus").unwrap();
    routes.no = DecisionRoute::Finding(unused);
    let (run, bodies) = observed(branched, vec![line_piece(3), line_piece(4)], 3, 0.9).await;
    assert_eq!(bodies.len(), 2);
    let value = serde_json::to_value(run).unwrap();
    assert!(value["results"][0].get("locator_omitted").is_none());
    assert_eq!(value["results"][0]["anchor"], "resolved");
    assert_eq!(value["results"][1]["anchor"], "default");
    assert_eq!(value["results"][1]["line"], 10);
    assert_eq!(
        value["results"][1]["locator_omitted"],
        json!({"locator":"focus","field":"body","reason":"line_limit","candidate_lines":4,"limit":3})
    );
    let (pass, bodies) = observed(located_rule(), vec![line_piece(4)], 3, 0.1).await;
    assert_eq!(bodies.len(), 1);
    assert!(matches!(
        pass.results[0].result,
        RulebookResult::Pass { .. }
    ));
    assert!(
        serde_json::to_value(pass).unwrap()["results"][0]
            .get("locator_omitted")
            .is_none()
    );
}

#[tokio::test]
async fn final_authored_choice_cardinality_is_checked_before_dispatch() {
    for count in [255, 256] {
        let mut rule = located_rule();
        rule.questions.insert(
            QuestionName::new("authored_choice").unwrap(),
            JudgmentQuestion::Choice {
                instructions: StructuredEntry::Text("Choose".into()),
                criteria: (0..count)
                    .map(|index| {
                        (
                            ChoiceLabel::new(format!("option{index}")).unwrap(),
                            StructuredEntry::Text("Option".into()),
                        )
                    })
                    .collect(),
            },
        );
        let (run, bodies) = observed(rule, vec![line_piece(1)], 80, 0.9).await;
        if count == 255 {
            assert_eq!(bodies.len(), 1);
            assert!(matches!(
                run.results[0].result,
                RulebookResult::Finding { .. }
            ));
        } else {
            assert!(bodies.is_empty());
            let value = serde_json::to_value(run).unwrap();
            assert_eq!(
                value["results"][0]["reason"]["choice_options_exceeded"],
                json!({"options":256,"limit":255})
            );
        }
    }
}
