use crate::{input_binding::AdmittedSources, *};
use sunstack_evidence::{
    AssistantSelection, ConversationBody, DeclarationId, EvidenceDeclaration, EvidenceRequirement,
    EvidenceSelection, EvidenceSession, Inclusion, MessageBlock, SelectionContent,
    SelectionReadiness, SourceName, SourceSpec, ToolFilter, ToolResult, ToolUseSelection,
    TurnWindow,
};
use tokio_util::sync::CancellationToken;
pub(crate) struct ConditionContext<'invocation> {
    pub sources: &'invocation AdmittedSources,
    pub origin: &'invocation InvocationOrigin,
    pub session: &'invocation mut EvidenceSession,
}
fn unresolved() -> ConditionOutcome {
    ConditionOutcome::Unresolved {
        diagnostic: ReportDiagnostic::Rule(RuleDiagnostic::new(
            RuleDiagnosticCode::ConditionUnresolved,
        )),
    }
}
fn comparison(result: bool) -> ConditionOutcome {
    if result {
        ConditionOutcome::Passed
    } else {
        ConditionOutcome::DidNotPass
    }
}
pub(crate) fn condition_basis(test: &ConditionTest) -> ConditionBasis {
    match test {
        ConditionTest::PayloadFieldEquals { input, pointer, .. } => {
            ConditionBasis::PayloadFieldEquals {
                input: input.clone(),
                pointer: pointer.clone(),
            }
        }
        ConditionTest::PayloadFieldIn { input, pointer, .. } => ConditionBasis::PayloadFieldIn {
            input: input.clone(),
            pointer: pointer.clone(),
        },
        ConditionTest::PayloadFieldPresent { input, pointer } => {
            ConditionBasis::PayloadFieldPresent {
                input: input.clone(),
                pointer: pointer.clone(),
            }
        }
        ConditionTest::OriginFieldEquals { field, .. } => {
            ConditionBasis::OriginFieldEquals { field: *field }
        }
        ConditionTest::PatternPresent {
            input,
            pattern,
            max_scan_bytes,
        } => ConditionBasis::PatternPresent {
            input: input.clone(),
            pattern_kind: match pattern {
                PresencePattern::Literal { .. } => PatternKind::Literal,
                PresencePattern::Regex { .. } => PatternKind::Regex,
                PresencePattern::Ast { .. } => PatternKind::Ast,
            },
            max_scan_bytes: max_scan_bytes.get(),
        },
    }
}
pub(crate) async fn evaluate_condition(
    test: &ConditionTest,
    context: ConditionContext<'_>,
) -> ConditionOutcome {
    match test {
        ConditionTest::OriginFieldEquals { field, value } => context
            .origin
            .field(*field)
            .map_or_else(unresolved, |observed| comparison(observed == value)),
        ConditionTest::PayloadFieldEquals {
            input,
            pointer,
            value,
        } => payload_field(context.sources, input, pointer)
            .map_or_else(unresolved, |observed| comparison(observed == value)),
        ConditionTest::PayloadFieldIn {
            input,
            pointer,
            values,
        } => payload_field(context.sources, input, pointer)
            .map_or_else(unresolved, |observed| comparison(values.contains(observed))),
        ConditionTest::PayloadFieldPresent { input, pointer } => {
            comparison(payload_field(context.sources, input, pointer).is_some())
        }
        ConditionTest::PatternPresent {
            input,
            pattern,
            max_scan_bytes,
        } => {
            let Some(source) = context.sources.get(input) else {
                return unresolved();
            };
            let select = match source.spec {
                SourceSpec::Document { .. } => EvidenceSelection::WholeDocument,
                SourceSpec::Conversation { .. } => EvidenceSelection::Conversation {
                    turns: TurnWindow::All,
                    user_messages: Inclusion::Included,
                    assistant_messages: AssistantSelection::All,
                    tool_use: ToolUseSelection::Included {
                        tools: ToolFilter::All,
                        results: Inclusion::Included,
                    },
                    hook_injected: Inclusion::Included,
                },
                _ => return unresolved(),
            };
            let Ok(id) = DeclarationId::new("condition_scan") else {
                return unresolved();
            };
            let result = context
                .session
                .select(&EvidenceDeclaration {
                    id,
                    input: source.name.clone(),
                    requirement: EvidenceRequirement::Required,
                    select,
                    max_items: None,
                    max_bytes: None,
                })
                .await;
            let text = match result.readiness {
                SelectionReadiness::Ready {
                    content: SelectionContent::DocumentText { text, .. },
                    ..
                } => text,
                SelectionReadiness::Ready {
                    content: SelectionContent::ConversationRecords { records, .. },
                    ..
                } => normalized_text(&records),
                _ => return unresolved(),
            };
            if text.len() > max_scan_bytes.get() {
                return unresolved();
            }
            let pattern = pattern.clone();
            match tokio::task::spawn_blocking(move || pattern_present(&text, &pattern)).await {
                Ok(Ok(present)) => comparison(present),
                _ => unresolved(),
            }
        }
    }
}
fn payload_field<'value>(
    sources: &'value AdmittedSources,
    input: &SourceName,
    pointer: &str,
) -> Option<&'value sunstack_evidence::StructuredJson> {
    match &sources.get(input)?.spec {
        SourceSpec::Payload { value, .. } => value.pointer(pointer),
        _ => None,
    }
}
fn normalized_text(records: &[sunstack_evidence::ConversationRecord]) -> String {
    let mut text = String::new();
    for record in records {
        match &record.body {
            ConversationBody::UserMessage { blocks }
            | ConversationBody::AssistantMessage { blocks } => {
                for block in blocks {
                    match block {
                        MessageBlock::Text { text: part }
                        | MessageBlock::Thinking { text: part } => {
                            text.push_str(part);
                            text.push('\n');
                        }
                        MessageBlock::Unsupported { content, .. } => {
                            if let Ok(part) = serde_json::to_string(content) {
                                text.push_str(&part);
                                text.push('\n');
                            }
                        }
                        MessageBlock::ToolCall { .. } => {}
                    }
                }
            }
            ConversationBody::ToolUse(tool) => {
                text.push_str(&tool.tool_name);
                text.push('\n');
                if let Ok(input) = serde_json::to_string(&tool.input) {
                    text.push_str(&input);
                    text.push('\n');
                }
                match &tool.result {
                    ToolResult::FullObserved { text: part }
                    | ToolResult::Clipped { text: part, .. } => {
                        text.push_str(part);
                        text.push('\n');
                    }
                    ToolResult::Absent => {}
                }
            }
        }
    }
    text
}
fn pattern_present(text: &str, pattern: &PresencePattern) -> Result<bool, ()> {
    match pattern {
        PresencePattern::Literal { text: needle } => Ok(text.contains(needle)),
        PresencePattern::Regex { pattern } => regex::Regex::new(pattern)
            .map(|pattern| pattern.is_match(text))
            .map_err(|_| ()),
        PresencePattern::Ast { language, pattern } => {
            use ast_grep_language::LanguageExt;
            let language: ast_grep_language::SupportLang = language.parse().map_err(|_| ())?;
            let pattern = ast_grep_core::Pattern::try_new(pattern, language).map_err(|_| ())?;
            Ok(language.ast_grep(text).root().find(pattern).is_some())
        }
    }
}
pub(crate) async fn evaluate_conditions(
    entries: &[ApplicabilityEntry],
    owner: EntryOwner,
    sources: &AdmittedSources,
    origin: &InvocationOrigin,
    session: &mut EvidenceSession,
    cancellation: &CancellationToken,
) -> (ApplicabilityOutcome, Vec<EvaluatedEntry>) {
    let mut evaluated = Vec::new();
    let mut outcome = ApplicabilityOutcome::Applicable;
    for (position, entry) in entries.iter().enumerate() {
        let key = EntryKey {
            owner: owner.clone(),
            position,
        };
        if cancellation.is_cancelled() {
            evaluated.push(EvaluatedEntry {
                key,
                result: EntryResult::NotEvaluated {
                    reason: NotEvaluatedReason::Cancelled,
                },
            });
            outcome = ApplicabilityOutcome::NotEvaluated {
                reason: NotEvaluatedReason::Cancelled,
            };
            continue;
        }
        if !matches!(outcome, ApplicabilityOutcome::Applicable) {
            evaluated.push(EvaluatedEntry {
                key,
                result: EntryResult::NotEvaluated {
                    reason: NotEvaluatedReason::EarlierEntryStopped,
                },
            });
            continue;
        }
        let ApplicabilityEntry::Condition { test } = entry else {
            break;
        };
        let basis = condition_basis(test);
        let result = evaluate_condition(
            test,
            ConditionContext {
                sources,
                origin,
                session,
            },
        )
        .await;
        outcome = match &result {
            ConditionOutcome::Passed => ApplicabilityOutcome::Applicable,
            ConditionOutcome::DidNotPass => ApplicabilityOutcome::NotApplicable {
                entry: key.clone(),
                basis: EntryBasis::Condition {
                    test: basis.clone(),
                },
            },
            ConditionOutcome::Unresolved { diagnostic } => {
                ApplicabilityOutcome::ApplicabilityFailed {
                    entry: key.clone(),
                    cause: Cause::Condition {
                        entry: key.clone(),
                        diagnostic: diagnostic.clone(),
                    },
                }
            }
        };
        evaluated.push(EvaluatedEntry {
            key,
            result: EntryResult::Condition {
                outcome: result,
                basis,
            },
        });
    }
    (outcome, evaluated)
}
pub(crate) fn stopped_occurrence(
    key: RuleOccurrenceKey,
    outcome: &ApplicabilityOutcome,
) -> Option<RuleOccurrenceResult> {
    match outcome {
        ApplicabilityOutcome::Applicable => None,
        ApplicabilityOutcome::NotApplicable { entry, basis } => {
            Some(RuleOccurrenceResult::NotApplicable {
                key,
                stopped_at: entry.clone(),
                basis: basis.clone(),
            })
        }
        ApplicabilityOutcome::ApplicabilityFailed { cause, .. } => {
            Some(RuleOccurrenceResult::EvaluationFailed {
                key,
                cause: cause.clone(),
            })
        }
        ApplicabilityOutcome::NotEvaluated { .. } => Some(RuleOccurrenceResult::Cancelled { key }),
    }
}
