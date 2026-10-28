use sunstack_evidence::*;
pub(crate) fn candidate_record_selection(
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
pub(crate) fn candidate_code_selection(
    context: &CandidateContext,
    declaration: &EvidenceDeclaration,
) -> SelectionResult {
    SelectionResult {
        id: declaration.id.clone(),
        readiness: SelectionReadiness::Ready {
            content: SelectionContent::CodeContext {
                context: Box::new(context.clone()),
            },
            omissions: vec![],
            coverage: Box::new(SelectionCoverage::ChangeSet {
                repository: context.repository.clone(),
                sources: context
                    .units
                    .iter()
                    .map(|unit| unit.unit.span.source.clone())
                    .chain(std::iter::once(context.matched.span.source.clone()))
                    .collect(),
                discovery: context.coverage.clone(),
            }),
        },
    }
}

pub(crate) fn candidate_document_selection(
    candidate: &DiscoveredDocumentMatch,
    discovery: &DiscoveryCoverage,
    declaration: &EvidenceDeclaration,
) -> SelectionResult {
    if let SelectionContent::FrontmatterField { result } = &candidate.content
        && let FrontmatterFieldContent::BlockUnavailable { diagnostics, .. } = result.as_ref()
    {
        return SelectionResult {
            id: declaration.id.clone(),
            readiness: SelectionReadiness::Refused {
                diagnostics: diagnostics.clone(),
            },
        };
    }
    let repository = match &candidate.source {
        SourceRef::RepositoryFile { repository, .. } => repository.clone(),
        _ => {
            return SelectionResult {
                id: declaration.id.clone(),
                readiness: SelectionReadiness::Refused {
                    diagnostics: DiagnosticList::one(EvidenceCode::InvalidSelector),
                },
            };
        }
    };
    SelectionResult {
        id: declaration.id.clone(),
        readiness: SelectionReadiness::Ready {
            content: candidate.content.clone(),
            omissions: vec![],
            coverage: Box::new(SelectionCoverage::ChangeSet {
                repository,
                sources: vec![candidate.source.clone()],
                discovery: discovery.clone(),
            }),
        },
    }
}
