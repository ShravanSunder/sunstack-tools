#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
#[test]
fn pointer_preserves_rfc6901_escaping_and_rejects_noncanonical_indices() {
    let payload: StructuredJson =
        serde_json::from_str(r#"{"a/b":{"~key":[null,"value"]}}"#).unwrap();
    assert_eq!(
        payload.pointer("/a~1b/~0key/1"),
        Some(&StructuredJson::String("value".into()))
    );
    assert!(payload.pointer("/a~1b/~0key/01").is_none());
    assert!(payload.pointer("/a~2b").is_none());
    assert!(payload.pointer("/a~1b/~0key/+1").is_none());
}
#[test]
fn generated_agent_job_schema_admits_reviewer_and_refuses_wrong_variants_without_values() {
    use std::collections::BTreeMap;
    let name = SchemaName::new("agent-job-packet").unwrap();
    let schema =
        serde_json::from_str(include_str!("fixtures/agent-job-packet.schema.json")).unwrap();
    let catalog = PayloadSchemaCatalog::compile(BTreeMap::from([(name.clone(), schema)])).unwrap();
    let valid = serde_json::json!({
        "role":"reviewer", "executionForm":"sidekick", "outcome":"Review an immutable artifact",
        "sources":[{"kind":"inline","content":"governing requirement","relevance":"oracle"}],
        "requiredReads":{"charter":"role-reviewer","practices":[],"principles":[]},
        "limits":{"access":"inspect"}, "stopCondition":"Return findings", "returnContract":{"kind":"owned-result","owner":"implementation-review"},
        "reviewTarget":"declared artifact at immutable revision", "procedure":"implementation-review", "authorHistory":"none"
    });
    let value: StructuredJson = serde_json::from_value(valid.clone()).unwrap();
    let admitted = catalog.validate(&name, value).unwrap();
    assert!(
        matches!(admitted.select(SourceName::new("commission").unwrap(),"/role").unwrap(),SelectionContent::PayloadValue{value:StructuredJson::String(role),..} if role=="reviewer")
    );
    for mutation in [
        serde_json::json!({"role":"wrong","sentinel":"SECRET_RECEIVED_VALUE"}),
        {
            let mut changed = valid.clone();
            changed.as_object_mut().unwrap().remove("executionForm");
            changed["execution_form"] = serde_json::json!("sidekick");
            changed
        },
        {
            let mut changed = valid.clone();
            changed["authorHistory"] = serde_json::json!("from-author");
            changed
        },
    ] {
        let error = catalog
            .validate(&name, serde_json::from_value(mutation).unwrap())
            .unwrap_err();
        assert_eq!(error.code, EvidenceCode::PayloadSchemaMismatch);
        assert!(!format!("{error:?}").contains("SECRET_RECEIVED_VALUE"));
    }
}
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
