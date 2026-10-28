#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use support::*;

#[test]
fn native_operands_do_not_replace_the_logical_verdict_when_scoring() {
    // Arrange: confident operands are preserved alongside a clear aggregate failure.
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::No, Split::HeldOut)]);
    let mut candidate = entry(
        1,
        json!({"kind":"dropped","at":{"kind":"verdict"},"answer":"verdict"}),
        None,
        Some(23),
    );
    candidate.answers = serde_json::from_value(json!([
        {"answer_id":"dimension_1","question":{"kind":"informational","name":"quality"},
         "page":null,"answer":{"type":"score","score":0.0,"confidence":0.9,"probabilities":{"0":1.0,"1":0.0},"legend":{"0":"low","1":"high"}},"settlement":"passed"},
        {"answer_id":"dimension_2","question":{"kind":"informational","name":"yes"},
         "page":null,"answer":{"type":"noul","yes_probability":0.1},"settlement":"passed"},
        {"answer_id":"verdict","question":{"kind":"verdict"},
         "page":null,"answer":null,"settlement":"did_not_pass"}
    ])).unwrap();
    // Act: scoring sees the logical verdict, without interpreting an operand as finding truth.
    let report = score(&[bundle(&set, vec![candidate], Some(0.1), 100)], &set).unwrap();
    // Assert: both the verdict step and final truth are the clear negative.
    assert_eq!(
        report.steps[0].correct,
        vec![CaseId::new("case_1").unwrap()]
    );
    assert!(report.steps[0].unknown.is_empty());
    assert_eq!(
        report.end_result.correct_negatives,
        vec![CaseId::new("case_1").unwrap()]
    );
}
