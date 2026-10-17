#![allow(clippy::unwrap_used)]
use serde_json::json;
use sunstack_decisions::*;

#[test]
fn structured_roots_and_null_survive_each_primitive_position() {
    for entry in [
        json!(null),
        json!("text"),
        json!([]),
        json!({}),
        json!({"nested":[true, 1, null, {"array":[]} ]}),
    ] {
        let request = json!({"questions":{
            "n":{"type":"noul", "instructions":entry,"criteria":{"true":entry,"false":entry}},
            "c":{"type":"choice","instructions":entry,"criteria":{"label":entry}},
            "s":{"type":"score","instructions":entry,"criteria":[entry]}
        },"state":{"nested":[null, true, 42]}});
        let decoded: StructuredJudgmentRequest = serde_json::from_value(request.clone()).unwrap();
        assert!(judgment_validation::validate_request(&decoded).is_ok());
        assert_eq!(serde_json::to_value(decoded).unwrap(), request);
    }
}

#[test]
fn numeric_and_boolean_entry_roots_are_refused_but_nested_values_are_allowed() {
    for entry in [json!(true), json!(1)] {
        assert!(serde_json::from_value::<StructuredEntry>(entry).is_err());
    }
    for state in [json!(true), json!(1), json!(null)] {
        assert!(serde_json::from_value::<SharedEvidenceState>(state).is_err());
    }
}

#[test]
fn missing_fields_partial_noul_unknown_fields_and_empty_identifiers_are_refused() {
    for question in [
        json!({"type":"noul"}),
        json!({"type":"noul","instructions":null,"criteria":null}),
        json!({"type":"noul","instructions":null,"criteria":{"true":null}}),
        json!({"type":"noul","instructions":null,"criteria":{"false":null}}),
        json!({"type":"noul","instructions":null,"extra":"sentinel"}),
        json!({"type":"choice","instructions":null,"criteria":{"":null}}),
        json!({"type":"other","instructions":null}),
    ] {
        assert!(
            serde_json::from_value::<JudgmentQuestion>(question.clone()).is_err(),
            "accepted {question}"
        );
    }
    assert!(QuestionName::new("").is_err());
    let omitted: JudgmentQuestion =
        serde_json::from_value(json!({"type":"noul","instructions":null})).unwrap();
    assert_eq!(
        serde_json::to_value(omitted).unwrap(),
        json!({"type":"noul","instructions":null})
    );
}

fn mixed_request() -> StructuredJudgmentRequest {
    serde_json::from_value(json!({"questions":{
        "n":{"type":"noul","instructions":null},
        "c":{"type":"choice","instructions":"classify","criteria":{"alpha":null,"beta":null}},
        "s":{"type":"score","instructions":"score","criteria":[null,"middle",{}]}
    },"state":"state"}))
    .unwrap()
}
fn receipt() -> ExecutionReceipt {
    ExecutionReceipt {
        resolved_model: ModelName::new("observed-model").unwrap(),
        usage: ExecutionUsage {
            input_tokens: 17,
            output_tokens: 3,
            cost_usd: None,
        },
    }
}
fn valid_answers() -> serde_json::Value {
    json!({"n":{"type":"noul","yes_probability":0.3},
        "c":{"type":"choice","selected_label":"alpha","confidence":0.4,"probabilities":{"alpha":0.6,"beta":0.3}},
        "s":{"type":"score","score":1.25,"confidence":0.7,"legend":{"0":"changed text","1":null,"2":{}},"probabilities":{"0":0.1,"1":0.5,"2":0.3}}})
}

#[test]
fn response_validation_accepts_fractions_without_sum_or_legend_text_constraints() {
    let answers: NamedAnswerMap = serde_json::from_value(valid_answers()).unwrap();
    let (answers, receipt) =
        judgment_validation::validate_response(&mixed_request(), answers, receipt()).unwrap();
    assert!(
        matches!(answers.get(&QuestionName::new("s").unwrap()), Some(JudgmentAnswer::Score {score, ..}) if *score == 1.25)
    );
    assert_eq!(receipt.usage.cost_usd, None);
}

#[test]
fn request_relative_answer_mismatches_refuse_the_whole_map() {
    let original = valid_answers();
    let mut cases = Vec::new();
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("n");
    cases.push(missing);
    let mut extra = original.clone();
    extra["extra"] = json!({"type":"noul","yes_probability":0.5});
    cases.push(extra);
    let mut kind = original.clone();
    kind["n"] = original["c"].clone();
    cases.push(kind);
    let mut selected = original.clone();
    selected["c"]["selected_label"] = json!("outside");
    cases.push(selected);
    let mut label = original.clone();
    label["c"]["probabilities"] = json!({"alpha":0.8,"outside":0.2});
    cases.push(label);
    for field in ["legend", "probabilities"] {
        let mut missing = original.clone();
        missing["s"][field].as_object_mut().unwrap().remove("1");
        cases.push(missing);
        let mut extra = original.clone();
        extra["s"][field]["3"] = if field == "legend" {
            json!(null)
        } else {
            json!(0.1)
        };
        cases.push(extra);
        let mut key = original.clone();
        let old = key["s"][field]
            .as_object_mut()
            .unwrap()
            .remove("0")
            .unwrap();
        key["s"][field]["00"] = old;
        cases.push(key);
    }
    for (name, field) in [
        ("n", "yes_probability"),
        ("c", "confidence"),
        ("s", "confidence"),
        ("s", "score"),
    ] {
        let mut negative = original.clone();
        negative[name][field] = json!(-0.1);
        cases.push(negative);
        let mut too_large = original.clone();
        too_large[name][field] = json!(3.0);
        cases.push(too_large);
    }
    for value in cases {
        let answers = serde_json::from_value(value).unwrap();
        assert!(
            judgment_validation::validate_response(&mixed_request(), answers, receipt()).is_err()
        );
    }
    let mut answers: NamedAnswerMap = serde_json::from_value(original).unwrap();
    answers.insert(
        QuestionName::new("n").unwrap(),
        JudgmentAnswer::Noul {
            yes_probability: f64::NAN,
        },
    );
    assert!(judgment_validation::validate_response(&mixed_request(), answers, receipt()).is_err());
}

#[test]
fn empty_criteria_refused_one_level_score_admitted() {
    for question in [
        json!({"type":"choice","instructions":null,"criteria":{}}),
        json!({"type":"score","instructions":null,"criteria":[]}),
    ] {
        let request = StructuredJudgmentRequest {
            questions: [(
                QuestionName::new("q").unwrap(),
                serde_json::from_value(question).unwrap(),
            )]
            .into(),
            state: SharedEvidenceState::Text(String::new()),
        };
        assert!(judgment_validation::validate_request(&request).is_err());
    }
    let request: StructuredJudgmentRequest=serde_json::from_value(json!({"questions":{"q":{"type":"score","instructions":null,"criteria":[null]}},"state":[]})).unwrap();
    let answers = serde_json::from_value(json!({"q":{"type":"score","score":0.0,"confidence":0.0,"legend":{"0":null},"probabilities":{"0":0.7}}})).unwrap();
    assert!(judgment_validation::validate_response(&request, answers, receipt()).is_ok());
}

proptest::proptest! {
    #[test]
    fn any_different_choice_label_set_is_refused(labels in proptest::collection::btree_set("[a-z]{1,8}", 1..12), replacement in "[A-Z]{1,8}") {
        let criteria = labels.iter().map(|name| (ChoiceLabel::new(name).unwrap(), StructuredEntry::Null)).collect();
        let request = StructuredJudgmentRequest {questions:[(QuestionName::new("q").unwrap(), JudgmentQuestion::Choice {instructions:StructuredEntry::Null,criteria})].into(),state:SharedEvidenceState::Text("state".into())};
        let mut probabilities:std::collections::BTreeMap<_,_>=labels.iter().map(|name| (ChoiceLabel::new(name).unwrap(), 0.5)).collect();
        let first=probabilities.keys().next().unwrap().clone(); probabilities.remove(&first); probabilities.insert(ChoiceLabel::new(replacement).unwrap(),0.5);
        let answers=[(QuestionName::new("q").unwrap(),JudgmentAnswer::Choice {selected_label:first,confidence:0.1, probabilities})].into();
        proptest::prop_assert!(judgment_validation::validate_response(&request,answers,receipt()).is_err());
    }
}
