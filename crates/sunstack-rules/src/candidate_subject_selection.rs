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
