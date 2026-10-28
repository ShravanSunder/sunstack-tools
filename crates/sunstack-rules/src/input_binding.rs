use crate::{definition_contracts::*, invocation_contracts::*, rule_diagnostics::*};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use sunstack_evidence::{
    RepositoryBinding, RepositoryId, SourceName, SourceSpec, TranscriptFormatName,
};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceId {
    Code {
        input_id: sunstack_evidence::SourceInputId,
    },
    Document {
        repository: RepositoryId,
        path: PathBuf,
    },
    Conversation {
        path: PathBuf,
        format: TranscriptFormatName,
        supplement_identity: Option<String>,
    },
    Payload {
        canonical_value_identity: String,
    },
    ChangeSet {
        repository: RepositoryId,
    },
}
#[derive(Debug, Clone)]
pub struct AdmittedSource {
    pub id: SourceId,
    pub name: SourceName,
    pub spec: SourceSpec,
    pub(crate) candidate: Option<CandidateSourceView>,
}
#[derive(Debug, Clone)]
pub(crate) enum CandidateSourceView {
    Record(std::sync::Arc<CandidateRecordView>),
    Document {
        selected: std::sync::Arc<sunstack_evidence::DiscoveredDocumentMatch>,
        coverage: sunstack_evidence::DiscoveryCoverage,
        grants: std::collections::BTreeSet<sunstack_evidence::PrivateKind>,
    },
    Code {
        context: std::sync::Arc<crate::CandidateContextView>,
        grants: std::collections::BTreeSet<sunstack_evidence::PrivateKind>,
    },
    Outside {
        grants: std::collections::BTreeSet<sunstack_evidence::PrivateKind>,
    },
}
impl AdmittedSource {
    pub(crate) fn candidate_document(
        &self,
    ) -> Option<(
        &sunstack_evidence::DiscoveredDocumentMatch,
        &sunstack_evidence::DiscoveryCoverage,
    )> {
        match &self.candidate {
            Some(CandidateSourceView::Document {
                selected, coverage, ..
            }) => Some((selected, coverage)),
            _ => None,
        }
    }

    pub(crate) fn candidate_record(&self) -> Option<&CandidateRecordView> {
        match &self.candidate {
            Some(CandidateSourceView::Record(record)) => Some(record),
            _ => None,
        }
    }
    pub(crate) fn candidate_code(&self) -> Option<&sunstack_evidence::CandidateContext> {
        match &self.candidate {
            Some(CandidateSourceView::Code { context, .. }) => Some(&context.context),
            _ => None,
        }
    }
    pub(crate) fn candidate_context(&self) -> Option<&std::sync::Arc<crate::CandidateContextView>> {
        match &self.candidate {
            Some(CandidateSourceView::Code { context, .. }) => Some(context),
            _ => None,
        }
    }
    pub(crate) fn private_access(
        &self,
    ) -> Option<&std::collections::BTreeSet<sunstack_evidence::PrivateKind>> {
        match &self.candidate {
            Some(
                CandidateSourceView::Outside { grants }
                | CandidateSourceView::Code { grants, .. }
                | CandidateSourceView::Document { grants, .. },
            ) => Some(grants),
            _ => None,
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct CandidateRecordView {
    pub record: sunstack_evidence::PositionedRecord,
    pub coverage: sunstack_evidence::CaptureCoverage,
}
pub(crate) type AdmittedSources = BTreeMap<SourceName, AdmittedSource>;
pub(crate) fn normalize_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            other => normalized.push(other),
        }
    }
    Some(normalized)
}
pub(crate) fn resolved_path(base: &Path, path: &Path) -> Result<PathBuf, DiagnosticList> {
    let joined = base.join(path);
    normalize_path(&joined)
        .map(|path| path.canonicalize().unwrap_or(path))
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))
}
pub(crate) fn admit_group(
    declarations: &BTreeMap<SourceName, InputDeclaration>,
    group: &BindingGroup,
    repositories: &[RepositoryBinding],
    all_sources: &mut BTreeMap<SourceName, SourceSpec>,
) -> Result<AdmittedSources, DiagnosticList> {
    if group
        .bindings
        .keys()
        .any(|name| !declarations.contains_key(name))
    {
        return Err(DiagnosticList::rule(RuleDiagnosticCode::ExtraBinding));
    }
    let mut admitted = BTreeMap::new();
    for (name, declaration) in declarations {
        let binding = group
            .bindings
            .get(name)
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnboundInput))?;
        let (spec, id) = match (declaration, binding) {
            (InputDeclaration::Document {}, InputBinding::Document { repository, path }) => {
                let path = normalize_path(path)
                    .filter(|path| !path.is_absolute() && !path.as_os_str().is_empty())
                    .ok_or_else(|| {
                        DiagnosticList::from(sunstack_evidence::DiagnosticList::one(
                            sunstack_evidence::EvidenceCode::PathOutsideRoot,
                        ))
                    })?;
                let binding = repositories
                    .iter()
                    .find(|binding| &binding.key == repository)
                    .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::BindingKindMismatch))?;
                if !sunstack_evidence::path_is_allowed(binding, &path)
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?
                {
                    return Err(sunstack_evidence::DiagnosticList::one(
                        sunstack_evidence::EvidenceCode::PathOutsideRoot,
                    )
                    .into());
                }
                (
                    SourceSpec::Document {
                        repository: repository.clone(),
                        path: path.clone(),
                    },
                    SourceId::Document {
                        repository: repository.clone(),
                        path: path.clone(),
                    },
                )
            }
            (
                InputDeclaration::Conversation {},
                InputBinding::Conversation {
                    transcript_path,
                    format,
                    final_assistant_text,
                },
            ) => {
                if transcript_path.as_os_str().is_empty() {
                    return Err(DiagnosticList::rule(
                        RuleDiagnosticCode::InvalidConfiguration,
                    ));
                }
                let cwd = std::env::current_dir()
                    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
                let path = resolved_path(&cwd, transcript_path)?;
                let id = SourceId::Conversation {
                    path: path.clone(),
                    format: format.clone(),
                    supplement_identity: final_assistant_text
                        .as_ref()
                        .map(|text| sunstack_evidence::content_digest(text.as_bytes())),
                };
                (
                    SourceSpec::Conversation {
                        transcript_path: path,
                        format: format.clone(),
                        final_assistant_text: final_assistant_text.clone(),
                    },
                    id,
                )
            }
            (InputDeclaration::Payload { schema }, InputBinding::Payload { value }) => {
                let bytes = serde_json::to_vec(value)
                    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
                (
                    SourceSpec::Payload {
                        schema: schema.clone(),
                        value: value.clone(),
                    },
                    SourceId::Payload {
                        canonical_value_identity: sunstack_evidence::content_digest(&bytes),
                    },
                )
            }
            (InputDeclaration::ChangeSet {}, InputBinding::ChangeSet { repository }) => (
                SourceSpec::ChangeSet {
                    repository: repository.clone(),
                },
                SourceId::ChangeSet {
                    repository: repository.clone(),
                },
            ),
            _ => {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::BindingKindMismatch,
                ));
            }
        };
        let source_name = SourceName::new(format!("s{:06}", all_sources.len() + 1))
            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        all_sources.insert(source_name.clone(), spec.clone());
        admitted.insert(
            name.clone(),
            AdmittedSource {
                id,
                name: source_name,
                spec,
                candidate: None,
            },
        );
    }
    Ok(admitted)
}
pub(crate) fn wire_group(
    target: &RouteTarget,
    group: &BindingGroup,
    declarations: &BTreeMap<SourceName, InputDeclaration>,
    router_inputs: &BTreeMap<SourceName, InputDeclaration>,
) -> Result<BindingGroup, DiagnosticList> {
    if target.inputs.len() != declarations.len() {
        return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring));
    }
    let mut bindings = BTreeMap::new();
    for (name, declaration) in declarations {
        let router_name = target
            .inputs
            .get(name)
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring))?;
        let router_declaration = router_inputs
            .get(router_name)
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring))?;
        let compatible = match (declaration, router_declaration) {
            (InputDeclaration::Document {}, InputDeclaration::Document {})
            | (InputDeclaration::Conversation {}, InputDeclaration::Conversation {})
            | (InputDeclaration::ChangeSet {}, InputDeclaration::ChangeSet {}) => true,
            (
                InputDeclaration::Payload { schema: first },
                InputDeclaration::Payload { schema: second },
            ) => first == second,
            _ => false,
        };
        if !compatible {
            return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring));
        }
        bindings.insert(
            name.clone(),
            group
                .bindings
                .get(router_name)
                .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnboundInput))?
                .clone(),
        );
    }
    Ok(BindingGroup { bindings })
}
