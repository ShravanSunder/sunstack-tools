//! Run stages whose blocking task ended without returning its result.
//!
//! A stage that panics, or whose runtime shuts down before it runs, has unknown effects:
//! that is a failure of the native call, not a domain-incomplete run. The CLI renders it
//! exactly as it always has; an in-process caller can tell it apart and keep everything
//! the run produced before the stage stopped.
use crate::{
    LintReport,
    lint_reporting::{outcome::selection_usage, saved_reports::capture_selection_requests},
};

/// A stage of a definition run that does its work on a blocking task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStage {
    Admission,
    TargetSelection,
    EvidenceBuilding,
    SelectionScope,
    EvidenceStage,
    ContextEvidence,
}

impl RunStage {
    /// The CLI's diagnostic message for a stopped stage.
    fn cli_message(self) -> &'static str {
        match self {
            Self::Admission => "rulebook loading failed",
            Self::TargetSelection => "target selection failed",
            Self::EvidenceBuilding => "evidence building failed",
            Self::SelectionScope => "selection scope failed",
            Self::EvidenceStage => "evidence stage failed",
            Self::ContextEvidence => "context evidence building failed",
        }
    }
}

/// A run stage whose blocking task ended without returning its result.
#[derive(Debug, Clone)]
pub struct RunStageStopped {
    stage: RunStage,
    /// The report as it stood when the stage started.
    report: Box<LintReport>,
    /// That report with the model requests the run had already settled, their usage and
    /// the checked files acquired so far; absent before model dispatch exists.
    progress: Option<Box<LintReport>>,
}

impl RunStageStopped {
    /// A stage that stopped before the run could dispatch any model request.
    pub(crate) fn before_dispatch(stage: RunStage, report: LintReport) -> Self {
        Self {
            stage,
            report: Box::new(report),
            progress: None,
        }
    }

    /// A stage that stopped after model dispatch was set up, keeping what it settled.
    pub(crate) fn with_settled_requests(
        stage: RunStage,
        report: LintReport,
        dispatcher: &sunstack_rules::ModelRequestDispatch,
        checked_files_acquired: usize,
    ) -> Self {
        let mut progress = report.clone();
        progress.usage = selection_usage(dispatcher, checked_files_acquired);
        // A request that cannot be serialized for its digest stays counted in usage
        // without a saved entry; the ordinary path refuses the whole run for it instead.
        let _ = capture_selection_requests(&mut progress, dispatcher);
        Self {
            stage,
            report: Box::new(report),
            progress: Some(Box::new(progress)),
        }
    }

    pub fn stage(&self) -> RunStage {
        self.stage
    }

    /// The CLI's report: the report as it stood, marked incomplete for the stage.
    pub fn into_cli_report(self) -> LintReport {
        super::lint_run::failed(*self.report, self.stage.cli_message())
    }

    /// Everything the run produced before the stage stopped, marked incomplete for the
    /// stage: the report as it stood plus any settled model requests and their usage.
    pub fn into_partial_report(self) -> LintReport {
        super::lint_run::failed(
            *self.progress.unwrap_or(self.report),
            self.stage.cli_message(),
        )
    }
}

#[cfg(test)]
#[path = "stopped_stages_test.rs"]
mod tests;
