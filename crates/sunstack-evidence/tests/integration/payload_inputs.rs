#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
#[test]
fn generated_agent_job_schema_admits_reviewer_and_refuses_wrong_variants_without_values() {
    use std::collections::BTreeMap;
    let name = SchemaName::new("agent-job-packet").unwrap();
    let schema =
        serde_json::from_str(include_str!("../fixtures/agent-job-packet.schema.json")).unwrap();
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
