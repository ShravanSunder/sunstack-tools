#![allow(clippy::unwrap_used, clippy::panic)]
use crate::rulebook_planning::rulebook_test_support::{
    entry_json, index, kinds, piece, project, rule,
};
use crate::*;
use serde_json::json;
use sunstack_evidence::{
    FieldName, FieldSource, FieldUnavailable, FieldValue, LoadTarget, ModelView, project_model_view,
};

#[derive(Clone, Copy, Debug)]
enum ReadShape {
    Full,
    BodyCut,
    OpeningCut,
    Unresolved,
    Unavailable,
}
fn policy_rule() -> RulebookRule {
    let mut encoded = entry_json("K001");
    encoded["severity"] = json!("warn");
    encoded["questions"] = json!({
        "q1":{"type":"noul","instructions":{"check":"body","context":["targets"],"task":"Does this contain product material?"}},
        "q2":{"type":"noul","instructions":{"check":"body","context":["targets"],"task":"Is all product material labelled as examples?"}},
        "q3":{"type":"noul","instructions":{"check":"body","context":["targets"],"task":"Does it quote a real product mistake?"}}
    });
    let nudge =
        json!({"finding":"Read this section and its loaded file, then decide where it belongs."});
    encoded["decision"] = json!({"start":"q1","steps":{
        "q1":{"ask":"q1","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"q2","no":"body_complete","uncertain":"uncertain"},
        "q2":{"ask":"q2","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"q3","no":nudge,"uncertain":"q3_trigger"},
        "q3":{"ask":"q3","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":nudge,"no":"body_complete","uncertain":"uncertain"},
        "q3_trigger":{"ask":"q3","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":nudge,"no":"uncertain","uncertain":"uncertain"},
        "body_complete":{"mechanical_check":{"tool":"complete","field":"body"},"yes":"targets_complete","no":"uncertain"},
        "targets_complete":{"mechanical_check":{"tool":"complete","field":"targets"},"yes":"pass","no":"uncertain"}
    }});
    let entry: RuleEntryWire = serde_json::from_value(encoded).unwrap();
    let mut kinds = kinds();
    kinds.get_mut(&entry.evidence).unwrap().fields.insert(
        FieldName::new("targets").unwrap(),
        serde_json::from_value::<FieldSource>(
            json!({"from":"target","regex":"`([^`]+)`","resolve":"path_or_skill"}),
        )
        .unwrap(),
    );
    let graph = admit_rule_entry(&entry, &kinds, &project(), &Default::default()).unwrap();
    let mut rule = rule("K001");
    rule.questions = entry.questions;
    rule.decision = graph;
    rule.severity = entry.severity;
    rule
}
fn policy_piece(shape: ReadShape) -> sunstack_evidence::EvidencePiece {
    let mut piece = piece();
    let mut target = LoadTarget {
        name: "practice".into(),
        path: "skills/practice/SKILL.md".into(),
        exists: true,
        anchor: None,
        candidates: vec![],
        opening: Some("Complete loaded source.".into()),
        fields: None,
        opening_cut: false,
        opening_last_line: Some(1),
    };
    match shape {
        ReadShape::OpeningCut => target.opening_cut = true,
        ReadShape::Unresolved => {
            target.exists = false;
            target.opening = None;
        }
        ReadShape::Unavailable => {
            piece.fields.insert(
                FieldName::new("body").unwrap(),
                FieldValue::Unavailable {
                    reason: FieldUnavailable::Unreadable,
                },
            );
        }
        _ => {}
    }
    piece.fields.insert(
        FieldName::new("targets").unwrap(),
        FieldValue::Targets {
            values: vec![target],
        },
    );
    piece.model_view = project_model_view(&piece.fields, 6000);
    if matches!(shape, ReadShape::BodyCut)
        && let ModelView::Bounded { cut, .. } = &mut piece.model_view
    {
        cut.push(FieldName::new("body").unwrap());
    }
    piece
}

#[test]
fn three_question_policy_keeps_known_triggers_and_guards_both_pass_routes() {
    // Fixed expected routes are the Program Design product-workflow tree, not derived from the walk.
    let rule = policy_rule();
    for (q1, q2, q3, shape, expected, path, read) in [
        (
            0.5,
            0.9,
            0.9,
            ReadShape::Full,
            "uncertain",
            vec!["q1"],
            vec!["q1"],
        ),
        (
            0.3,
            0.9,
            0.9,
            ReadShape::Full,
            "pass",
            vec!["q1", "body_complete", "targets_complete"],
            vec!["q1"],
        ),
        (
            0.1,
            0.9,
            0.9,
            ReadShape::BodyCut,
            "uncertain",
            vec!["q1", "body_complete"],
            vec!["q1"],
        ),
        (
            0.1,
            0.9,
            0.9,
            ReadShape::OpeningCut,
            "uncertain",
            vec!["q1", "body_complete", "targets_complete"],
            vec!["q1"],
        ),
        (
            0.1,
            0.9,
            0.9,
            ReadShape::Unresolved,
            "uncertain",
            vec!["q1", "body_complete", "targets_complete"],
            vec!["q1"],
        ),
        (
            0.7,
            0.3,
            0.1,
            ReadShape::Full,
            "finding",
            vec!["q1", "q2"],
            vec!["q1", "q2"],
        ),
        (
            0.9,
            0.9,
            0.9,
            ReadShape::Full,
            "finding",
            vec!["q1", "q2", "q3"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.5,
            0.9,
            ReadShape::Full,
            "finding",
            vec!["q1", "q2", "q3_trigger"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.5,
            0.1,
            ReadShape::Full,
            "uncertain",
            vec!["q1", "q2", "q3_trigger"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.9,
            0.1,
            ReadShape::Full,
            "pass",
            vec!["q1", "q2", "q3", "body_complete", "targets_complete"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.9,
            0.1,
            ReadShape::BodyCut,
            "uncertain",
            vec!["q1", "q2", "q3", "body_complete"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.9,
            0.1,
            ReadShape::OpeningCut,
            "uncertain",
            vec!["q1", "q2", "q3", "body_complete", "targets_complete"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.9,
            0.1,
            ReadShape::Unresolved,
            "uncertain",
            vec!["q1", "q2", "q3", "body_complete", "targets_complete"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.1,
            0.9,
            0.9,
            ReadShape::Unavailable,
            "incomplete",
            vec!["q1"],
            vec![],
        ),
        (
            0.9,
            0.9,
            0.5,
            ReadShape::Full,
            "uncertain",
            vec!["q1", "q2", "q3"],
            vec!["q1", "q2", "q3"],
        ),
        (
            0.9,
            0.5,
            0.5,
            ReadShape::Full,
            "uncertain",
            vec!["q1", "q2", "q3_trigger"],
            vec!["q1", "q2", "q3"],
        ),
    ] {
        let answers = serde_json::from_value(json!({"q1":{"type":"noul","yes_probability":q1},"q2":{"type":"noul","yes_probability":q2},"q3":{"type":"noul","yes_probability":q3}})).unwrap();
        let WalkOutcome::Settled(result) =
            walk_decision(&rule, &policy_piece(shape), &index(), Some(&answers))
        else {
            panic!("scripted answers must settle")
        };
        let serialized = serde_json::to_value(&result).unwrap();
        assert_eq!(serialized["result"], expected, "{q1}/{q2}/{q3} {shape:?}");
        assert_eq!(serialized["path_taken"], json!(path), "{shape:?}");
        assert_eq!(
            result
                .answers_read()
                .keys()
                .map(|name| name.as_str())
                .collect::<Vec<_>>(),
            read
        );
    }
    let state =
        serde_json::to_value(piece_state(&rule, &policy_piece(ReadShape::Unavailable)).unwrap())
            .unwrap();
    assert_eq!(state["unavailable-fields"], json!({"body":"unreadable"}));
    assert!(state.get("body").is_none());
}
