use crate::{
    LintReport, LintStatus, ScopeReport,
    lint_reporting::formats::{incomplete_text, transport_level, uncertain_text},
};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct SarifDocument<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun<'a>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun<'a> {
    tool: SarifTool<'a>,
    column_kind: &'static str,
    results: Vec<SarifResult<'a>>,
    invocations: Vec<SarifInvocation<'a>>,
    properties: SarifProperties<'a>,
}
#[derive(Serialize)]
struct SarifTool<'a> {
    driver: SarifDriver<'a>,
}
#[derive(Serialize)]
struct SarifDriver<'a> {
    name: &'static str,
    version: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult<'a> {
    rule_id: String,
    level: &'static str,
    message: SarifMessage<'a>,
    locations: Vec<SarifLocation>,
}
#[derive(Serialize)]
struct SarifMessage<'a> {
    text: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifInvocation<'a> {
    execution_successful: bool,
    tool_execution_notifications: Vec<SarifNotification<'a>>,
}
#[derive(Serialize)]
struct SarifNotification<'a> {
    level: &'static str,
    message: SarifMessage<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    locations: Vec<SarifLocation>,
}
#[derive(Serialize)]
struct SarifProperties<'a> {
    status: LintStatus,
    scope: &'a ScopeReport,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<SarifRegion>,
}
#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
}
fn artifact_uri(file: &Path) -> String {
    file.to_string_lossy()
        .as_bytes()
        .iter()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~' | b'/') {
                char::from(*byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
fn location(file: &Path, region: Option<SarifRegion>) -> SarifLocation {
    SarifLocation {
        physical_location: SarifPhysicalLocation {
            artifact_location: SarifArtifactLocation {
                uri: artifact_uri(file),
            },
            region,
        },
    }
}
pub(crate) fn render_sarif(report: &LintReport) -> Result<String, serde_json::Error> {
    let results = report
        .findings
        .iter()
        .map(|finding| SarifResult {
            rule_id: finding.rule.as_str().to_owned(),
            level: transport_level(finding.severity),
            message: SarifMessage {
                text: &finding.message,
            },
            locations: vec![location(
                &finding.file,
                finding.span.as_ref().map(|span| SarifRegion {
                    start_line: span.start_line,
                    start_column: Some(span.start_column),
                    end_line: Some(span.end_line),
                    end_column: Some(span.end_column),
                }),
            )],
        })
        .collect();
    let mut notifications = report
        .diagnostics
        .iter()
        .map(|diagnostic| SarifNotification {
            level: "error",
            message: SarifMessage {
                text: &diagnostic.message,
            },
            locations: diagnostic
                .file
                .iter()
                .map(|file| {
                    location(
                        file,
                        diagnostic
                            .line
                            .and_then(|line| usize::try_from(line).ok())
                            .map(|line| SarifRegion {
                                start_line: line,
                                start_column: diagnostic
                                    .column
                                    .and_then(|column| usize::try_from(column).ok()),
                                end_line: None,
                                end_column: None,
                            }),
                    )
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let incomplete_messages = report
        .incomplete
        .iter()
        .map(|part| incomplete_text(&part.reason))
        .collect::<Vec<_>>();
    notifications.extend(report.incomplete.iter().zip(&incomplete_messages).map(
        |(incomplete, message)| {
            SarifNotification {
                level: "error",
                message: SarifMessage { text: message },
                locations: incomplete
                    .file
                    .iter()
                    .map(|file| location(file, None))
                    .collect(),
            }
        },
    ));
    let uncertain_messages = report
        .uncertain
        .iter()
        .map(uncertain_text)
        .collect::<Vec<_>>();
    notifications.extend(report.uncertain.iter().zip(&uncertain_messages).map(
        |(result, message)| SarifNotification {
            level: "note",
            message: SarifMessage { text: message },
            locations: vec![location(
                &result.location.file,
                Some(SarifRegion {
                    start_line: result.location.first_line,
                    start_column: Some(result.location.span.start_column),
                    end_line: Some(result.location.span.end_line),
                    end_column: Some(result.location.span.end_column),
                }),
            )],
        },
    ));
    serde_json::to_string_pretty(&SarifDocument {
        schema: "https://json.schemastore.org/sarif-2.1.0.json",
        version: "2.1.0",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "sunstack",
                    version: &report.version,
                },
            },
            column_kind: "unicodeCodePoints",
            results,
            invocations: vec![SarifInvocation {
                execution_successful: matches!(
                    report.status,
                    LintStatus::Clean
                        | LintStatus::Findings
                        | LintStatus::NothingSelected
                        | LintStatus::Uncertain
                ),
                tool_execution_notifications: notifications,
            }],
            properties: SarifProperties {
                status: report.status,
                scope: &report.scope,
            },
        }],
    })
}
