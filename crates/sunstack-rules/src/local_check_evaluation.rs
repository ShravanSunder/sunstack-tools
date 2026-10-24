use crate::*;
use std::borrow::Cow;
use sunstack_evidence::{
    CaptureTail, DiscoveryCoverage, EvidenceCode, EvidenceSession, FrontmatterFieldContent,
    MarkdownByteRange, Omission, OmissionReason, SelectionContent, SelectionCoverage,
};
use tokio_util::sync::CancellationToken;

fn unresolved(diagnostics: DiagnosticList) -> LocalSettlement {
    LocalSettlement::Unresolved { diagnostics }
}
fn completed(passed: bool, observations: Vec<LocalObservation>) -> LocalSettlement {
    LocalSettlement::Completed {
        result: if passed {
            MechanicalResult::Pass
        } else {
            MechanicalResult::Fail
        },
        observations,
    }
}
fn expect_text(predicate: &ExpectPredicate, text: Option<&str>) -> Result<bool, DiagnosticList> {
    let Some(text) = text else {
        return Ok(false);
    };
    let invalid = || DiagnosticList::rule(RuleDiagnosticCode::InvalidMechanicalCheck);
    Ok(match predicate {
        ExpectPredicate::Matches { matches } => regex::Regex::new(matches)
            .map_err(|_| invalid())?
            .is_match(text),
        ExpectPredicate::NotMatches { not_matches } => !regex::Regex::new(not_matches)
            .map_err(|_| invalid())?
            .is_match(text),
        ExpectPredicate::StartsWith { starts_with } => text.starts_with(starts_with),
        ExpectPredicate::Present { present } => *present,
    })
}
fn incomplete_selection(omissions: &[Omission], coverage: &SelectionCoverage) -> DiagnosticList {
    let mut diagnostics = sunstack_evidence::DiagnosticList::default();
    for omission in omissions {
        match &omission.reason {
            OmissionReason::Limit => diagnostics.push(sunstack_evidence::Diagnostic::new(
                EvidenceCode::SelectorLimit,
            )),
            OmissionReason::OptionalUnavailable => diagnostics.push(
                sunstack_evidence::Diagnostic::new(EvidenceCode::SourceUnavailable),
            ),
            OmissionReason::Window | OmissionReason::Projection | OmissionReason::InputPattern => {}
        }
    }
    match coverage {
        SelectionCoverage::SingleSource { capture } => {
            if matches!(capture.tail, CaptureTail::Incomplete { .. }) {
                diagnostics.push(sunstack_evidence::Diagnostic::new(
                    EvidenceCode::MissingContext,
                ));
            }
        }
        SelectionCoverage::ChangeSet { discovery, .. } => match discovery {
            DiscoveryCoverage::CompleteWithinDeclaredScope { .. } => {}
            DiscoveryCoverage::Limited { .. } => diagnostics.push(
                sunstack_evidence::Diagnostic::new(EvidenceCode::DiscoveryLimit),
            ),
            DiscoveryCoverage::Unavailable {
                diagnostics: unavailable,
            } => {
                diagnostics.push(sunstack_evidence::Diagnostic::new(
                    EvidenceCode::SourceUnavailable,
                ));
                for diagnostic in &unavailable.items {
                    diagnostics.push(diagnostic.clone());
                }
                diagnostics.omitted_count += unavailable.omitted_count;
            }
        },
    }
    diagnostics.into()
}
fn check_expect(
    predicate: &ExpectPredicate,
    content: &SelectionContent,
    omissions: &[Omission],
    coverage: &SelectionCoverage,
) -> LocalSettlement {
    let selected_items = match content {
        SelectionContent::Matches { matches } => Some(matches.len()),
        SelectionContent::ConversationRecords { records, .. } => Some(records.len()),
        _ => None,
    };
    if let Some(count) = selected_items {
        let ExpectPredicate::Present { present } = predicate else {
            return unresolved(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget));
        };
        if count == 0 {
            let diagnostics = incomplete_selection(omissions, coverage);
            if !diagnostics.is_empty() {
                return unresolved(diagnostics);
            }
        }
        return completed(*present == (count > 0), vec![]);
    }
    let text = match content {
        SelectionContent::MarkdownUnits { units } if units.len() == 1 => {
            Some(Cow::Borrowed(units[0].visible_text.as_str()))
        }
        SelectionContent::FrontmatterField { result } => match result.as_ref() {
            FrontmatterFieldContent::Present { value, .. } => match value {
                sunstack_evidence::StructuredJson::String(text) => {
                    Some(Cow::Borrowed(text.as_str()))
                }
                _ => match serde_json::to_string(value) {
                    Ok(text) => Some(Cow::Owned(text)),
                    Err(_) => {
                        return unresolved(DiagnosticList::rule(
                            RuleDiagnosticCode::InvalidMechanicalCheck,
                        ));
                    }
                },
            },
            FrontmatterFieldContent::Absent { .. } => None,
            FrontmatterFieldContent::BlockUnavailable { diagnostics, .. } => {
                return unresolved(diagnostics.clone().into());
            }
        },
        _ => return unresolved(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget)),
    };
    match expect_text(predicate, text.as_deref()) {
        Ok(passed) => completed(passed, vec![]),
        Err(diagnostics) => unresolved(diagnostics),
    }
}

pub(crate) async fn settle_mechanical(
    check: &MechanicalCheck,
    content: &SelectionContent,
    omissions: &[Omission],
    coverage: &SelectionCoverage,
    session: &mut EvidenceSession,
    cancellation: &CancellationToken,
) -> LocalSettlement {
    if cancellation.is_cancelled() {
        return LocalSettlement::Cancelled;
    }
    let result = match check {
        MechanicalCheck::Expect { expect } => {
            let expect = expect.clone();
            let content = content.clone();
            let omissions = omissions.to_vec();
            let coverage = coverage.clone();
            tokio::task::spawn_blocking(move || {
                check_expect(&expect, &content, &omissions, &coverage)
            })
            .await
            .unwrap_or_else(|_| {
                unresolved(DiagnosticList::rule(
                    RuleDiagnosticCode::UnexpectedExecutionError,
                ))
            })
        }
        MechanicalCheck::AstGrep { ast_grep } => {
            let SelectionContent::MarkdownUnits { units } = content else {
                return unresolved(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget));
            };
            let [unit] = units.as_slice() else {
                return unresolved(DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget));
            };
            let matcher = match ast_grep.compile() {
                Ok(matcher) => matcher,
                Err(diagnostics) => return unresolved(diagnostics),
            };
            match session
                .markdown_matches(
                    &unit.span.source,
                    MarkdownByteRange {
                        start: unit.span.byte_start,
                        end: unit.span.byte_end,
                    },
                    matcher,
                )
                .await
            {
                Ok(matches) => completed(
                    matches.is_empty(),
                    matches
                        .into_iter()
                        .map(|span| LocalObservation::AstMatch { span })
                        .collect(),
                ),
                Err(error) => unresolved(error.diagnostics().into()),
            }
        }
    };
    if cancellation.is_cancelled() {
        LocalSettlement::Cancelled
    } else {
        result
    }
}
