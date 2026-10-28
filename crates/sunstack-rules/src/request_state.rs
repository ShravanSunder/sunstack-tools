use crate::{input_binding::AdmittedSources, *};
use serde::Serialize;
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentQuestion, SharedEvidenceState, StructuredEntry};
use sunstack_evidence::{
    EvidenceDeclaration, EvidenceRequirement, EvidenceSelection, EvidenceSession, Omission,
    OmissionReason, SelectionContent, SelectionCoverage, SelectionReadiness, SelectionResult,
    SourceName, SourceSpan,
};
#[derive(Debug, Clone)]
pub(crate) struct LabeledSelection {
    pub label: String,
    pub declaration: EvidenceDeclaration,
    pub result: SelectionResult,
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
            let mut result = if let Some(candidate) = &source.candidate {
                candidate_record_selection(candidate, declaration)
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
        });
    }
    selections
}

fn candidate_record_selection(
    candidate: &crate::input_binding::CandidateRecordView,
    declaration: &EvidenceDeclaration,
) -> SelectionResult {
    let readiness = SelectionReadiness::Ready {
        content: SelectionContent::PositionedRecord {
            source: candidate.coverage.source.clone(),
            record: Box::new(candidate.record.clone()),
        },
        omissions: vec![],
        coverage: Box::new(SelectionCoverage::SingleSource {
            capture: candidate.coverage.clone(),
        }),
    };
    SelectionResult {
        id: declaration.id.clone(),
        readiness,
    }
}
pub(crate) fn resolve_target(
    selection: &LabeledSelection,
) -> Option<sunstack_evidence::TargetLocation> {
    let SelectionReadiness::Ready { content, .. } = &selection.result.readiness else {
        return None;
    };
    match (content, &selection.declaration.select) {
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
#[derive(Serialize)]
struct RuleRequestState {
    state_version: u32,
    evidence: BTreeMap<String, AttributedEvidence>,
    contents: BTreeMap<String, ContentValue>,
}
#[derive(Serialize)]
struct EvidenceAttribution {
    input: SourceName,
    content: String,
    omissions: Vec<Omission>,
    coverage: SelectionCoverage,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AttributedEvidence {
    PositionedRecord {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
    FrontmatterField {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
        result: Box<sunstack_evidence::FrontmatterFieldContent>,
    },
    MarkdownUnits {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
        units: Vec<MarkdownUnitAttribution>,
    },
    Paragraph {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
        span: SourceSpan,
    },
    DocumentText {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
        span: SourceSpan,
    },
    Matches {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
    ConversationRecords {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
    PayloadField {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
        pointer: String,
    },
    ChangedUnits {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
    RelatedGroup {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
    OptionalUnavailable {
        input: SourceName,
        omissions: Vec<Omission>,
    },
}
#[derive(Clone, Serialize)]
#[serde(untagged)]
enum ContentValue {
    PositionedRecord(Box<sunstack_evidence::PositionedRecord>),
    FrontmatterField(FrontmatterValue),
    MarkdownUnits(Vec<MarkdownUnitText>),
    Text(String),
    Matches(Vec<sunstack_evidence::EvidenceMatch>),
    Records(Vec<sunstack_evidence::ConversationRecord>),
    Payload(sunstack_evidence::StructuredJson),
    ChangedUnits(Vec<sunstack_evidence::SelectedSourceUnit>),
    RelatedGroup(Box<sunstack_evidence::ContextGroupOutcome>),
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FrontmatterValue {
    Present {
        value: sunstack_evidence::StructuredJson,
    },
    Absent,
    BlockUnavailable {
        reason: sunstack_evidence::FrontmatterUnavailableReason,
    },
}

#[derive(Clone, Serialize)]
struct MarkdownUnitText {
    text: String,
    visible_text: String,
}

#[derive(Clone, Serialize)]
struct MarkdownUnitAttribution {
    kind: sunstack_evidence::MarkdownUnitKind,
    span: SourceSpan,
    occurrence: sunstack_evidence::MarkdownOccurrence,
}
pub(crate) fn build_request_state(
    selections: impl Iterator<Item = LabeledSelection>,
    omit_optional: bool,
) -> Result<SharedEvidenceState, DiagnosticList> {
    let mut state = RuleRequestState {
        state_version: 1,
        evidence: BTreeMap::new(),
        contents: BTreeMap::new(),
    };
    let mut identities = BTreeMap::new();
    for selection in selections {
        if state.evidence.contains_key(&selection.label) {
            continue;
        }
        let optional = selection.declaration.requirement == EvidenceRequirement::Optional;
        let entry = match selection.result.readiness {
            SelectionReadiness::Ready {
                content,
                mut omissions,
                coverage,
            } if !(optional && omit_optional) => {
                let value = match &content {
                    SelectionContent::PositionedRecord { record, .. } => {
                        ContentValue::PositionedRecord(record.clone())
                    }
                    SelectionContent::FrontmatterField { result } => {
                        ContentValue::FrontmatterField(match result.as_ref() {
                            sunstack_evidence::FrontmatterFieldContent::Present {
                                value, ..
                            } => FrontmatterValue::Present {
                                value: value.clone(),
                            },
                            sunstack_evidence::FrontmatterFieldContent::Absent { .. } => {
                                FrontmatterValue::Absent
                            }
                            sunstack_evidence::FrontmatterFieldContent::BlockUnavailable {
                                reason,
                                ..
                            } => FrontmatterValue::BlockUnavailable { reason: *reason },
                        })
                    }
                    SelectionContent::MarkdownUnits { units } => ContentValue::MarkdownUnits(
                        units
                            .iter()
                            .map(|unit| MarkdownUnitText {
                                text: unit.text.clone(),
                                visible_text: unit.visible_text.clone(),
                            })
                            .collect(),
                    ),
                    SelectionContent::Paragraph { text, .. }
                    | SelectionContent::DocumentText { text, .. } => {
                        ContentValue::Text(text.clone())
                    }
                    SelectionContent::Matches { matches } => ContentValue::Matches(matches.clone()),
                    SelectionContent::ConversationRecords { records, .. } => {
                        ContentValue::Records(records.clone())
                    }
                    SelectionContent::PayloadValue { value, .. } => {
                        ContentValue::Payload(value.clone())
                    }
                    SelectionContent::ChangedUnits { units } => {
                        ContentValue::ChangedUnits(units.clone())
                    }
                    SelectionContent::RelatedGroup { group } => {
                        ContentValue::RelatedGroup(group.clone())
                    }
                    SelectionContent::RelatedGroups { .. } => {
                        return Err(sunstack_evidence::DiagnosticList::one(
                            sunstack_evidence::EvidenceCode::InvalidSelector,
                        )
                        .into());
                    }
                };
                let bytes = serde_json::to_vec(&value).map_err(|_| {
                    DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError)
                })?;
                let identity = sunstack_evidence::content_digest(&bytes);
                let content_id = identities
                    .entry(identity)
                    .or_insert_with(|| format!("c{}", state.contents.len().saturating_add(1)))
                    .clone();
                state.contents.entry(content_id.clone()).or_insert(value);
                let attribution = EvidenceAttribution {
                    input: selection.declaration.input.clone(),
                    content: content_id,
                    omissions: std::mem::take(&mut omissions),
                    coverage: *coverage,
                };
                match content {
                    SelectionContent::PositionedRecord { .. } => {
                        AttributedEvidence::PositionedRecord { attribution }
                    }
                    SelectionContent::FrontmatterField { result } => {
                        AttributedEvidence::FrontmatterField {
                            attribution,
                            result,
                        }
                    }
                    SelectionContent::MarkdownUnits { units } => {
                        AttributedEvidence::MarkdownUnits {
                            attribution,
                            units: units
                                .into_iter()
                                .map(|unit| MarkdownUnitAttribution {
                                    kind: unit.kind,
                                    span: unit.span,
                                    occurrence: unit.occurrence,
                                })
                                .collect(),
                        }
                    }
                    SelectionContent::Paragraph { span, .. } => {
                        AttributedEvidence::Paragraph { attribution, span }
                    }
                    SelectionContent::DocumentText { span, .. } => {
                        AttributedEvidence::DocumentText { attribution, span }
                    }
                    SelectionContent::Matches { .. } => AttributedEvidence::Matches { attribution },
                    SelectionContent::ConversationRecords { .. } => {
                        AttributedEvidence::ConversationRecords { attribution }
                    }
                    SelectionContent::PayloadValue { pointer, .. } => {
                        AttributedEvidence::PayloadField {
                            attribution,
                            pointer,
                        }
                    }
                    SelectionContent::ChangedUnits { .. } => {
                        AttributedEvidence::ChangedUnits { attribution }
                    }
                    SelectionContent::RelatedGroup { .. } => {
                        AttributedEvidence::RelatedGroup { attribution }
                    }
                    SelectionContent::RelatedGroups { .. } => {
                        return Err(sunstack_evidence::DiagnosticList::one(
                            sunstack_evidence::EvidenceCode::InvalidSelector,
                        )
                        .into());
                    }
                }
            }
            SelectionReadiness::Ready { .. } | SelectionReadiness::Refused { .. } if optional => {
                AttributedEvidence::OptionalUnavailable {
                    input: selection.declaration.input,
                    omissions: vec![Omission {
                        reason: if omit_optional {
                            OmissionReason::Limit
                        } else {
                            OmissionReason::OptionalUnavailable
                        },
                        count: 1,
                    }],
                }
            }
            SelectionReadiness::Refused { diagnostics } => return Err(diagnostics.into()),
            SelectionReadiness::Ready { .. } => {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::UnexpectedExecutionError,
                ));
            }
        };
        state.evidence.insert(selection.label, entry);
    }
    let bytes = serde_json::to_vec(&state)
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))
}
pub(crate) fn wrap_question(
    question: &JudgmentQuestion,
    owner: &str,
    reads: Vec<String>,
    common: Option<&StructuredEntry>,
) -> Result<JudgmentQuestion, DiagnosticList> {
    #[derive(Serialize)]
    struct Instructions<'content> {
        owner: &'content str,
        reads: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        common_ruleset_instructions: Option<&'content StructuredEntry>,
        question: &'content StructuredEntry,
    }
    let mut question = question.clone();
    let instructions = match &mut question {
        JudgmentQuestion::Noul { instructions, .. }
        | JudgmentQuestion::Choice { instructions, .. }
        | JudgmentQuestion::Score { instructions, .. } => instructions,
    };
    let bytes = serde_json::to_vec(&Instructions {
        owner,
        reads,
        common_ruleset_instructions: common.filter(|value| !matches!(value, StructuredEntry::Null)),
        question: instructions,
    })
    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    *instructions = serde_json::from_slice(&bytes)
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    Ok(question)
}
