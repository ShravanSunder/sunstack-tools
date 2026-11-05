#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_decisions::{DecisionRunOutcome, ExecutionReceipt, ExecutionUsage, ModelName};
use sunstack_rules::*;

fn usage_report(costs: &[Option<f64>]) -> RulebookRun {
    RulebookRun {
        results: vec![],
        requests: costs
            .iter()
            .enumerate()
            .map(|(index, cost)| RulebookRequestRecord {
                pair_index: index,
                request: serde_json::from_value(
                    json!({"questions":{"q":{"type":"noul","instructions":"Evaluate"}},"state":{}}),
                )
                .unwrap(),
                outcome: DecisionRunOutcome::Completed {
                    answers: serde_json::from_value(
                        json!({"q":{"type":"noul","yes_probability":1.0}}),
                    )
                    .unwrap(),
                    receipt: ExecutionReceipt {
                        resolved_model: ModelName::new("fixture").unwrap(),
                        usage: ExecutionUsage {
                            input_tokens: 10,
                            output_tokens: 2,
                            cost_usd: *cost,
                        },
                    },
                    attempts: vec![sunstack_decisions::InferenceAttemptRecord {
                        ordinal: 1,
                        evidence: sunstack_decisions::AttemptEvidence::Completed {
                            http_status: 200,
                        },
                    }],
                },
            })
            .collect(),
    }
}
#[test]
fn usage_sums_each_receipt_once_and_distinguishes_missing_cost_from_reported_zero() {
    for (costs, expected, reported, missing) in [
        (vec![], None, 0, 0),
        (vec![None], None, 0, 1),
        (vec![Some(0.0)], Some(0.0), 1, 0),
        (vec![Some(0.01), None, Some(0.02)], Some(0.03), 2, 1),
    ] {
        let summary =
            sunstack_lint::lint_reporting::outcome::summarize_usage(&usage_report(&costs));
        assert_eq!(summary.model_requests, costs.len() as u32);
        assert_eq!(
            (
                summary.cost_reported_requests,
                summary.cost_missing_requests
            ),
            (reported, missing)
        );
        assert_eq!(
            summary.input_tokens,
            (!costs.is_empty()).then_some(costs.len() as u64 * 10)
        );
        assert_eq!(
            summary.output_tokens,
            (!costs.is_empty()).then_some(costs.len() as u64 * 2)
        );
        match (summary.cost_usd, expected) {
            (Some(actual), Some(expected)) => assert!((actual - expected).abs() < 1e-12),
            (None, None) => {}
            _ => panic!("missing cost became a number"),
        }
    }
}
