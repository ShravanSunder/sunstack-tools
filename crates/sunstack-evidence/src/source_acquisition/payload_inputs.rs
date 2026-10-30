use crate::*;
use std::collections::BTreeMap;

/// Compiled validators, built once from caller-supplied resources with retrieval disabled.
pub struct PayloadSchemaCatalog {
    validators: BTreeMap<SchemaName, jsonschema::Validator>,
}
impl PayloadSchemaCatalog {
    pub fn compile(schemas: BTreeMap<SchemaName, serde_json::Value>) -> EvidenceResult<Self> {
        let mut registry = jsonschema::Registry::new();
        for (name, schema) in &schemas {
            registry = registry
                .add(name.as_str(), schema.clone())
                .map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
        }
        let registry = registry
            .prepare()
            .map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
        let mut validators = BTreeMap::new();
        for (name, schema) in schemas {
            let validator = jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .with_registry(&registry)
                .offline()
                .build(&schema)
                .map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
            validators.insert(name, validator);
        }
        Ok(Self { validators })
    }
    pub fn validate(
        &self,
        schema: &SchemaName,
        value: StructuredJson,
    ) -> EvidenceResult<AdmittedPayload> {
        let started_at = chrono::Utc::now();
        let validator = self
            .validators
            .get(schema)
            .ok_or(EvidenceCode::UnknownPayloadSchema)?;
        let raw = serde_json::to_value(&value).map_err(|_| EvidenceCode::PayloadSchemaMismatch)?;
        if let Some(error) = validator.iter_errors(&raw).next() {
            // Arbitrary payload keys and the validator's Display text are never diagnostic copy.
            let mut field_path = vec!["value".to_string()];
            for segment in error
                .instance_path()
                .to_string()
                .split('/')
                .skip(1)
                .take(31)
            {
                field_path.push(
                    if segment.len() <= 9 && segment.bytes().all(|byte| byte.is_ascii_digit()) {
                        segment.to_string()
                    } else {
                        "field".into()
                    },
                );
            }
            return Err(EvidenceError {
                code: EvidenceCode::PayloadSchemaMismatch,
                field_path,
            });
        }
        Ok(AdmittedPayload {
            interval: CaptureInterval {
                started_at,
                completed_at: chrono::Utc::now(),
            },
            schema: schema.clone(),
            value,
        })
    }
}
#[derive(Clone, Debug)]
pub struct AdmittedPayload {
    pub interval: CaptureInterval,
    pub schema: SchemaName,
    pub value: StructuredJson,
}
impl AdmittedPayload {
    pub fn select(&self, input: SourceName, pointer: &str) -> EvidenceResult<SelectionContent> {
        let value = self
            .value
            .pointer(pointer)
            .ok_or(EvidenceCode::InvalidPointer)?
            .clone();
        Ok(SelectionContent::PayloadValue {
            input,
            pointer: pointer.into(),
            value,
        })
    }
}
