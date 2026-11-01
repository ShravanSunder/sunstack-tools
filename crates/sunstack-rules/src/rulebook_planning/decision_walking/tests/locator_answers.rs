use super::*;

#[test]
fn reached_locators_read_their_answer_while_unreached_and_left_out_locators_keep_the_default() {
    let gate = json!({"ask":"gate","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"verdict","no":"pass"});
    let verdict = json!({"mechanical_check":{"tool":"contains","field":"body","text":"line"},"yes":{"finding":"Read here.","at":"body","locate":{"name":"focus_line","task":"Choose a line.","min_probability":0.7}},"no":"pass"});
    let mut rule = routed_rule(gate, verdict);
    let mut piece = fixture::piece();
    let prefix = "## Subject\n\n";
    let body = (51..=60)
        .map(|line| format!("line{line}\n"))
        .collect::<String>();
    piece.text = format!("{prefix}{body}");
    piece.location.span.byte_start = 100;
    piece.location.span.byte_end = 100 + piece.text.len();
    piece.location.span.end_line = 61;
    piece.location.span.end_column = 1;
    let mut span = piece.location.span.clone();
    span.byte_start += prefix.len();
    span.start_line = 51;
    piece.fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Text {
            value: body,
            spans: vec![span],
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    for (gate, probability, outcome, line, anchor, read_locator) in [
        (0.9, Some(0.8), "finding", 55, Anchor::Resolved, true),
        (0.9, Some(0.7), "finding", 55, Anchor::Resolved, true),
        (0.9, Some(0.6), "finding", 51, Anchor::Default, true),
        (0.9, None, "finding", 51, Anchor::Default, false),
        (0.1, Some(0.8), "pass", 0, Anchor::Default, false),
    ] {
        let mut answers: NamedAnswerMap = serde_json::from_value(json!({"gate":{"type":"noul","yes_probability":gate},"verdict":{"type":"noul","yes_probability":0.1}})).unwrap();
        if let Some(probability) = probability {
            let probabilities = (51..=60)
                .map(|line| {
                    (
                        format!("L{line}"),
                        json!(if line == 55 { probability } else { 0.01 }),
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            answers.insert(sunstack_decisions::QuestionName::new("focus_line").unwrap(), serde_json::from_value(json!({"type":"choice","selected_label":"L55","confidence":0.01,"probabilities":probabilities})).unwrap());
        }
        let WalkOutcome::Settled(result) =
            walk_decision(&rule, &piece, &fixture::index(), Some(&answers))
        else {
            panic!()
        };
        assert_eq!(result.basis(), Some(Basis::Judged));
        assert_eq!(
            result
                .answers_read()
                .contains_key(&sunstack_decisions::QuestionName::new("focus_line").unwrap()),
            read_locator
        );
        assert!(
            !result
                .answers_read()
                .contains_key(&sunstack_decisions::QuestionName::new("verdict").unwrap())
        );
        let value = serde_json::to_value(result).unwrap();
        assert_eq!(value["result"], outcome);
        if outcome == "finding" {
            assert_eq!(value["line"], line);
            assert_eq!(value["anchor"], serde_json::to_value(anchor).unwrap());
        }
    }
    rule.decision.start = StepName::new("verdict").unwrap();
    let WalkOutcome::Settled(result) = walk_decision(&rule, &piece, &fixture::index(), None) else {
        panic!()
    };
    assert_eq!(result.basis(), Some(Basis::Mechanical));
    assert!(result.answers_read().is_empty());
    assert!(matches!(
        result,
        RulebookResult::Finding {
            line: 51,
            anchor: Anchor::Default,
            ..
        }
    ));
}
