use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::SourceInput;

pub(crate) fn validate_cases(
    sources: &[CaseSource],
    labels: &[LabeledCase],
) -> Result<(), EvalError> {
    if sources.is_empty() || labels.is_empty() {
        return Err(invalid("cases"));
    }
    let mut source_ids = BTreeMap::new();
    for source in sources {
        if source_ids.insert(&source.source_id, source).is_some() {
            return Err(invalid("sources.source_id"));
        }
    }
    let mut case_ids = BTreeSet::new();
    let mut anchors = BTreeSet::new();
    let mut sessions = BTreeMap::new();
    let mut projects = BTreeMap::new();
    for case in labels {
        if !case_ids.insert(&case.case_id) {
            return Err(invalid("labels.case_id"));
        }
        let source = source_ids
            .get(&case.source)
            .ok_or_else(|| invalid("labels.source"))?;
        let anchor = case
            .source_anchor
            .occurrence_identity()
            .map_err(|_| invalid("labels.source_anchor"))?;
        let question =
            serde_json::to_string(&case.question).map_err(|_| invalid("labels.question"))?;
        if !anchors.insert((&case.source, anchor, question)) {
            return Err(invalid("labels.source_anchor"));
        }
        if let Some(prior) = sessions.insert(&case.session, case.split)
            && prior != case.split
        {
            return Err(EvalError::SplitLeakage {
                field: "session".into(),
            });
        }
        if let Some(project) = &case.project
            && let Some(prior) = projects.insert(project, case.split)
            && prior != case.split
        {
            return Err(EvalError::SplitLeakage {
                field: "project".into(),
            });
        }
        match &source.source {
            SourceInput::SessionPair { provider, session } => {
                if provider != &case.session.provider || session != &case.session.session {
                    return Err(invalid("labels.session"));
                }
            }
            SourceInput::CustomEntry { .. } => {}
        }
    }
    Ok(())
}
pub(crate) fn invalid(field: &str) -> EvalError {
    EvalError::Invalid {
        field: field.into(),
    }
}
