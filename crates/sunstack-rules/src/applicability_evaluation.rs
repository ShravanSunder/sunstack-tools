use crate::{input_binding::AdmittedSources, *};
use sunstack_evidence::{
    AssistantSelection, ConversationBody, DeclarationId, EvidenceDeclaration, EvidenceRequirement,
    EvidenceSelection, EvidenceSession, Inclusion, MessageBlock, SelectionContent,
    SelectionReadiness, SourceName, SourceSpec, ToolFilter, ToolResult, ToolUseSelection,
    TurnWindow,
};
pub(crate) struct ConditionContext<'invocation> {
    pub sources: &'invocation AdmittedSources,
    pub origin: &'invocation InvocationOrigin,
    pub session: &'invocation mut EvidenceSession,
}
fn unresolved() -> ConditionOutcome {
    ConditionOutcome::Unresolved {
        diagnostic: ReportDiagnostic::Rule(Box::new(RuleDiagnostic::new(
            RuleDiagnosticCode::ConditionUnresolved,
        ))),
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
    mut context: ConditionContext<'_>,
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
        } => settle_payload_condition(
            payload_field(&mut context, input, pointer).await,
            |observed| observed == value,
        ),
        ConditionTest::PayloadFieldIn {
            input,
            pointer,
            values,
        } => settle_payload_condition(
            payload_field(&mut context, input, pointer).await,
            |observed| values.contains(observed),
        ),
        ConditionTest::PayloadFieldPresent { input, pointer } => {
            settle_payload_condition(payload_field(&mut context, input, pointer).await, |_| true)
        }
        ConditionTest::PatternPresent {
            input,
            pattern,
            max_scan_bytes,
        } => {
            let Some(source) = context.sources.get(input) else {
                return unresolved();
            };
            if let Some(candidate) = source.candidate_record() {
                let Some(text) = positioned_condition_text(&candidate.record) else {
                    return unresolved();
                };
                if text.len() > max_scan_bytes.get() {
                    return unresolved();
                }
                let pattern = pattern.clone();
                return match tokio::task::spawn_blocking(move || pattern_present(&text, &pattern))
                    .await
                {
                    Ok(Ok(present)) => comparison(present),
                    _ => unresolved(),
                };
            }
            if let Some(code) = source.candidate_code() {
                let text = std::iter::once(code.matched.text.as_str())
                    .chain(code.units.iter().map(|unit| unit.text.as_str()))
                    .chain(code.imports.iter().map(|import| import.text.as_str()))
                    .chain(
                        code.changes
                            .iter()
                            .flat_map(|hunk| [hunk.before.as_ref(), hunk.after.as_ref()])
                            .flatten()
                            .map(|excerpt| excerpt.text.as_str()),
                    )
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.len() > max_scan_bytes.get() {
                    return unresolved();
                }
                let pattern = pattern.clone();
                return match tokio::task::spawn_blocking(move || pattern_present(&text, &pattern))
                    .await
                {
                    Ok(Ok(present)) => comparison(present),
                    _ => unresolved(),
                };
            }
            let select = match source.spec {
                SourceSpec::Document { .. } => EvidenceSelection::WholeDocument {},
                SourceSpec::Conversation { .. } => EvidenceSelection::Conversation {
                    turns: TurnWindow::All {},
                    user_messages: Inclusion::Included,
                    assistant_messages: AssistantSelection::All,
                    tool_use: ToolUseSelection::Included {
                        tools: ToolFilter::All {},
                        results: Inclusion::Included,
                    },
                    hook_injected: Inclusion::Included,
                },
                _ => return unresolved(),
            };
            let Ok(id) = DeclarationId::new("condition_scan") else {
                return unresolved();
            };
            let declaration = EvidenceDeclaration {
                id,
                input: source.name.clone(),
                requirement: EvidenceRequirement::Required,
                select,
                max_items: None,
                max_bytes: None,
            };
            let result = if let Some(grants) = source.private_access() {
                context.session.select_projected(&declaration, grants).await
            } else {
                context.session.select(&declaration).await
            };
            let text = match result.readiness {
                SelectionReadiness::Ready {
                    content: SelectionContent::DocumentText { text, .. },
                    ..
                } => text,
                SelectionReadiness::Ready {
                    content: SelectionContent::ConversationRecords { records, .. },
                    ..
                } => normalized_text(&records),
                SelectionReadiness::Refused { diagnostics }
                    if source.private_access().is_some() =>
                {
                    return evidence_unresolved(
                        diagnostics.items.into_iter().next().unwrap_or_else(|| {
                            sunstack_evidence::Diagnostic::new(
                                sunstack_evidence::EvidenceCode::InvalidSelector,
                            )
                        }),
                    );
                }
                _ => return unresolved(),
            };
            if text.len() > max_scan_bytes.get() {
                return if source.private_access().is_some() {
                    evidence_unresolved(sunstack_evidence::Diagnostic::new(
                        sunstack_evidence::EvidenceCode::SelectorLimit,
                    ))
                } else {
                    unresolved()
                };
            }
            let pattern = pattern.clone();
            match tokio::task::spawn_blocking(move || pattern_present(&text, &pattern)).await {
                Ok(Ok(present)) => comparison(present),
                _ => unresolved(),
            }
        }
    }
}
fn positioned_condition_text(record: &sunstack_evidence::PositionedRecord) -> Option<String> {
    use sunstack_evidence::PositionedBody;
    match &record.body {
        PositionedBody::Message { blocks } => {
            let mut text = String::new();
            for block in blocks {
                match &block.block {
                    MessageBlock::Text { text: part } | MessageBlock::Thinking { text: part } => {
                        text.push_str(part);
                        text.push('\n');
                    }
                    MessageBlock::Unsupported { content, .. } => {
                        text.push_str(&serde_json::to_string(content).ok()?);
                        text.push('\n');
                    }
                    MessageBlock::ToolCall { .. } => {}
                }
            }
            Some(text)
        }
        PositionedBody::Private { text, .. } => text.clone(),
        PositionedBody::ToolCall { tool, input, .. } => Some(format!(
            "{}\n{}",
            tool.as_str(),
            serde_json::to_string(input).ok()?
        )),
        PositionedBody::ToolResult { result, .. } => match result {
            ToolResult::FullObserved { text } | ToolResult::Clipped { text, .. } => {
                Some(text.clone())
            }
            ToolResult::Absent => None,
        },
        PositionedBody::Custom { fields, .. } => serde_json::to_string(fields).ok(),
    }
}
async fn payload_field<'value>(
    context: &mut ConditionContext<'value>,
    input: &SourceName,
    pointer: &str,
) -> Result<
    Option<std::borrow::Cow<'value, sunstack_evidence::StructuredJson>>,
    Box<sunstack_evidence::Diagnostic>,
> {
    let sources: &'value AdmittedSources = context.sources;
    let Some(source) = sources.get(input) else {
        return Ok(None);
    };
    if let Some(grants) = source.private_access() {
        let declaration = EvidenceDeclaration {
            id: DeclarationId::new("condition_payload")
                .map_err(|error| Box::new(error.diagnostic()))?,
            input: source.name.clone(),
            requirement: EvidenceRequirement::Required,
            select: EvidenceSelection::PayloadField {
                pointer: pointer.into(),
            },
            max_items: None,
            max_bytes: None,
        };
        return match context
            .session
            .select_projected(&declaration, grants)
            .await
            .readiness
        {
            SelectionReadiness::Ready {
                content: SelectionContent::PayloadValue { value, .. },
                ..
            } => Ok(Some(std::borrow::Cow::Owned(value))),
            SelectionReadiness::Refused { diagnostics } => Err(Box::new(
                diagnostics.items.into_iter().next().unwrap_or_else(|| {
                    sunstack_evidence::Diagnostic::new(
                        sunstack_evidence::EvidenceCode::InvalidSelector,
                    )
                }),
            )),
            _ => Ok(None),
        };
    }
    Ok(match &source.spec {
        SourceSpec::Payload { value, .. } => value.pointer(pointer).map(std::borrow::Cow::Borrowed),
        _ => None,
    })
}
fn evidence_unresolved(
    diagnostic: impl Into<Box<sunstack_evidence::Diagnostic>>,
) -> ConditionOutcome {
    ConditionOutcome::Unresolved {
        diagnostic: ReportDiagnostic::Evidence(diagnostic.into()),
    }
}
fn settle_payload_condition(
    result: Result<
        Option<std::borrow::Cow<'_, sunstack_evidence::StructuredJson>>,
        Box<sunstack_evidence::Diagnostic>,
    >,
    test: impl FnOnce(&sunstack_evidence::StructuredJson) -> bool,
) -> ConditionOutcome {
    match result {
        Ok(Some(value)) => comparison(test(value.as_ref())),
        Ok(None) => unresolved(),
        Err(diagnostic) => evidence_unresolved(diagnostic),
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
pub(crate) fn stopped_occurrence(
    key: RuleOccurrenceKey,
    outcome: &ApplicabilityOutcome,
    selection: OccurrenceSelection,
) -> Option<RuleOccurrenceResult> {
    match outcome {
        ApplicabilityOutcome::Applicable => None,
        ApplicabilityOutcome::Uncertain { entry, answer } => {
            Some(RuleOccurrenceResult::Uncertain {
                key,
                at: entry.clone(),
                answer: answer.clone(),
            })
        }
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
                selection,
                cause: cause.clone(),
            })
        }
        ApplicabilityOutcome::NotEvaluated { .. } => {
            Some(RuleOccurrenceResult::Cancelled { key, selection })
        }
    }
}
