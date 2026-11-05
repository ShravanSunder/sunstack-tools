#![allow(clippy::unwrap_used)]
use super::super::*;
use sunstack_decisions::{ChoiceLabel, ModelName};
use sunstack_rules::{
    FilterAnswer, FilterAnswerState, FilterIncomplete, FilterName, FilterSkip, FilterSkipReason,
};

#[test]
fn filter_status_preserves_cancelled_incomplete_findings_and_uncertainty_priority() {
    for (cancelled, incomplete, findings, expected) in [
        (false, false, false, LintStatus::Uncertain),
        (false, false, true, LintStatus::Findings),
        (false, true, true, LintStatus::Incomplete),
        (true, true, true, LintStatus::Cancelled),
    ] {
        let mut report = LintReport::empty(
            "config.json".into(),
            ScopeChoice::All,
            EffectiveLimits::default(),
        );
        report.counts.error = u32::from(findings);
        report.filter_answers.push(FilterAnswer {
            filter: FilterName::new("purpose").unwrap(),
            folder: "packages/one".into(),
            input_cut: false,
            requested_model: ModelName::new("jev-latest").unwrap(),
            resolved_model: None,
            state: FilterAnswerState::Unsure {
                answer: ChoiceLabel::new("a").unwrap(),
                probability: 0.3,
                probabilities: std::collections::BTreeMap::from([
                    (ChoiceLabel::new("a").unwrap(), 0.3),
                    (ChoiceLabel::new("b").unwrap(), 0.7),
                ]),
            },
        });
        if incomplete {
            report.filter_answers[0].state = FilterAnswerState::Incomplete {
                reason: FilterIncomplete::ReadsAbsent,
            };
        }
        let run = RulebookRun {
            results: vec![],
            requests: vec![],
            skips: vec![],
        };
        let result = project_outcome(run, &[], &[], report, None, cancelled);
        assert_eq!(result.status, expected);
    }
    let report = LintReport::empty(
        "config.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    let run = RulebookRun {
        results: vec![],
        requests: vec![],
        skips: vec![FilterSkip {
            rule: sunstack_rules::RuleId::new("check").unwrap(),
            file: "outside.md".into(),
            filter: FilterName::new("purpose").unwrap(),
            folder: None,
            why: FilterSkipReason::NoFolder,
            probabilities: None,
        }],
    };
    let report = project_outcome(run, &[], &[], report, None, false);
    assert_eq!(report.status, LintStatus::Incomplete);
    assert!(matches!(
        report.incomplete[0].reason,
        IncompleteReason::NoFilterFolder { .. }
    ));
}
