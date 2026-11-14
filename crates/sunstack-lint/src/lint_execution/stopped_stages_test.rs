#![allow(clippy::unwrap_used)]
use super::*;
use crate::{EffectiveLimits, ScopeChoice};

/// Each stage's CLI report is the incomplete report the CLI gave before stops were typed,
/// with the stage's original message.
#[test]
fn each_stopped_stage_keeps_its_original_cli_report() {
    let report = LintReport::empty(
        "sunstack.lint.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    for (stage, original_message) in [
        (RunStage::Admission, "rulebook loading failed"),
        (RunStage::TargetSelection, "target selection failed"),
        (RunStage::EvidenceBuilding, "evidence building failed"),
        (RunStage::SelectionScope, "selection scope failed"),
        (RunStage::EvidenceStage, "evidence stage failed"),
        (
            RunStage::ContextEvidence,
            "context evidence building failed",
        ),
    ] {
        let cli = RunStageStopped::before_dispatch(stage, report.clone()).into_cli_report();
        let original = super::super::lint_run::failed(report.clone(), original_message);
        assert_eq!(
            serde_json::to_value(&cli).unwrap(),
            serde_json::to_value(&original).unwrap(),
            "{stage:?}"
        );
    }
}
