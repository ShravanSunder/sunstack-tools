use crate::{input_binding::*, invocation_admission::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use sunstack_evidence::{
    EvidencePlanInput, EvidenceSession, PayloadSchemaPath, SourceName, SourceSpec,
};

pub(crate) struct CandidateAdmission {
    pub invocation: AdmittedInvocation,
    pub session: EvidenceSession,
}
pub(crate) struct CandidateEvidenceInputs {
    pub sources: BTreeMap<SourceName, SourceSpec>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub entries: BTreeMap<(PathBuf, usize), AdmittedSources>,
    pub unavailable: BTreeMap<(PathBuf, usize), BTreeMap<SourceName, UnavailableReason>>,
}

/// Admit only supplied outside facts. A missing fact has no fabricated source or value.
pub(crate) fn admit_candidate_evidence(
    input: &CandidateRunInput,
    sets: &[Arc<LoadedRuleSet>],
) -> Result<CandidateEvidenceInputs, DiagnosticList> {
    let names = sets
        .iter()
        .flat_map(|set| set.definition.inputs.keys())
        .collect::<BTreeSet<_>>();
    if input
        .evidence
        .keys()
        .chain(input.sources.iter().flat_map(|entry| entry.evidence.keys()))
        .any(|name| !names.contains(name))
    {
        return Err(DiagnosticList::rule(RuleDiagnosticCode::ExtraBinding));
    }
    let cwd = std::env::current_dir()
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let root = resolved_path(&cwd, &input.definition_root)?;
    let mut payload_schemas = input.payload_schemas.clone();
    for schema in &mut payload_schemas {
        schema.path = resolved_path(&root, &schema.path)?;
    }
    let mut prepared = CandidateEvidenceInputs {
        sources: BTreeMap::new(),
        payload_schemas,
        entries: BTreeMap::new(),
        unavailable: BTreeMap::new(),
    };
    for set in sets {
        for (position, entry) in input.sources.iter().enumerate() {
            let mut declarations = BTreeMap::new();
            let mut bindings = BTreeMap::new();
            let mut unavailable = BTreeMap::new();
            for (name, declaration) in &set.definition.inputs {
                match entry
                    .evidence
                    .get(name)
                    .or_else(|| input.evidence.get(name))
                {
                    Some(EvidenceBinding::Bound { binding }) => {
                        declarations.insert(name.clone(), declaration.clone());
                        bindings.insert(name.clone(), binding.clone());
                    }
                    Some(EvidenceBinding::Unavailable { reason }) => {
                        unavailable.insert(name.clone(), *reason);
                    }
                    None => {
                        unavailable.insert(name.clone(), UnavailableReason::NotRecorded);
                    }
                }
            }
            let admitted = admit_group(
                &declarations,
                &BindingGroup { bindings },
                &input.repositories,
                &mut prepared.sources,
            )?;
            let key = (set.path.clone(), position);
            prepared.entries.insert(key.clone(), admitted);
            prepared.unavailable.insert(key, unavailable);
        }
    }
    Ok(prepared)
}

pub(crate) fn candidate_evidence_plan(
    input: &CandidateRunInput,
    prepared: &CandidateEvidenceInputs,
) -> EvidencePlanInput {
    EvidencePlanInput {
        repositories: input.repositories.clone(),
        sources: prepared.sources.clone(),
        payload_schemas: prepared.payload_schemas.clone(),
        cursors: vec![],
        declarations: vec![],
        native_queries: vec![],
        package_bindings: vec![],
        limits: input.limits.clone(),
        discovery: input.discovery.clone(),
    }
}
