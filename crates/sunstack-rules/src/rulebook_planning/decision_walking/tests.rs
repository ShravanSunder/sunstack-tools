#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::rulebook_planning::rulebook_test_support as fixture;
use serde_json::json;
use sunstack_evidence::{FieldUnavailable, project_model_view};

fn answers(covers: f64, choice_probability: f64) -> NamedAnswerMap {
    serde_json::from_value(json!({
        "kind":{"type":"choice","selected_label":"outcome","confidence":choice_probability,"probabilities":{"outcome":choice_probability,"action":1.0-choice_probability,"other":0.0}},
        "covers":{"type":"noul","yes_probability":covers},
        "checkable":{"type":"score","score":2.0,"confidence":1.0,"legend":{"0":"Not checkable.","1":"Partly checkable.","2":"Directly checkable."},"probabilities":{"0":0.0,"1":0.0,"2":1.0}}
    })).unwrap()
}

#[test]
fn worked_step_uses_lazy_asking_combines_without_rounding_and_locates_the_completion() {
    let rule = fixture::rule("K001");
    let piece = fixture::piece();
    let index = fixture::index();
    assert!(
        matches!(walk_decision(&rule,&piece,&index,None),WalkOutcome::NeedsAnswers { step } if step.as_str()=="kind")
    );
    let result = walk_decision(&rule, &piece, &index, Some(&answers(0.30, 0.86)));
    assert!(
        matches!(result,WalkOutcome::Settled(RulebookResult::Finding { line:59,field:Some(field),value:Some(value),severity:Severity::Warn,.. }) if field.as_str()=="completion" && (value-0.58).abs()<1e-12)
    );
    let result = walk_decision(
        &rule,
        &piece,
        &index,
        Some(&answers((0.65 - 0.4) / 0.6, 0.86)),
    );
    assert!(
        matches!(result,WalkOutcome::Settled(RulebookResult::Uncertain { step,state,answers,.. }) if step.as_str()=="quality" && answers.len()==3 && serde_json::to_value(&state).unwrap().get("standard").is_some())
    );
    assert!(
        matches!(walk_decision(&rule,&piece,&index,Some(&answers(0.3,0.55))),WalkOutcome::Settled(RulebookResult::Uncertain { step,.. }) if step.as_str()=="kind")
    );
    assert!(matches!(
        walk_decision(&rule, &piece, &FileMatchIndex::default(), None),
        WalkOutcome::Settled(RulebookResult::Finding {
            line: 49,
            answers: None,
            ..
        })
    ));
}

#[test]
fn binary_and_combined_thresholds_have_unique_below_at_above_routes() {
    let rule = fixture::rule("K001");
    let piece = fixture::piece();
    let index = fixture::index();
    for (total, expected) in [
        (0.6 - 1e-9, "finding"),
        (0.6, "finding"),
        (0.6 + 1e-9, "uncertain"),
        (0.8 - 1e-9, "uncertain"),
        (0.8, "pass"),
        (0.8 + 1e-9, "pass"),
    ] {
        let outcome = walk_decision(
            &rule,
            &piece,
            &index,
            Some(&answers((total - 0.4) / 0.6, 0.86)),
        );
        let actual = match outcome {
            WalkOutcome::Settled(RulebookResult::Pass { .. }) => "pass",
            WalkOutcome::Settled(RulebookResult::Finding { .. }) => "finding",
            WalkOutcome::Settled(RulebookResult::Uncertain { .. }) => "uncertain",
            _ => "other",
        };
        assert_eq!(actual, expected, "total={total}");
    }
    let rule = fixture::rule("K003");
    for (value, expected) in [
        (0.6 - 1e-9, "finding"),
        (0.6, "finding"),
        (0.6 + 1e-9, "uncertain"),
        (0.8 - 1e-9, "uncertain"),
        (0.8, "pass"),
        (0.8 + 1e-9, "pass"),
    ] {
        let answers = serde_json::from_value(
            json!({"names_trigger":{"type":"noul","yes_probability":value}}),
        )
        .unwrap();
        let outcome = walk_decision(&rule, &piece, &index, Some(&answers));
        let actual = match outcome {
            WalkOutcome::Settled(RulebookResult::Pass { .. }) => "pass",
            WalkOutcome::Settled(RulebookResult::Finding { .. }) => "finding",
            WalkOutcome::Settled(RulebookResult::Uncertain { .. }) => "uncertain",
            _ => "other",
        };
        assert_eq!(actual, expected, "value={value}");
    }
}

#[test]
fn too_large_only_blocks_an_asking_path_and_unavailable_field_is_contained() {
    let rule = fixture::rule("K001");
    let mut piece = fixture::piece();
    piece.model_view = ModelView::TooLarge {
        chars: 7000,
        limit: 6000,
    };
    assert!(matches!(
        walk_decision(&rule, &piece, &FileMatchIndex::default(), None),
        WalkOutcome::Settled(RulebookResult::Finding { .. })
    ));
    assert!(matches!(
        walk_decision(&rule, &piece, &fixture::index(), None),
        WalkOutcome::Settled(RulebookResult::Incomplete {
            reason: RulebookIncompleteReason::EvidenceTooLarge,
            ..
        })
    ));
    let rule = fixture::rule("K002");
    piece.fields.insert(
        FieldName::new("description").unwrap(),
        FieldValue::Unavailable {
            reason: FieldUnavailable::NotAString,
        },
    );
    assert!(
        matches!(walk_decision(&rule,&piece,&fixture::index(),None),WalkOutcome::Settled(RulebookResult::Incomplete { reason:RulebookIncompleteReason::EvidenceUnavailable,field:Some(field), .. }) if field.as_str()=="description")
    );
}

#[test]
fn spanless_at_keeps_field_name_and_falls_back_to_piece_line() {
    let mut rule = fixture::rule("K002");
    let step = rule.decision.steps.get_mut(&rule.decision.start).unwrap();
    let DecisionStep::Mechanical { yes, no, .. } = step else {
        panic!()
    };
    *yes = no.clone();
    let mut piece = fixture::piece();
    let field = FieldName::new("description").unwrap();
    let mut present_span = piece.location.span.clone();
    present_span.start_line = 3;
    for (value, line) in [
        (
            FieldValue::Text {
                value: "".into(),
                spans: vec![present_span],
            },
            3,
        ),
        (FieldValue::Absent, 49),
        (
            FieldValue::Texts {
                values: vec![],
                spans: vec![],
            },
            49,
        ),
    ] {
        piece.fields.insert(field.clone(), value);
        assert!(
            matches!(walk_decision(&rule,&piece,&fixture::index(),None),WalkOutcome::Settled(RulebookResult::Finding { line:actual,field:Some(field),.. }) if actual==line && field.as_str()=="description")
        );
    }
}

#[test]
fn mechanical_regex_and_contains_read_uncut_middle_and_length_reads_whole_field() {
    let mut piece = fixture::piece();
    piece.fields.insert(
        FieldName::new("body").unwrap(),
        FieldValue::Text {
            value: format!("{}MIDDLE_TOKEN{}", "a".repeat(3500), "z".repeat(3500)),
            spans: vec![],
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    let view = serde_json::to_string(&piece.model_view).unwrap();
    assert!(!view.contains("MIDDLE_TOKEN"));
    for tool in [
        DecisionMechanicalTool::Regex {
            field: FieldName::new("body").unwrap(),
            pattern: "MIDDLE_TOKEN".into(),
        },
        DecisionMechanicalTool::Contains {
            field: FieldName::new("body").unwrap(),
            text: "MIDDLE_TOKEN".into(),
        },
    ] {
        assert_eq!(mechanical_check(&tool, &piece, &fixture::index()), Ok(true));
    }
    assert_eq!(
        mechanical_check(
            &DecisionMechanicalTool::MaxChars {
                field: FieldName::new("body").unwrap(),
                max: 6000
            },
            &piece,
            &fixture::index()
        ),
        Ok(false)
    );
}

#[test]
fn ask_binary_accepts_an_optional_uncertain_route() {
    let wire: crate::AskBinaryWire = serde_json::from_value(json!({"ask":"gate","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable","uncertain":"verdict"})).unwrap();
    assert_eq!(serde_json::to_value(wire).unwrap()["uncertain"], "verdict");
}

fn routed_rule(gate: serde_json::Value, verdict: serde_json::Value) -> RulebookRule {
    let mut entry = fixture::entry_json("K001");
    entry["questions"] = json!({"gate":{"type":"noul","instructions":{"check":"body","task":"Is it eligible?"}},"verdict":{"type":"noul","instructions":{"check":"body","task":"Does it comply?"}}});
    entry["decision"] = json!({"start":"gate","steps":{"gate":gate,"verdict":verdict}});
    let wire: crate::RuleEntryWire = serde_json::from_value(entry).unwrap();
    let mut rule = fixture::rule("K001");
    rule.questions = wire.questions.clone();
    rule.decision = crate::admit_rule_entry(&wire, &fixture::kinds(), &fixture::project()).unwrap();
    rule
}
fn verdict_step() -> serde_json::Value {
    json!({"ask":"verdict","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable"})
}
fn gate_step() -> serde_json::Value {
    json!({"ask":"gate","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable","uncertain":"verdict"})
}

#[test]
fn in_band_binary_and_combine_follow_the_uncertain_edge_and_report_the_final_step() {
    for gate in [
        gate_step(),
        json!({"combine":{"gate":1.0},"yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable","uncertain":"verdict"}),
    ] {
        let rule = routed_rule(gate, verdict_step());
        for (probability, expected) in [(0.9, "pass"), (0.5, "uncertain"), (0.1, "not_applicable")]
        {
            let answers=serde_json::from_value(json!({"gate":{"type":"noul","yes_probability":0.5},"verdict":{"type":"noul","yes_probability":probability}})).unwrap();
            let WalkOutcome::Settled(result) =
                walk_decision(&rule, &fixture::piece(), &fixture::index(), Some(&answers))
            else {
                panic!("unexpected request")
            };
            let result = serde_json::to_value(result).unwrap();
            assert_eq!(result["result"], expected);
            assert_eq!(result["path_taken"], json!(["gate", "verdict"]));
            if expected == "uncertain" {
                assert_eq!(result["step"], "verdict");
            }
        }
    }
}

#[test]
fn uncertain_choice_and_mechanical_terminal_keep_the_ending_step() {
    let mut entry = fixture::entry_json("K001");
    entry["decision"]["steps"]["kind"]["uncertain"] = json!("quality");
    let wire: crate::RuleEntryWire = serde_json::from_value(entry).unwrap();
    let mut rule = fixture::rule("K001");
    rule.decision = crate::admit_rule_entry(&wire, &fixture::kinds(), &fixture::project()).unwrap();
    let result = walk_decision(
        &rule,
        &fixture::piece(),
        &fixture::index(),
        Some(&answers((0.65 - 0.4) / 0.6, 0.55)),
    );
    assert!(
        matches!(result,WalkOutcome::Settled(RulebookResult::Uncertain{step,path_taken,..}) if step.as_str()=="quality" && path_taken.iter().map(StepName::as_str).collect::<Vec<_>>()==vec!["has_completion","kind","quality"])
    );
    let mechanical = json!({"mechanical_check":{"tool":"contains","field":"body","text":"Choose"},"yes":"uncertain","no":"not_applicable"});
    let mut rule = routed_rule(gate_step(), mechanical);
    let answers=serde_json::from_value(json!({"gate":{"type":"noul","yes_probability":0.5},"verdict":{"type":"noul","yes_probability":0.9}})).unwrap();
    let result = walk_decision(&rule, &fixture::piece(), &fixture::index(), Some(&answers));
    assert!(
        matches!(result,WalkOutcome::Settled(RulebookResult::Uncertain{step,path_taken,..}) if step.as_str()=="verdict" && path_taken.len()==2)
    );
    rule.decision.start = StepName::new("verdict").unwrap();
    let result = walk_decision(&rule, &fixture::piece(), &fixture::index(), None);
    assert!(
        matches!(result,WalkOutcome::Settled(RulebookResult::Uncertain{step,path_taken,..}) if step.as_str()=="verdict" && path_taken.len()==1)
    );
}

#[test]
fn uncertain_targets_use_existing_undefined_and_cycle_admission() {
    for target in ["missing", "names_trigger"] {
        let mut entry = fixture::entry_json("K003");
        entry["decision"]["steps"]["names_trigger"]["uncertain"] = json!(target);
        let wire: crate::RuleEntryWire = serde_json::from_value(entry).unwrap();
        let refusals =
            crate::admit_rule_entry(&wire, &fixture::kinds(), &fixture::project()).unwrap_err();
        assert!(
            refusals
                .iter()
                .any(|r| r.message.contains(if target == "missing" {
                    "missing"
                } else {
                    "cycle"
                })),
            "{refusals:?}"
        );
    }
}
