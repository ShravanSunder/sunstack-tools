use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, PathBuf},
};
use sunstack_evidence::{
    ProviderName, RepositoryId, SessionId, SourceInput, SourceInputId, SourceOccurrence, SourceRef,
};

/// Derived only in process; no case or manifest can supply its split group.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SourceGroup {
    Session {
        provider: ProviderName,
        session: SessionId,
    },
    Code {
        repository: RepositoryId,
        path: PathBuf,
    },
    Diff {
        input: SourceInputId,
    },
    Custom {
        location: PathBuf,
    },
}

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
    let mut groups = BTreeMap::new();
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
        let group = source_group(source, &case.source_anchor)?;
        if let Some(prior) = groups.insert(group, case.split)
            && prior != case.split
        {
            return Err(EvalError::SplitLeakage {
                field: "group".into(),
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
    }
    Ok(())
}

fn source_group(source: &CaseSource, anchor: &SourceOccurrence) -> Result<SourceGroup, EvalError> {
    match &source.source {
        SourceInput::SessionPair { provider, session } => {
            require_record_anchor(anchor)?;
            Ok(SourceGroup::Session {
                provider: provider.clone(),
                session: session.clone(),
            })
        }
        SourceInput::Code { repository, .. } => {
            let path = repository_document_path(source, anchor, repository)?;
            Ok(SourceGroup::Code {
                repository: repository.clone(),
                path,
            })
        }
        SourceInput::Diff { repository, .. } => {
            repository_document_path(source, anchor, repository)?;
            // SourceInput owns this identity: a diff contains only its repository and revision scope.
            let input = source
                .source
                .source_identity()
                .map_err(|_| invalid("sources.source"))?;
            Ok(SourceGroup::Diff { input })
        }
        SourceInput::CustomEntry { location, .. } => {
            require_record_anchor(anchor)?;
            Ok(SourceGroup::Custom {
                location: location.clone(),
            })
        }
    }
}
fn require_record_anchor(anchor: &SourceOccurrence) -> Result<(), EvalError> {
    match anchor {
        SourceOccurrence::Record { .. } => Ok(()),
        SourceOccurrence::CodeMatch { .. }
        | SourceOccurrence::ContextMatch { .. }
        | SourceOccurrence::DocumentUnit { .. }
        | SourceOccurrence::DocumentField { .. } => Err(invalid("labels.source_anchor")),
    }
}
fn repository_document_path(
    source: &CaseSource,
    anchor: &SourceOccurrence,
    repository: &RepositoryId,
) -> Result<PathBuf, EvalError> {
    let (input_id, anchor_source) = match anchor {
        SourceOccurrence::CodeMatch { input_id, span, .. }
        | SourceOccurrence::ContextMatch { input_id, span, .. } => (input_id, &span.source),
        SourceOccurrence::DocumentUnit {
            input_id,
            occurrence,
            ..
        } => {
            let span = match occurrence {
                sunstack_evidence::MarkdownOccurrence::Unit { span } => span,
                sunstack_evidence::MarkdownOccurrence::CallSite { unit_span, .. } => unit_span,
            };
            (input_id, &span.source)
        }
        SourceOccurrence::DocumentField {
            input_id, source, ..
        } => (input_id, source),
        SourceOccurrence::Record { .. } => return Err(invalid("labels.source_anchor")),
    };
    let expected = source
        .source
        .source_identity()
        .map_err(|_| invalid("sources.source"))?;
    if input_id != &expected {
        return Err(invalid("labels.source_anchor.input_id"));
    }
    let SourceRef::RepositoryFile {
        repository: anchor_repository,
        path,
        ..
    } = anchor_source
    else {
        return Err(invalid("labels.source_anchor.source"));
    };
    if anchor_repository != repository {
        return Err(invalid("labels.source_anchor.repository"));
    }
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(invalid("labels.source_anchor.path"));
    }
    Ok(path.clone())
}
pub(crate) fn invalid(field: &str) -> EvalError {
    EvalError::Invalid {
        field: field.into(),
    }
}
