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
pub(crate) fn incomplete_text(reason: &IncompleteReason) -> String {
    match reason {
        IncompleteReason::SelectionFailed => "selection failed",
        IncompleteReason::SelectionIncomplete => "selection incomplete",
        IncompleteReason::EvaluationFailed => "evaluation failed",
        IncompleteReason::NotStarted => "not started",
        IncompleteReason::PreparationFailed => "preparation failed",
        IncompleteReason::CoveragePartial => "partial rule coverage",
        IncompleteReason::CoverageNone => "no rule coverage",
        IncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        } => {
            let class = provider_failure_name(*failure_class);
            return match http_status {
                Some(status) => format!("provider failure ({class}, HTTP {status})"),
                None => format!("provider failure ({class})"),
            };
        }
        IncompleteReason::InvalidRequest => "invalid request",
        IncompleteReason::InvalidResponse => "invalid response",
        IncompleteReason::RequestLimitReached => "request limit reached",
        IncompleteReason::EvidenceUnavailable => "evidence unavailable",
        IncompleteReason::EvidenceTooLarge => "evidence too large",
        IncompleteReason::RequestTooLarge => "request too large",
    }
    .into()
}

pub(crate) fn selection_incomplete_text(entry: &SelectionEntry) -> Option<String> {
    use sunstack_rules::{SelectionIncompleteReason, SelectionOutcome};
    let SelectionOutcome::Incomplete { reason } = &entry.outcome else {
        return None;
    };
    let reason = match reason {
        SelectionIncompleteReason::EvidenceUnavailable { .. } => {
            IncompleteReason::EvidenceUnavailable
        }
        SelectionIncompleteReason::EvidenceTooLarge => IncompleteReason::EvidenceTooLarge,
        SelectionIncompleteReason::RequestTooLarge { .. } => IncompleteReason::RequestTooLarge,
        SelectionIncompleteReason::InvalidRequest => IncompleteReason::InvalidRequest,
        SelectionIncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        } => IncompleteReason::ProviderFailure {
            failure_class: *failure_class,
            http_status: *http_status,
        },
        SelectionIncompleteReason::InvalidResponse => IncompleteReason::InvalidResponse,
        SelectionIncompleteReason::RequestLimitReached => IncompleteReason::RequestLimitReached,
        SelectionIncompleteReason::Cancelled => IncompleteReason::NotStarted,
    };
    Some(format!(
        "{}:{} [{}]: selection incomplete ({})",
        entry.location.file.display(),
        entry.location.first_line,
        entry.kind.as_str(),
        incomplete_text(&reason)
    ))
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
    let mut totals = std::collections::BTreeMap::<String, [usize; 5]>::new();
    for file in &report.file_counts {
        for (kind, count) in &file.kinds {
            if let Some(selection) = &count.selection {
                let total = totals.entry(kind.as_str().into()).or_default();
                total[0] += selection.candidates;
                total[1] += selection.selected;
                total[2] += selection.not_selected;
                total[3] += selection.uncertain;
                total[4] += selection.incomplete;
            }
        }
    }
    for (kind, count) in totals {
        let _ = writeln!(
            output,
            "Selection {kind}: {} candidates, {} selected, {} not_selected, {} uncertain, {} incomplete",
            count[0], count[1], count[2], count[3], count[4]
        );
    }
    for file in &report.file_counts {
        for (kind, count) in &file.kinds {
            if let Some(selection) = &count.selection
                && selection.candidates > 0
                && selection.selected == 0
            {
                let kind = kind.as_str();
                let _ = writeln!(
                    output,
                    "{} [{kind}]: {} candidates, none selected; {} not_selected, {} uncertain, {} incomplete ({} asked)",
                    file.file.display(),
                    selection.candidates,
                    selection.not_selected,
                    selection.uncertain,
                    selection.incomplete,
                    selection.asked
                );
            }
        }
    }
    for detail in report
        .selections
        .iter()
        .filter_map(selection_incomplete_text)
    {
        let _ = writeln!(output, "{detail}");
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
        OutputFormat::Sarif => crate::lint_reporting::sarif_format::render_sarif(report)
            .unwrap_or_else(|_| "Unable to encode SARIF report.\n".into()),
    }
}

fn provider_failure_name(class: sunstack_decisions::ProviderFailureClass) -> &'static str {
    use sunstack_decisions::ProviderFailureClass;
    match class {
        ProviderFailureClass::ProviderRejectedRequest => "provider_rejected_request",
        ProviderFailureClass::Authentication => "authentication",
        ProviderFailureClass::Authorization => "authorization",
        ProviderFailureClass::InsufficientCredits => "insufficient_credits",
        ProviderFailureClass::ProviderResourceUnavailable => "provider_resource_unavailable",
        ProviderFailureClass::RequestTooLarge => "request_too_large",
        ProviderFailureClass::RateLimit => "rate_limit",
        ProviderFailureClass::ProviderUnavailable => "provider_unavailable",
        ProviderFailureClass::Timeout => "timeout",
        ProviderFailureClass::Connection => "connection",
        ProviderFailureClass::HttpError => "http_error",
        ProviderFailureClass::UnexpectedProviderError => "unexpected_provider_error",
    }
}
