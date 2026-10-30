use crate::{InvalidConfigValue, *};
use std::{fmt::Write, str::FromStr};
use sunstack_rules::Severity;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Default,
    Json,
    Github,
    Sarif,
}
impl FromStr for OutputFormat {
    type Err = InvalidConfigValue;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "json" => Ok(Self::Json),
            "github" => Ok(Self::Github),
            "sarif" => Ok(Self::Sarif),
            _ => Err(InvalidConfigValue(
                "use default, json, github or sarif for --format",
            )),
        }
    }
}
fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warn",
    }
}
// GitHub and SARIF prescribe "warning" as their transport level.
pub(crate) fn transport_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
    }
}
pub(crate) fn incomplete_text(reason: &IncompleteReason) -> &'static str {
    match reason {
        IncompleteReason::SelectionFailed => "selection failed",
        IncompleteReason::EvaluationFailed => "evaluation failed",
        IncompleteReason::NotStarted => "not started",
        IncompleteReason::PreparationFailed => "preparation failed",
        IncompleteReason::CoveragePartial => "partial rule coverage",
        IncompleteReason::CoverageNone => "no rule coverage",
        IncompleteReason::ProviderFailure => "provider failure",
        IncompleteReason::RequestLimitReached => "request limit reached",
        IncompleteReason::EvidenceUnavailable => "evidence unavailable",
        IncompleteReason::EvidenceTooLarge => "evidence too large",
        IncompleteReason::RequestTooLarge => "request too large",
    }
}
fn scope_text(scope: &ScopeReport) -> String {
    let scope_name = match &scope.kind {
        ScopeChoice::Default => "uncommitted (staged, unstaged and untracked)".into(),
        ScopeChoice::All => "all configured targets".into(),
        ScopeChoice::Since { rev } => format!("changes since {rev}"),
    };
    if scope.widened.is_some() {
        format!("{scope_name}; config changed, so scope was widened")
    } else {
        scope_name
    }
}
fn status_text(status: LintStatus) -> &'static str {
    match status {
        LintStatus::Clean => "Clean",
        LintStatus::NothingSelected => "No checks selected",
        LintStatus::Findings => "Lint findings",
        LintStatus::Incomplete => "Incomplete",
        LintStatus::Invalid => "Invalid input",
        LintStatus::Cancelled => "Cancelled",
        LintStatus::Uncertain => "Uncertain",
    }
}
pub(crate) fn uncertain_text(result: &UncertainResult) -> String {
    format!(
        "Uncertain [{} {}] at step {}: state {}; answers {}",
        result.rule.as_str(),
        result.marker.as_str(),
        result.step.as_str(),
        serde_json::to_string(&result.state).unwrap_or_else(|_| "unavailable".into()),
        serde_json::to_string(&result.answers).unwrap_or_else(|_| "unavailable".into())
    )
}
fn human(report: &LintReport) -> String {
    let mut output = format!("Scope: {}\n", scope_text(&report.scope));
    for finding in &report.findings {
        let location = finding.span.as_ref().map_or_else(
            || finding.file.display().to_string(),
            |span| {
                format!(
                    "{}:{}:{}",
                    finding.file.display(),
                    span.start_line,
                    span.start_column
                )
            },
        );
        let _ = writeln!(
            output,
            "{location}: {} [{}] {}",
            severity_name(finding.severity),
            finding.rule.as_str(),
            finding.message
        );
    }
    for result in &report.uncertain {
        let _ = writeln!(
            output,
            "{}:{}: {}",
            result.location.file.display(),
            result.location.first_line,
            uncertain_text(result)
        );
    }
    for diagnostic in &report.diagnostics {
        let _ = writeln!(
            output,
            "{}:{}:{}: {}",
            diagnostic.file.as_ref().map_or_else(
                || report.config.display().to_string(),
                |path| path.display().to_string()
            ),
            diagnostic.line.unwrap_or(1),
            diagnostic.column.unwrap_or(1),
            diagnostic.message
        );
    }
    for incomplete in &report.incomplete {
        let _ = writeln!(
            output,
            "Incomplete: {} ({})",
            incomplete
                .rule
                .as_ref()
                .map_or_else(|| "run".into(), |rule| rule.as_str().to_owned()),
            incomplete_text(&incomplete.reason)
        );
    }
    let _ = writeln!(
        output,
        "{}: {} error(s), {} warning(s)",
        status_text(report.status),
        report.counts.error,
        report.counts.warn
    );
    if report.usage.model_requests > 0 {
        let _ = writeln!(
            output,
            "Model requests: {}; input tokens: {}; output tokens: {}; cost USD: {} ({} reported, {} missing)",
            report.usage.model_requests,
            report
                .usage
                .input_tokens
                .map_or_else(|| "not reported".into(), |tokens| tokens.to_string()),
            report
                .usage
                .output_tokens
                .map_or_else(|| "not reported".into(), |tokens| tokens.to_string()),
            report
                .usage
                .cost_usd
                .map_or_else(|| "not reported".into(), |cost| cost.to_string()),
            report.usage.cost_reported_requests,
            report.usage.cost_missing_requests
        );
    }
    output
}
fn annotation_text(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}
fn annotation_property(text: &str) -> String {
    annotation_text(text)
        .replace(':', "%3A")
        .replace(',', "%2C")
}
fn github(report: &LintReport) -> String {
    let mut output = String::new();
    for finding in &report.findings {
        let mut properties = format!(
            "file={}",
            annotation_property(&finding.file.display().to_string())
        );
        if let Some(span) = &finding.span {
            let _ = write!(
                properties,
                ",line={},col={},endLine={},endColumn={}",
                span.start_line, span.start_column, span.end_line, span.end_column
            );
        }
        let _ = writeln!(
            output,
            "::{} {}::{}",
            transport_level(finding.severity),
            properties,
            annotation_text(&format!("[{}] {}", finding.rule.as_str(), finding.message))
        );
    }
    for result in &report.uncertain {
        let _ = writeln!(
            output,
            "::notice file={},line={}::{}",
            annotation_property(&result.location.file.display().to_string()),
            result.location.first_line,
            annotation_text(&uncertain_text(result))
        );
    }
    for diagnostic in &report.diagnostics {
        let mut properties = String::new();
        if let Some(file) = &diagnostic.file {
            let _ = write!(
                properties,
                "file={}",
                annotation_property(&file.display().to_string())
            );
        }
        if let Some(line) = diagnostic.line {
            let _ = write!(properties, ",line={line}");
        }
        if let Some(column) = diagnostic.column {
            let _ = write!(properties, ",col={column}");
        }
        let _ = writeln!(
            output,
            "::error {}::{}",
            properties.trim_start_matches(','),
            annotation_text(&diagnostic.message)
        );
    }
    let _ = writeln!(
        output,
        "::notice::{}; scope: {}",
        status_text(report.status),
        annotation_text(&scope_text(&report.scope))
    );
    output
}
pub fn render(report: &LintReport, format: OutputFormat) -> String {
    match format {
        OutputFormat::Default => human(report),
        OutputFormat::Github => github(report),
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .unwrap_or_else(|_| "Unable to encode lint report.\n".into()),
        OutputFormat::Sarif => crate::sarif_format::render_sarif(report)
            .unwrap_or_else(|_| "Unable to encode SARIF report.\n".into()),
    }
}
