use crate::candidate_subject_selection::{candidate_code_selection, candidate_record_selection};
use crate::{input_binding::AdmittedSources, *};
use sunstack_evidence::{
    EvidenceDeclaration, EvidenceRequirement, EvidenceSelection, EvidenceSession, SelectionContent,
    SelectionCoverage, SelectionReadiness, SelectionResult,
};
#[derive(Debug, Clone)]
pub(crate) struct LabeledSelection {
    pub label: String,
    pub declaration: EvidenceDeclaration,
    pub result: SelectionResult,
    pub context: Option<std::sync::Arc<CandidateContextView>>,
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedRuleEvidence {
    pub selections: Vec<LabeledSelection>,
    pub target: Result<sunstack_evidence::TargetLocation, DiagnosticList>,
}
pub(crate) async fn select_rule_evidence(
    rule: &RuleDefinition,
    key: &RuleOccurrenceKey,
    target_selection: Option<&LabeledSelection>,
    sources: &AdmittedSources,
    group: usize,
    session: &mut EvidenceSession,
) -> PreparedRuleEvidence {
    let selections = select_owner_evidence_with_target(
        &key.evidence_label(),
        &rule.evidence,
        sources,
        group,
        session,
        target_selection,
    )
    .await;
    let mut refused = DiagnosticList::default();
    for selection in &selections {
        if selection.declaration.requirement == EvidenceRequirement::Required
            && let SelectionReadiness::Refused { diagnostics } = &selection.result.readiness
        {
            refused.extend(diagnostics.clone().into())
        }
    }
    let target = if refused.is_empty() {
        selections
            .iter()
            .find(|selection| selection.declaration.id == rule.target)
            .and_then(resolve_target)
            .ok_or_else(|| {
                DiagnosticList::from(sunstack_evidence::DiagnosticList::one(
                    sunstack_evidence::EvidenceCode::MissingTarget,
                ))
            })
    } else {
        Err(refused)
    };
    PreparedRuleEvidence { selections, target }
}
pub(crate) async fn select_owner_evidence(
    owner: &str,
    declarations: &[EvidenceDeclaration],
    sources: &AdmittedSources,
    group: usize,
    session: &mut EvidenceSession,
) -> Vec<LabeledSelection> {
    select_owner_evidence_with_target(owner, declarations, sources, group, session, None).await
}

async fn select_owner_evidence_with_target(
    owner: &str,
    declarations: &[EvidenceDeclaration],
    sources: &AdmittedSources,
    group: usize,
    session: &mut EvidenceSession,
    target_selection: Option<&LabeledSelection>,
) -> Vec<LabeledSelection> {
    let mut selections = Vec::new();
    for declaration in declarations {
        let mut mapped = declaration.clone();
        let result = if let Some(selected) =
            target_selection.filter(|selection| selection.declaration.id == declaration.id)
        {
            selected.result.clone()
        } else if let Some(source) = sources.get(&declaration.input) {
            mapped.input = source.name.clone();
            let mut result = if let Some(candidate) = source.candidate_record() {
                candidate_record_selection(candidate, declaration)
            } else if let Some(context) = source.candidate_code() {
                candidate_code_selection(context, declaration)
            } else if let Some(grants) = source.private_access() {
                session.select_projected(&mapped, grants).await
            } else {
                session.select(&mapped).await
            };
            if let SelectionReadiness::Ready {
                content, coverage, ..
            } = &mut result.readiness
            {
                if let SelectionContent::PayloadValue { input, .. } = content {
                    *input = declaration.input.clone();
                }
                if let SelectionCoverage::SingleSource { capture } = coverage.as_mut()
                    && let sunstack_evidence::SourceRef::Payload {
                        group_position,
                        input,
                    } = &mut capture.source
                {
                    *group_position = group;
                    *input = declaration.input.clone();
                }
            }
            result
        } else {
            SelectionResult {
                id: declaration.id.clone(),
                readiness: SelectionReadiness::Refused {
                    diagnostics: sunstack_evidence::DiagnosticList::one(
                        sunstack_evidence::EvidenceCode::InvalidSelector,
                    ),
                },
            }
        };
        selections.push(LabeledSelection {
            label: format!("{owner}/{}", declaration.id.as_str()),
            declaration: declaration.clone(),
            result,
            context: sources
                .get(&declaration.input)
                .and_then(|source| source.candidate_context())
                .cloned(),
        });
    }
    selections
}

pub(crate) fn resolve_target(
    selection: &LabeledSelection,
) -> Option<sunstack_evidence::TargetLocation> {
    let SelectionReadiness::Ready { content, .. } = &selection.result.readiness else {
        return None;
    };
    match (content, &selection.declaration.select) {
        (SelectionContent::CodeContext { context }, _) => {
            Some(sunstack_evidence::TargetLocation::CodeMatch {
                span: context.matched.span.clone(),
            })
        }
        (SelectionContent::PositionedRecord { source, record }, _) => {
            Some(sunstack_evidence::TargetLocation::PositionedRecord {
                source: source.clone(),
                record_id: record.record_id.clone(),
            })
        }
        (SelectionContent::MarkdownUnits { units }, _) if units.len() == 1 => {
            Some(sunstack_evidence::TargetLocation::MarkdownUnit {
                occurrence: Box::new(units[0].occurrence.clone()),
            })
        }
        (SelectionContent::FrontmatterField { result }, _) => {
            use sunstack_evidence::{
                FrontmatterFieldContent as Content, FrontmatterFieldLocation as Location,
            };
            let (source, field, state) = match result.as_ref() {
                Content::Present {
                    source,
                    field,
                    span,
                    ..
                } => (source, field, Location::Present { span: span.clone() }),
                Content::Absent {
                    source,
                    field,
                    frontmatter_span,
                } => (
                    source,
                    field,
                    Location::Absent {
                        frontmatter_span: frontmatter_span.clone(),
                    },
                ),
                Content::BlockUnavailable {
                    source,
                    field,
                    reason,
                    ..
                } => (
                    source,
                    field,
                    Location::BlockUnavailable { reason: *reason },
                ),
            };
            Some(sunstack_evidence::TargetLocation::FrontmatterField {
                source: source.clone(),
                field: field.clone(),
                state: Box::new(state),
            })
        }
        (SelectionContent::Paragraph { span, .. }, EvidenceSelection::Paragraph { target_id }) => {
            Some(sunstack_evidence::TargetLocation::Paragraph {
                span: span.clone(),
                target_id: target_id.clone(),
            })
        }
        (SelectionContent::ConversationRecords { source, records }, _) => {
            Some(sunstack_evidence::TargetLocation::ConversationRecords {
                source: source.clone(),
                record_ids: records
                    .iter()
                    .map(|record| record.record_id.clone())
                    .collect(),
            })
        }
        (SelectionContent::PayloadValue { input, pointer, .. }, _) => {
            Some(sunstack_evidence::TargetLocation::PayloadField {
                input: input.clone(),
                pointer: pointer.clone(),
            })
        }
        (SelectionContent::ChangedUnits { units }, _) => {
            code_target(units.iter().map(|unit| &unit.unit.key))
        }
        (SelectionContent::RelatedGroup { group }, _) => match group.as_ref() {
            sunstack_evidence::ContextGroupOutcome::Complete { group, .. }
            | sunstack_evidence::ContextGroupOutcome::Partial { group, .. } => {
                code_target(group.units.iter())
            }
            sunstack_evidence::ContextGroupOutcome::Unavailable { .. } => None,
        },
        _ => None,
    }
}
fn code_target<'source>(
    units: impl Iterator<Item = &'source sunstack_evidence::SourceUnitKey>,
) -> Option<sunstack_evidence::TargetLocation> {
    let units = units
        .filter(|unit| {
            matches!(
                unit.revision,
                sunstack_evidence::SourceRevision::Commit {
                    side: sunstack_evidence::SourceSide::After,
                    ..
                } | sunstack_evidence::SourceRevision::Index {
                    side: sunstack_evidence::SourceSide::After,
                    ..
                } | sunstack_evidence::SourceRevision::Worktree {
                    side: sunstack_evidence::SourceSide::After,
                    ..
                }
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    (!units.is_empty()).then_some(sunstack_evidence::TargetLocation::CodeUnits { units })
}

pub(crate) use crate::request_state_rendering::{
    build_request_state, rendered_code_page_chars, wrap_question,
};
