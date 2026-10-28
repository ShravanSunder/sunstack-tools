use crate::{
    definition_loading::DefinitionLoader, input_binding::resolved_path,
    invocation_admission::LoadedRuleSet, *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use sunstack_evidence::{
    DeclaredSourceKind, EvidenceDeclaration, PayloadSchemaCatalog, SourceName,
};
use tokio_util::sync::CancellationToken;

pub(crate) fn load_checked_definitions(
    input: &CheckDefinitionsInput,
    cancellation: CancellationToken,
) -> Result<Vec<Arc<LoadedRuleSet>>, DiagnosticList> {
    let cwd = std::env::current_dir()
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let root = resolved_path(&cwd, &input.definition_root)?;
    let loader = DefinitionLoader::new(input.limits.max_file_bytes.get(), cancellation);
    load_declared_definitions(&input.definitions, &input.payload_schemas, &root, loader)
}
pub(crate) fn load_declared_definitions(
    definitions: &CandidateDefinitions,
    payload_schemas: &[sunstack_evidence::PayloadSchemaPath],
    root: &std::path::Path,
    mut loader: DefinitionLoader,
) -> Result<Vec<Arc<LoadedRuleSet>>, DiagnosticList> {
    let mut sets = BTreeMap::new();
    let router = match definitions {
        CandidateDefinitions::RuleSets { rule_sets } => {
            if rule_sets.is_empty() {
                return Err(DiagnosticList::rule(
                    RuleDiagnosticCode::InvalidConfiguration,
                ));
            }
            for path in rule_sets {
                let path = loader.resolve(root, path)?;
                if sets.contains_key(&path) {
                    return Err(DiagnosticList::rule(RuleDiagnosticCode::DuplicateRuleSet));
                }
                sets.insert(path.clone(), loader.load_set(path)?);
            }
            None
        }
        CandidateDefinitions::Router { path } => {
            let path = loader.resolve(root, path)?;
            let definition = decode_router(&loader.read(&path)?, &path)?;
            let directory = path
                .parent()
                .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
            for targets in definition.routes.values() {
                let mut seen = BTreeSet::new();
                for target in targets {
                    let path = loader.resolve(directory, &target.rule_set)?;
                    if !seen.insert(path.clone()) {
                        return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidRoute));
                    }
                    let set = loader.load_set(path.clone())?;
                    crate::definition_routing::validate_route_mapping(
                        target,
                        &set.definition.inputs,
                        &definition.inputs,
                    )?;
                    sets.insert(path, set);
                }
            }
            Some(definition)
        }
    };
    let mut schemas = BTreeMap::new();
    for schema in payload_schemas {
        let path = loader.resolve(root, &schema.path)?;
        let value = serde_json::from_slice(&loader.read(&path)?).map_err(|_| {
            DiagnosticList::from(sunstack_evidence::DiagnosticList::one(
                sunstack_evidence::EvidenceCode::PayloadSchemaInvalid,
            ))
        })?;
        if schemas.insert(schema.name.clone(), value).is_some() {
            return Err(DiagnosticList::from(
                sunstack_evidence::DiagnosticList::one(
                    sunstack_evidence::EvidenceCode::PayloadSchemaInvalid,
                ),
            ));
        }
    }
    let schema_names = schemas.keys().cloned().collect::<BTreeSet<_>>();
    PayloadSchemaCatalog::compile(schemas)
        .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
    if let Some(router) = &router {
        validate_input_schemas(&router.inputs, &schema_names)?;
        validate_declarations(&router.evidence, &router.inputs, &schema_names)?;
    }
    for set in sets.values() {
        validate_input_schemas(&set.definition.inputs, &schema_names)?;
        validate_entries(
            &set.definition.applicability,
            &set.definition.inputs,
            &schema_names,
        )?;
        for located in &set.rules {
            let rule = &located.definition;
            let admitted: Result<(), DiagnosticList> = (|| {
                validate_declarations(&rule.evidence, &set.definition.inputs, &schema_names)?;
                validate_entries(&rule.applicability, &set.definition.inputs, &schema_names)?;
                if let Some(narrowing) = &rule.narrowing {
                    validate_declarations(
                        &narrowing.evidence,
                        &set.definition.inputs,
                        &schema_names,
                    )?;
                }
                Ok(())
            })();
            admitted.map_err(|mut diagnostics| {
                if let Some(provenance) = &located.provenance {
                    crate::definition_admission::locate_at(
                        &mut diagnostics,
                        &provenance.rules,
                        provenance.entry_location.line,
                        provenance.entry_location.column,
                    );
                } else {
                    crate::definition_admission::locate(
                        &mut diagnostics,
                        &located.source,
                        located.line,
                    );
                }
                diagnostics
            })?;
        }
    }
    Ok(sets.into_values().collect())
}
fn validate_input_schemas(
    inputs: &BTreeMap<SourceName, InputDeclaration>,
    schemas: &BTreeSet<sunstack_evidence::SchemaName>,
) -> Result<(), DiagnosticList> {
    for (name, input) in inputs {
        if let InputDeclaration::Payload { schema } = input
            && !schemas.contains(schema)
        {
            let mut diagnostic = sunstack_evidence::Diagnostic::new(
                sunstack_evidence::EvidenceCode::UnknownPayloadSchema,
            );
            diagnostic.location = sunstack_evidence::DiagnosticLocation::Field {
                field_path: vec!["inputs".into(), name.as_str().into(), "schema".into()],
            };
            return Err(sunstack_evidence::DiagnosticList {
                items: vec![diagnostic],
                omitted_count: 0,
            }
            .into());
        }
    }
    Ok(())
}
fn validate_entries(
    entries: &[ApplicabilityEntry],
    inputs: &BTreeMap<SourceName, InputDeclaration>,
    schemas: &BTreeSet<sunstack_evidence::SchemaName>,
) -> Result<(), DiagnosticList> {
    for entry in entries {
        if let ApplicabilityEntry::Decision { evidence, .. } = entry {
            validate_declarations(evidence, inputs, schemas)?;
        }
    }
    Ok(())
}
fn validate_declarations(
    declarations: &[EvidenceDeclaration],
    inputs: &BTreeMap<SourceName, InputDeclaration>,
    schemas: &BTreeSet<sunstack_evidence::SchemaName>,
) -> Result<(), DiagnosticList> {
    for declaration in declarations {
        let input = inputs
            .get(&declaration.input)
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UndeclaredInput))?;
        let kind = match input {
            InputDeclaration::Document {} => DeclaredSourceKind::Document,
            InputDeclaration::Conversation {} => DeclaredSourceKind::Conversation,
            InputDeclaration::ChangeSet {} => DeclaredSourceKind::ChangeSet,
            InputDeclaration::Payload { schema } => {
                if !schemas.contains(schema) {
                    return Err(DiagnosticList::from(
                        sunstack_evidence::DiagnosticList::one(
                            sunstack_evidence::EvidenceCode::UnknownPayloadSchema,
                        ),
                    ));
                }
                DeclaredSourceKind::Payload
            }
        };
        sunstack_evidence::admit_declared_selection(&declaration.select, kind)
            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
    }
    Ok(())
}
