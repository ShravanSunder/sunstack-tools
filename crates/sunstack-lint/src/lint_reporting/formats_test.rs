#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
use std::path::Path;

fn summary_report(status: LintStatus, diagnostic_count: usize) -> LintReport {
    let mut report = LintReport::empty(
        "sunstack.lint.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report.status = status;
    report.diagnostics = (0..diagnostic_count)
        .map(|_| LintDiagnostic {
            file: None,
            path: None,
            line: None,
            column: None,
            code: LintInvalidCode::Syntax,
            message: "Cannot read the configured rulebook".into(),
        })
        .collect();
    report
}

fn prepared_report(request_count: usize) -> LintReport {
    let mut report = summary_report(LintStatus::Prepared, 0);
    report.preparation = Some(PreparedReport {
        mode: PrepareMode::Prepare,
        needs_answer: vec![],
    });
    report.requests = (1..=request_count)
        .map(|id| {
            serde_json::from_value(json!({
                "id": id,
                "owner": {
                    "kind": "selection",
                    "selection_kind": "section",
                    "file": "docs/one.md"
                },
                "requested_model": "controlled",
                "resolved_model": null,
                "request": {"state": "First line.", "questions": {}},
                "state": "prepared",
                "status": "prepared"
            }))
            .unwrap()
        })
        .collect();
    // Saved requests are not sent model requests.
    report.usage.model_requests = 0;
    report
}

#[test]
fn invalid_summary_reports_diagnostics_instead_of_finding_counts() {
    // Arrange
    let report = summary_report(LintStatus::Invalid, 1);

    // Act
    let output = render(&report, OutputFormat::Default);

    // Assert
    assert_eq!(output.lines().last(), Some("Invalid input: 1 problem"));
    assert!(!output.contains("error(s)"), "{output}");
    assert!(!output.contains("warning(s)"), "{output}");
    assert!(output.contains("Cannot read the configured rulebook"));
}

#[test]
fn cancelled_summary_reports_diagnostics_instead_of_finding_counts() {
    // Arrange
    let report = summary_report(LintStatus::Cancelled, 2);

    // Act
    let output = render(&report, OutputFormat::Default);

    // Assert
    assert_eq!(output.lines().last(), Some("Cancelled: 2 problems"));
    assert!(!output.contains("error(s)"), "{output}");
    assert!(!output.contains("warning(s)"), "{output}");
}

#[test]
fn nothing_selected_summary_reports_zero_diagnostics_without_finding_counts() {
    // Arrange
    let report = summary_report(LintStatus::NothingSelected, 0);

    // Act
    let output = render(&report, OutputFormat::Default);

    // Assert
    assert_eq!(
        output.lines().last(),
        Some("No checks selected: 0 problems")
    );
    assert!(!output.contains("error(s)"), "{output}");
    assert!(!output.contains("warning(s)"), "{output}");
}

#[test]
fn evaluated_status_summaries_keep_finding_counts() {
    for (status, expected) in [
        (LintStatus::Clean, "Clean: 0 error(s), 0 warning(s)"),
        (
            LintStatus::Findings,
            "Lint findings: 2 error(s), 3 warning(s)",
        ),
        (
            LintStatus::Incomplete,
            "Incomplete: 2 error(s), 3 warning(s)",
        ),
        (LintStatus::Uncertain, "Uncertain: 2 error(s), 3 warning(s)"),
    ] {
        // Arrange
        let mut report = summary_report(status, 1);
        if status != LintStatus::Clean {
            report.counts = SeverityCounts { error: 2, warn: 3 };
        }

        // Act
        let output = render(&report, OutputFormat::Default);

        // Assert
        assert_eq!(output.lines().last(), Some(expected), "{status:?}");
    }
}

#[test]
fn prepared_summary_names_saved_request_count_and_destination() {
    for (request_count, expected) in [
        (
            0,
            "Prepared 0 requests in tmp/review requests.json: 0 error(s), 0 warning(s)",
        ),
        (
            1,
            "Prepared 1 request in tmp/review requests.json: 0 error(s), 0 warning(s)",
        ),
        (
            2,
            "Prepared 2 requests in tmp/review requests.json: 0 error(s), 0 warning(s)",
        ),
    ] {
        // Arrange
        let report = prepared_report(request_count);

        // Act
        let output = render_prepared(
            &report,
            OutputFormat::Default,
            Path::new("tmp/review requests.json"),
        );

        // Assert
        assert_eq!(output.lines().last(), Some(expected));
    }
}

#[test]
fn prepared_summary_without_destination_names_only_saved_request_count() {
    // Arrange
    let report = prepared_report(2);

    // Act
    let output = render(&report, OutputFormat::Default);

    // Assert
    assert_eq!(
        output.lines().last(),
        Some("Prepared 2 requests: 0 error(s), 0 warning(s)")
    );
}

#[test]
fn prepared_destination_preserves_transport_outputs_and_saved_report() {
    // Arrange
    let report = prepared_report(1);
    let saved_before = serde_json::to_value(&report).unwrap();

    for format in [
        OutputFormat::Json,
        OutputFormat::Github,
        OutputFormat::Sarif,
    ] {
        // Act
        let output = render_prepared(&report, format, Path::new("tmp/review requests.json"));

        // Assert
        assert_eq!(output, render(&report, format), "{format:?}");
    }
    assert_eq!(serde_json::to_value(&report).unwrap(), saved_before);
}

#[test]
fn successful_warning_summary_names_actual_policy_without_changing_transports() {
    let mut report = summary_report(LintStatus::Clean, 0);
    report.counts.warn = 2;
    let before = serde_json::to_value(&report).unwrap();
    assert!(
        render(&report, OutputFormat::Default)
            .contains("Completed with 2 warnings; exit 0 because no --max-warnings was set")
    );
    assert!(
        render_with_warning_limit(&report, OutputFormat::Default, Some(3))
            .contains("Completed with 2 warnings; exit 0 within --max-warnings 3")
    );
    for format in [
        OutputFormat::Json,
        OutputFormat::Github,
        OutputFormat::Sarif,
    ] {
        assert_eq!(
            render(&report, format),
            render_with_warning_limit(&report, format, Some(3))
        );
    }
    assert_eq!(serde_json::to_value(&report).unwrap(), before);
}
