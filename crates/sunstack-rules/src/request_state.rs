use crate::{input_binding::AdmittedSources, *};
use serde::Serialize;
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentQuestion, SharedEvidenceState, StructuredEntry};
use sunstack_evidence::{
    CaptureCoverage, EvidenceDeclaration, EvidenceRequirement, EvidenceSelection, EvidenceSession,
    Omission, OmissionReason, SelectionContent, SelectionReadiness, SelectionResult, SourceName,
    SourceSpan,
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
    sources: &AdmittedSources,
    group: usize,
    session: &mut EvidenceSession,
) -> PreparedRuleEvidence {
    let selections = select_owner_evidence(
        &format!("rule:{}", rule.rule_id.as_str()),
        &rule.evidence,
        sources,
        group,
        session,
    )
    .await;
    let mut refused = DiagnosticList::default();
    for selection in &selections {
        if selection.declaration.requirement == EvidenceRequirement::Required {
            if let SelectionReadiness::Refused { diagnostics } = &selection.result.readiness {
                refused.extend(diagnostics.clone().into())
            }
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
    let mut selections = Vec::new();
    for declaration in declarations {
        let mut mapped = declaration.clone();
        let result = if let Some(source) = sources.get(&declaration.input) {
            mapped.input = source.name.clone();
            let mut result = session.select(&mapped).await;
            if let SelectionReadiness::Ready {
                content, coverage, ..
            } = &mut result.readiness
            {
                if let SelectionContent::PayloadValue { input, .. } = content {
                    *input = declaration.input.clone();
                }
                if let sunstack_evidence::SourceRef::Payload {
                    group_position,
                    input,
                } = &mut coverage.source
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
fn resolve_target(selection: &LabeledSelection) -> Option<sunstack_evidence::TargetLocation> {
    let SelectionReadiness::Ready { content, .. } = &selection.result.readiness else {
        return None;
    };
    match (content, &selection.declaration.select) {
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
        _ => None,
    }
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
    coverage: CaptureCoverage,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AttributedEvidence {
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
    OptionalUnavailable {
        input: SourceName,
        omissions: Vec<Omission>,
    },
}
#[derive(Clone, Serialize)]
#[serde(untagged)]
enum ContentValue {
    Text(String),
    Matches(Vec<sunstack_evidence::EvidenceMatch>),
    Records(Vec<sunstack_evidence::ConversationRecord>),
    Payload(sunstack_evidence::StructuredJson),
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
