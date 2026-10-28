use crate::{case_validation::invalid, definition_snapshot::DefinitionContents, *};
use std::{collections::BTreeSet, path::Path};
use sunstack_evidence::{PayloadSchemaPath, content_digest};

pub(crate) fn schema_paths(schemas: &[FrozenPayloadSchema]) -> Vec<PayloadSchemaPath> {
    schemas
        .iter()
        .map(|schema| PayloadSchemaPath {
            name: schema.name.clone(),
            path: schema.path.clone(),
        })
        .collect()
}
pub(crate) fn capture_schemas(
    input: &FreezeInput,
    contents: &mut DefinitionContents,
) -> Result<Vec<FrozenPayloadSchema>, EvalError> {
    let root = input
        .definition_root
        .canonicalize()
        .map_err(|_| EvalError::File {
            path: input.definition_root.clone(),
        })?;
    let mut schemas = Vec::new();
    let mut names = BTreeSet::new();
    for schema in &input.payload_schemas {
        if !names.insert(&schema.name) {
            return Err(invalid("payload_schemas.name"));
        }
        if schema
            .path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(invalid("payload_schemas.path"));
        }
        let path = root
            .join(&schema.path)
            .canonicalize()
            .map_err(|_| EvalError::File {
                path: schema.path.clone(),
            })?;
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| invalid("payload_schemas.outside_root"))?
            .to_owned();
        if !crate::definition_snapshot::relative_definition_path(&relative) {
            return Err(invalid("payload_schemas.path"));
        }
        let bytes = std::fs::read(&path).map_err(|_| EvalError::File { path: path.clone() })?;
        let content_identity = ContentIdentity::new(content_digest(&bytes))?;
        if let Some(prior) = contents.get(&relative) {
            if prior != &bytes {
                return Err(EvalError::Integrity { path });
            }
        } else {
            contents.insert(relative.clone(), bytes);
        }
        schemas.push(FrozenPayloadSchema {
            name: schema.name.clone(),
            path: relative,
            content_identity,
        });
    }
    Ok(schemas)
}
pub(crate) fn load_schemas(
    home: &Path,
    schemas: &[FrozenPayloadSchema],
    contents: &mut DefinitionContents,
) -> Result<(), EvalError> {
    let mut names = BTreeSet::new();
    for schema in schemas {
        if !names.insert(&schema.name)
            || !crate::definition_snapshot::relative_definition_path(&schema.path)
        {
            return Err(invalid("manifest.payload_schemas"));
        }
        if let Some(bytes) = contents.get(&schema.path) {
            if content_digest(bytes) != schema.content_identity.as_str() {
                return Err(EvalError::Integrity {
                    path: home.join(&schema.path),
                });
            }
        } else {
            let files = crate::definition_snapshot::load_definitions(
                home,
                &[FrozenFile {
                    path: schema.path.clone(),
                    content_identity: schema.content_identity.clone(),
                }],
            )?;
            contents.extend(files);
        }
    }
    Ok(())
}
