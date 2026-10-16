#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentAnswer, QuestionName};
use sunstack_rules::*;
fn reference(name: &str) -> JudgmentReference {
    JudgmentReference {
        request_index: 7,
        question_name: QuestionName::new(name).unwrap(),
    }
}
fn rule() -> RuleDefinition {
    decode_rule_definition(
        include_str!("../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl")
            .lines()
            .nth(2)
            .unwrap()
            .as_bytes(),
        "rules.jsonl".as_ref(),
        3,
    )
    .unwrap()
}
#[test]
fn blocking_band_table_preserves_inclusive_threshold_and_native_basis() {
    let mut rule = rule();
    if let VerdictDeclaration::Noul { threshold, .. } = &mut rule.verdict {
        *threshold = 0.8
    }
    for (value, expected) in [
        (0.30, "block"),
        (0.31, "nudge_above_band"),
        (0.79, "nudge_above_band"),
        (0.80, "pass"),
    ] {
        let input = VerdictAnswers {
            noul_verdict: Some(ReferencedAnswer {
                answer: JudgmentAnswer::Noul {
                    yes_probability: value,
                },
                reference: reference("q000001"),
            }),
            informational: BTreeMap::new(),
        };
        let result = evaluate_verdict(&rule, &input).unwrap();
        assert_eq!(result.value, value);
        match (expected, &result.result) {
            (
                "block",
                VerdictResult::Fail {
                    activation: Activation::Block,
                },
            )
            | (
                "nudge_above_band",
                VerdictResult::Fail {
                    activation: Activation::NudgeAboveBand { band: 0.3 },
                },
            )
            | ("pass", VerdictResult::Pass) => {}
            _ => panic!("{expected}: {result:?}"),
        }
        assert!(matches!(result.basis,VerdictBasis::Noul{answer} if answer==reference("q000001")));
    }
}
#[test]
fn composite_normalizes_fractional_scores_and_keeps_each_native_reference() {
    let rule = decode_rule_definition(
        include_str!("../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl")
            .lines()
            .nth(1)
            .unwrap()
            .as_bytes(),
        "rules.jsonl".as_ref(),
        2,
    )
    .unwrap();
    let input = VerdictAnswers {
        noul_verdict: None,
        informational: BTreeMap::from([
            (
                QuestionName::new("specificity").unwrap(),
                ReferencedAnswer {
                    answer: JudgmentAnswer::Score {
                        score: 1.5,
                        confidence: 0.4,
                        legend: BTreeMap::new(),
                        probabilities: BTreeMap::new(),
                    },
                    reference: reference("q000002"),
                },
            ),
            (
                QuestionName::new("names_file").unwrap(),
                ReferencedAnswer {
                    answer: JudgmentAnswer::Noul {
                        yes_probability: 0.8,
                    },
                    reference: reference("q000003"),
                },
            ),
        ]),
    };
    let result = evaluate_verdict(&rule, &input).unwrap();
    assert!((result.value - 0.575).abs() < 1e-12);
    let VerdictBasis::Composite { dimensions } = result.basis else {
        panic!("wrong basis")
    };
    assert_eq!(dimensions[0].normalized_value, 0.5);
    assert_eq!(dimensions[0].answer, reference("q000002"));
    assert_eq!(dimensions[1].normalized_value, 0.8);
    assert_eq!(dimensions[1].answer, reference("q000003"));
    assert!(
        !serde_json::to_string(&dimensions[1])
            .unwrap()
            .contains("confidence")
    );
    assert!(
        evaluate_verdict(
            &rule,
            &VerdictAnswers {
                noul_verdict: None,
                informational: BTreeMap::new()
            }
        )
        .is_err()
    );
}
