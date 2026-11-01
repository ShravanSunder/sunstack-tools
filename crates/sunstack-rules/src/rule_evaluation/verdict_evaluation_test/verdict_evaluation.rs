#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentAnswer, QuestionName};
fn reference(name: &str) -> JudgmentReference {
    JudgmentReference {
        request_index: 7,
        question_name: QuestionName::new(name).unwrap(),
    }
}
fn rule() -> RuleDefinition {
    decode_rule_definition(
        include_str!(
            "../../../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl"
        )
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
fn error_band_table_preserves_inclusive_threshold_and_native_basis() {
    let mut rule = rule();
    if let VerdictDeclaration::Noul { threshold, .. } = &mut rule.verdict {
        *threshold = 0.8
    }
    for (value, expected) in [
        (0.30, "error"),
        (0.31, "warn_above_band"),
        (0.79, "warn_above_band"),
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
                "error",
                VerdictResult::Fail {
                    activation: Activation::Error,
                },
            )
            | (
                "warn_above_band",
                VerdictResult::Fail {
                    activation: Activation::WarnAboveBand { band: 0.3 },
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
        include_str!(
            "../../../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl"
        )
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
#[test]
fn composite_threshold_equality_and_default_warn_use_native_values_without_fabricated_confidence() {
    let mut rule = rule();
    rule.questions = serde_json::from_value(serde_json::json!({"yes":{"type":"noul","instructions":"yes"},"quality":{"type":"score","instructions":"quality","criteria":["low","middle","high"]}})).unwrap();
    rule.verdict = serde_json::from_value(serde_json::json!({"kind":"composite","dimensions":[{"question":"yes","weight":0.5},{"question":"quality","weight":0.5}],"threshold":0.75})).unwrap();
    rule.severity = Severity::Warn;
    for (probability, expected, passes) in [(0.5, 0.75, true), (0.4998, 0.7499, false)] {
        let informational = BTreeMap::from([
            (
                QuestionName::new("yes").unwrap(),
                ReferencedAnswer {
                    answer: JudgmentAnswer::Noul {
                        yes_probability: probability,
                    },
                    reference: reference("q000001"),
                },
            ),
            (
                QuestionName::new("quality").unwrap(),
                ReferencedAnswer {
                    answer: JudgmentAnswer::Score {
                        score: 2.0,
                        confidence: 0.9,
                        legend: BTreeMap::new(),
                        probabilities: BTreeMap::new(),
                    },
                    reference: reference("q000002"),
                },
            ),
        ]);
        let verdict = evaluate_verdict(
            &rule,
            &VerdictAnswers {
                noul_verdict: None,
                informational,
            },
        )
        .unwrap();
        assert!((verdict.value - expected).abs() < 1e-12);
        assert_eq!(matches!(verdict.result, VerdictResult::Pass), passes);
        if !passes {
            assert!(matches!(
                verdict.result,
                VerdictResult::Fail {
                    activation: Activation::Warn
                }
            ));
        }
    }
    let raw = serde_json::json!({"rule_id":"default","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"judge"},"threshold":0.8}});
    let default = decode_rule_definition(
        &serde_json::to_vec(&raw).unwrap(),
        "rules.jsonl".as_ref(),
        1,
    )
    .unwrap();
    assert_eq!(default.severity, Severity::Warn);
}
