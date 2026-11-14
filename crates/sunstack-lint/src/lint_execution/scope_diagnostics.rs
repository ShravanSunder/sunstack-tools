use crate::{
    IncompletePart, IncompleteReason, LintDiagnostic, LintInvalidCode, LintReport, LintStatus,
};
use std::path::Path;
use sunstack_evidence::{DiscoveryCoverage, DiscoveryLimit, DiscoveryLimits, EvidenceError};

fn scope_diagnostic(root: &Path, path: Option<&Path>, message: String) -> LintDiagnostic {
    LintDiagnostic {
        definition: None,
        json_path: vec![],
        file: None,
        path: path.map(|path| root.join(path)),
        line: None,
        column: None,
        code: LintInvalidCode::Syntax,
        message,
    }
}

pub(super) fn capture_refusal(
    mut report: LintReport,
    root: &Path,
    error: EvidenceError,
) -> LintReport {
    report.status = LintStatus::Invalid;
    let mut diagnostic = scope_diagnostic(
        root,
        error.path.as_deref(),
        format!("cannot resolve change scope: {:?}", error.code),
    );
    diagnostic.file = Some(report.config.clone());
    diagnostic.line = Some(1);
    diagnostic.column = Some(1);
    report.diagnostics.push(diagnostic);
    report
}

pub(super) fn incomplete_capture(
    mut report: LintReport,
    root: &Path,
    coverage: &DiscoveryCoverage,
    limits: &DiscoveryLimits,
) -> LintReport {
    let diagnostics = match coverage {
        DiscoveryCoverage::Limited { cutoffs, .. } => cutoffs
            .iter()
            .map(|cutoff| {
                let (setting, value) = limit_setting(cutoff.limit, limits);
                scope_diagnostic(
                    root,
                    cutoff.path.as_deref(),
                    format!("change discovery reached a limit: {setting}={value}"),
                )
            })
            .collect::<Vec<_>>(),
        DiscoveryCoverage::Unavailable { diagnostics } => diagnostics
            .items
            .iter()
            .map(|item| {
                scope_diagnostic(
                    root,
                    item.path.as_deref(),
                    format!("change discovery unavailable: {:?}", item.code),
                )
            })
            .collect(),
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. } => vec![],
    };
    report.status = LintStatus::Incomplete;
    report.incomplete.push(IncompletePart {
        rule: None,
        field: None,
        file: None,
        reason: IncompleteReason::PreparationFailed,
    });
    if diagnostics.is_empty() {
        report.diagnostics.push(scope_diagnostic(
            root,
            None,
            "change discovery did not complete".into(),
        ));
    } else {
        report.diagnostics.extend(diagnostics);
    }
    report
}

fn limit_setting(limit: DiscoveryLimit, limits: &DiscoveryLimits) -> (&'static str, usize) {
    match limit {
        DiscoveryLimit::Files => ("max_files", limits.max_files.get()),
        DiscoveryLimit::FileBytes => ("max_file_bytes", limits.max_file_bytes.get()),
        DiscoveryLimit::TotalBytes => ("max_total_bytes", limits.max_total_bytes.get()),
        DiscoveryLimit::Matches => ("max_matches", limits.max_matches.get()),
        DiscoveryLimit::UnitVisits => ("max_unit_visits", limits.max_unit_visits.get()),
        DiscoveryLimit::RelationshipCandidates => (
            "max_relationship_candidates",
            limits.max_relationship_candidates.get(),
        ),
        DiscoveryLimit::FanOut => ("max_fan_out", limits.max_fan_out.get()),
        DiscoveryLimit::HistoryCommits => ("max_history_commits", limits.max_history_commits.get()),
        DiscoveryLimit::LocalTime => ("max_local_time_ms", limits.max_local_time_ms.get()),
    }
}

#[cfg(test)]
#[path = "scope_diagnostics_test.rs"]
mod tests;
