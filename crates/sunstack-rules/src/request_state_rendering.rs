use crate::{request_state::LabeledSelection, *};
use serde::Serialize;
use std::collections::BTreeMap;
use sunstack_decisions::{JudgmentQuestion, SharedEvidenceState, StructuredEntry};
use sunstack_evidence::{
    EvidenceRequirement, Omission, OmissionReason, SelectionContent, SelectionCoverage,
    SelectionReadiness, SourceName, SourceSpan,
};
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
    CodeContextPage {
        content: String,
    },
    CodeContext {
        #[serde(flatten)]
        attribution: EvidenceAttribution,
    },
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
    CodeContextPage { page: String },
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
fn code_page_content(page: &ContextPage) -> ContentValue {
    ContentValue::CodeContextPage {
        page: page.text.clone(),
    }
}
pub(crate) fn delivered_code_page(state: &SharedEvidenceState) -> Option<&str> {
    use sunstack_decisions::StructuredJson;
    let SharedEvidenceState::Object(state) = state else {
        return None;
    };
    let StructuredJson::Object(contents) = state.get("contents")? else {
        return None;
    };
    let StructuredJson::Object(content) = contents.get("code_context")? else {
        return None;
    };
    let StructuredJson::Text(page) = content.get("page")? else {
        return None;
    };
    Some(page)
}
// This is the same serialized payload build_request_state places on the wire;
// no excerpt-only estimate or second page rendering participates in packing.
pub(crate) fn rendered_code_page_chars(page: &ContextPage) -> Result<usize, DiagnosticList> {
    let rendered = serde_json::to_string(&code_page_content(page))
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    let mut empty = page.clone();
    empty.text.clear();
    let baseline = serde_json::to_string(&code_page_content(&empty))
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    rendered
        .chars()
        .count()
        .checked_sub(baseline.chars().count())
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))
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
        if !(optional && omit_optional)
            && let Some(context) = &selection.context
            && matches!(selection.result.readiness, SelectionReadiness::Ready { .. })
        {
            let [page] = context.pages.as_slice() else {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::InvalidConfiguration,
                ));
            };
            let content_id = "code_context".to_owned();
            state
                .contents
                .insert(content_id.clone(), code_page_content(page));
            state.evidence.insert(
                selection.label,
                AttributedEvidence::CodeContextPage {
                    content: content_id,
                },
            );
            continue;
        }
        let entry = match selection.result.readiness {
            SelectionReadiness::Ready {
                content,
                mut omissions,
                coverage,
            } if !(optional && omit_optional) => {
                let value = match &content {
                    SelectionContent::CodeContext { .. } => {
                        return Err(DiagnosticList::rule(
                            RuleDiagnosticCode::InvalidConfiguration,
                        ));
                    }
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
                    SelectionContent::CodeContext { .. } => {
                        AttributedEvidence::CodeContext { attribution }
                    }
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
