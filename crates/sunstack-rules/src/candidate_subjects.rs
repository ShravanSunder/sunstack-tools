use crate::{DiagnosticList, input_binding::*, *};
use std::sync::Arc;
use sunstack_evidence::*;
pub(crate) struct CandidateMaterial {
    pub candidate_id: CandidateId,
    pub entry_position: usize,
    pub source_name: SourceName,
    pub rule: RuleRef,
    pub occurrence: SourceOccurrence,
    pub view: PrivacyView,
    pub selected: CandidateSelection,
}
pub(crate) enum CandidateSelection {
    Record {
        selected: Arc<CandidateRecordView>,
    },
    Code {
        context: Arc<CandidateContextView>,
        grants: std::collections::BTreeSet<PrivateKind>,
    },
    LeftOutForPrivacy,
}
struct SubjectSource {
    id: SourceId,
    spec: SourceSpec,
    binding: InputBinding,
    view: CandidateSourceView,
}
impl CandidateMaterial {
    fn subject_source(&self) -> Result<SubjectSource, DiagnosticList> {
        match &self.selected {
            CandidateSelection::Record { selected } => {
                let SourceRef::Transcript { path, format, .. } = &selected.coverage.source else {
                    return Err(crate::candidate_inputs::invalid_candidate_field(vec![
                        "sources".into(),
                    ]));
                };
                Ok(SubjectSource {
                    id: SourceId::Conversation {
                        path: path.clone(),
                        format: format.clone(),
                        supplement_identity: None,
                    },
                    spec: SourceSpec::Conversation {
                        transcript_path: path.clone(),
                        format: format.clone(),
                        final_assistant_text: None,
                    },
                    binding: InputBinding::Conversation {
                        transcript_path: path.clone(),
                        format: format.clone(),
                        final_assistant_text: None,
                    },
                    view: CandidateSourceView::Record(selected.clone()),
                })
            }
            CandidateSelection::Code { context, grants } => Ok(SubjectSource {
                id: SourceId::Code {
                    input_id: context.context.input_id.clone(),
                },
                spec: SourceSpec::ChangeSet {
                    repository: context.context.repository.clone(),
                },
                binding: InputBinding::ChangeSet {
                    repository: context.context.repository.clone(),
                },
                view: CandidateSourceView::Code {
                    context: context.clone(),
                    grants: grants.clone(),
                },
            }),
            CandidateSelection::LeftOutForPrivacy => {
                Err(crate::candidate_inputs::invalid_candidate_field(vec![
                    "sources".into(),
                ]))
            }
        }
    }
    pub fn subject_binding(&self) -> Result<InputBinding, DiagnosticList> {
        Ok(self.subject_source()?.binding)
    }
    pub fn admitted_subject(&self) -> Result<AdmittedSource, DiagnosticList> {
        let source = self.subject_source()?;
        Ok(AdmittedSource {
            id: source.id,
            name: self.source_name.clone(),
            spec: source.spec,
            candidate: Some(source.view),
        })
    }
    pub fn context(&self) -> Option<CandidateContextView> {
        match &self.selected {
            CandidateSelection::Code { context, .. } => Some(context.as_ref().clone()),
            _ => None,
        }
    }
}
pub(crate) fn code_selection(
    context: CandidateContext,
    grants: &std::collections::BTreeSet<PrivateKind>,
    settings: &ContextSettings,
    redactions: &[CodeRedactionRange],
) -> Result<CandidateSelection, DiagnosticList> {
    let projection = crate::context_pages::project_context_pages(
        &context,
        settings,
        redactions,
        crate::request_state::rendered_code_page_chars,
    )?;
    Ok(CandidateSelection::Code {
        context: Arc::new(CandidateContextView {
            context,
            labels: projection.labels,
            pages: projection.pages,
            coverage: projection.coverage,
        }),
        grants: grants.clone(),
    })
}
