#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::*;
#[test]
fn offline_catalog_allows_local_refs_and_refuses_network_and_file_refs() {
    use std::collections::BTreeMap;
    let schema_name = SchemaName::new("urn:packet").unwrap();
    let schemas = BTreeMap::from([
        (
            SchemaName::new("urn:labels").unwrap(),
            serde_json::json!({"$defs":{"label":{"type":"string"}}}),
        ),
        (
            schema_name.clone(),
            serde_json::json!({"$ref":"urn:labels#/$defs/label"}),
        ),
    ]);
    let catalog = PayloadSchemaCatalog::compile(schemas).unwrap();
    assert!(
        catalog
            .validate(&schema_name, StructuredJson::String("valid".into()))
            .is_ok()
    );
    assert_eq!(
        catalog
            .validate(&schema_name, StructuredJson::Boolean(false))
            .unwrap_err()
            .code,
        EvidenceCode::PayloadSchemaMismatch
    );
    for reference in [
        "https://127.0.0.1:9/no-schema",
        "file:///definitely-not-a-registered-schema.json",
    ] {
        assert!(matches!(
            PayloadSchemaCatalog::compile(BTreeMap::from([(
                schema_name.clone(),
                serde_json::json!({"$ref":reference})
            )])),
            Err(EvidenceError {
                code: EvidenceCode::PayloadSchemaInvalid,
                ..
            })
        ));
    }
}
