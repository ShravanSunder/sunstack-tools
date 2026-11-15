use super::incomplete_formats::{incomplete_text, selection_incomplete_text, unavailable_text};
use crate::{InvalidConfigValue, *};
use std::{fmt::Write, path::Path, str::FromStr};
use sunstack_rules::{Anchor, Severity};

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
        LintStatus::Prepared => "Prepared",
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
fn human(report: &LintReport, prepare_file: Option<&Path>, max_warnings: Option<u32>) -> String {
    let mut output = format!("Scope: {}\n", scope_text(&report.scope));
    if !report.filter_answers.is_empty() {
        let mut labels = std::collections::BTreeMap::<String, usize>::new();
        let mut unsure = 0;
        let mut incomplete = 0;
        let mut unavailable = 0;
        for answer in &report.filter_answers {
            match &answer.state {
                sunstack_rules::FilterAnswerState::Judged { label, .. } => {
                    *labels
                        .entry(format!("{}={}", answer.filter.as_str(), label.as_str()))
                        .or_default() += 1
                }
                sunstack_rules::FilterAnswerState::Unsure { .. } => unsure += 1,
                sunstack_rules::FilterAnswerState::Incomplete { .. } => incomplete += 1,
                sunstack_rules::FilterAnswerState::Unavailable { .. } => unavailable += 1,
            }
        }
        let counts = labels
            .into_iter()
            .map(|(label, count)| format!("{label}: {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            output,
            "Filters: {counts}; {unsure} unsure, {incomplete} incomplete, {unavailable} unavailable"
        );
    }
    for skip in &report.filter_skips {
        let _ = writeln!(
            output,
            "{}:{}: skipped [{}] filter {} ({:?}) #{}",
            skip.file.display(),
            skip.locations
                .first()
                .map_or(1, |location| location.first_line),
            skip.rule.as_str(),
            skip.filter.as_str(),
            skip.why,
            skip.id
        );
    }
    for finding in &report.findings {
        let column = if finding.anchor == Anchor::Resolved {
            1
        } else {
            finding.span.as_ref().map_or(1, |span| span.start_column)
        };
        let location = format!("{}:{}:{column}", finding.file.display(), finding.line);
        let _ = writeln!(
            output,
            "{location}: {} [{}] {} #{}",
            severity_name(finding.severity),
            finding.rule.as_str(),
            finding.message,
            finding.id
        );
        if let Some(omission) = &finding.locator_omitted {
            let _ = writeln!(
                output,
                "  locator {} omitted: {} candidate lines exceed limit {}; using default line",
                omission.locator.as_str(),
                omission.candidate_lines,
                omission.limit
            );
        }
        for place in &finding.related {
            let _ = writeln!(
                output,
                "  {}: {}:{}",
                place.label,
                place.file.display(),
                place.first_line
            );
        }
    }
    let mut totals = std::collections::BTreeMap::<String, [usize; 6]>::new();
    for file in &report.file_counts {
        for (kind, count) in &file.kinds {
            if let Some(selection) = &count.selection {
                let total = totals.entry(kind.as_str().into()).or_default();
                total[0] += selection.candidates;
                total[1] += selection.selected;
                total[2] += selection.not_selected;
                total[3] += selection.uncertain;
                total[4] += selection.incomplete;
                total[5] += selection.unavailable;
            }
        }
    }
    for (kind, count) in totals {
        let _ = writeln!(
            output,
            "Selection {kind}: {} candidates, {} selected, {} not_selected, {} uncertain, {} incomplete, {} unavailable",
            count[0], count[1], count[2], count[3], count[4], count[5]
        );
    }
    for file in &report.file_counts {
        for (kind, count) in &file.kinds {
            if let Some(selection) = &count.selection
                && selection.candidates > 0
                && selection.selected == 0
            {
                let pending = report
                    .selections
                    .iter()
                    .filter(|entry| {
                        entry.kind == *kind
                            && entry.location.file == file.file
                            && matches!(
                                entry.outcome,
                                sunstack_rules::SelectionOutcome::Prepared { .. }
                                    | sunstack_rules::SelectionOutcome::Deferred
                            )
                    })
                    .count();
                let kind = kind.as_str();
                if pending > 0 {
                    let _ = writeln!(
                        output,
                        "{} [{kind}]: {pending} candidates awaiting prepared selection answers",
                        file.file.display()
                    );
                    continue;
                }
                let _ = writeln!(
                    output,
                    "{} [{kind}]: {} candidates, none selected; {} not_selected, {} uncertain, {} incomplete, {} unavailable ({} asked)",
                    file.file.display(),
                    selection.candidates,
                    selection.not_selected,
                    selection.uncertain,
                    selection.incomplete,
                    selection.unavailable,
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
    let mut unsure_by_rule = std::collections::BTreeMap::new();
    for result in &report.uncertain {
        let (count, _) = unsure_by_rule
            .entry(result.rule.as_str())
            .or_insert((0usize, result.id));
        *count += 1;
    }
    for (rule, (count, first_id)) in unsure_by_rule {
        let _ = writeln!(
            output,
            "{count} unsure [{rule}]; details: lint show <report> {first_id} (save with --report <report>)"
        );
    }
    for result in &report.results {
        if let RulePieceEnding::Incomplete { gap: Some(gap) } = &result.ending {
            let reason = incomplete_text(
                &crate::lint_reporting::outcome::rulebook_incomplete_reason(*gap),
            );
            let _ = writeln!(
                output,
                "{}:{}-{}: incomplete [{}] {} #{}",
                result.file.display(),
                result.first_line,
                result.last_line,
                result.name.as_str(),
                reason,
                result.id
            );
        }
    }
    for result in &report.results {
        if let RulePieceEnding::Unavailable { reason } = &result.ending {
            let _ = writeln!(
                output,
                "{}:{}-{}: unavailable [{}] {} #{}",
                result.file.display(),
                result.first_line,
                result.last_line,
                result.name.as_str(),
                unavailable_text(reason),
                result.id
            );
        }
    }
    let unavailable = report.unavailable_counts();
    if unavailable != UnavailableCounts::default() {
        let _ = writeln!(
            output,
            "Unavailable: {} rule pieces, {} filter folders, {} selection candidates, {} evidence scopes",
            unavailable.rule_pieces,
            unavailable.filter_folders,
            unavailable.selection_candidates,
            unavailable.evidence_scopes
        );
    }
    for diagnostic in &report.diagnostics {
        let _ = writeln!(
            output,
            "{}:{}:{}: {}",
            diagnostic.location_path().map_or_else(
                || report.config.display().to_string(),
                |path| path.display().to_string()
            ),
            diagnostic.line.unwrap_or(1),
            diagnostic.column.unwrap_or(1),
            diagnostic.message
        );
    }
    for incomplete in report.incomplete.iter().filter(|part| part.rule.is_none()) {
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
    match report.status {
        LintStatus::Clean if report.counts.warn > 0 => {
            let noun = if report.counts.warn == 1 {
                "warning"
            } else {
                "warnings"
            };
            let reason = max_warnings.map_or_else(
                || "because no --max-warnings was set".to_owned(),
                |maximum| format!("within --max-warnings {maximum}"),
            );
            let _ = writeln!(
                output,
                "Completed with {} {noun}; exit 0 {reason}",
                report.counts.warn
            );
        }
        LintStatus::Invalid | LintStatus::Cancelled | LintStatus::NothingSelected => {
            let diagnostic_count = report.diagnostics.len();
            let problem_word = if diagnostic_count == 1 {
                "problem"
            } else {
                "problems"
            };
            let _ = writeln!(
                output,
                "{}: {diagnostic_count} {problem_word}",
                status_text(report.status)
            );
        }
        LintStatus::Prepared
        | LintStatus::Clean
        | LintStatus::Findings
        | LintStatus::Incomplete
        | LintStatus::Uncertain => {
            let _ = write!(output, "{}", status_text(report.status));
            if report.status == LintStatus::Prepared {
                let request_count = report.requests.len();
                let request_word = if request_count == 1 {
                    "request"
                } else {
                    "requests"
                };
                let _ = write!(output, " {request_count} {request_word}");
                if let Some(path) = prepare_file {
                    let _ = write!(output, " in {}", path.display());
                }
            }
            let _ = writeln!(
                output,
                ": {} error(s), {} warning(s)",
                report.counts.error, report.counts.warn
            );
        }
    }
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
        if finding.anchor == Anchor::Resolved {
            let _ = write!(properties, ",line={},col=1", finding.line);
        } else if let Some(span) = &finding.span {
            let _ = write!(
                properties,
                ",line={},col={},endLine={},endColumn={}",
                finding.line, span.start_column, span.end_line, span.end_column
            );
        } else {
            let _ = write!(properties, ",line={}", finding.line);
        }
        let _ = writeln!(
            output,
            "::{} {}::{}",
            transport_level(finding.severity),
            properties,
            annotation_text(&format!("[{}] {}", finding.rule.as_str(), finding.message))
        );
        for place in &finding.related {
            let _ = writeln!(
                output,
                "  {}: {}:{}",
                place.label,
                place.file.display(),
                place.first_line
            );
        }
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
        if let Some(file) = diagnostic.location_path() {
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
/// Render with no warning limit; callers using a limit pass it to `render_with_warning_limit`.
pub fn render(report: &LintReport, format: OutputFormat) -> String {
    render_with_warning_limit(report, format, None)
}

/// Render the human exit-policy explanation with the caller's actual warning limit.
pub fn render_with_warning_limit(
    report: &LintReport,
    format: OutputFormat,
    max_warnings: Option<u32>,
) -> String {
    match format {
        OutputFormat::Default => human(report, None, max_warnings),
        OutputFormat::Github => github(report),
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .unwrap_or_else(|_| "Unable to encode lint report.\n".into()),
        OutputFormat::Sarif => crate::lint_reporting::sarif_format::render_sarif(report)
            .unwrap_or_else(|_| "Unable to encode SARIF report.\n".into()),
    }
}

pub fn render_prepared(report: &LintReport, format: OutputFormat, prepare_file: &Path) -> String {
    match format {
        OutputFormat::Default => human(report, Some(prepare_file), None),
        _ => render(report, format),
    }
}

#[cfg(test)]
#[path = "formats_test.rs"]
mod tests;
